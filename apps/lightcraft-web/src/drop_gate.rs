//! When the browser host may consume queued drops. Kept out of the wasm-only `web` module so the
//! rule that keeps stored drops out of a library behind a foreground window is testable natively.

use lightcraft_ui_egui::LightcraftApp;

/// The browser host's two drop queues: file handles not yet read, and originals whose asynchronous
/// storage has finished and that wait to be imported. Taking either only through this trait keeps
/// the gate and the drain together.
pub trait QueuedDrops {
    type Dropped;
    type Pending;
    /// Whether a prompt or the blocking library window owns input right now.
    fn drops_wait(&self) -> bool;
    fn take_dropped(&mut self) -> Vec<Self::Dropped>;
    fn take_pending(&mut self) -> Vec<Self::Pending>;
}

/// A prompt or the blocking library window (over an already open library, or its own New Library
/// name prompt or asynchronous folder picker) owns input: leaves the dropped-file queue and the
/// originals still being stored for a later UI pass, instead of importing them behind the window.
/// With no library open and no dialog in front, the existing drop path is unchanged.
pub fn drops_wait(app: &LightcraftApp) -> bool {
    lightcraft_ui_egui::panels::notices::blocks_input(app) || lightcraft_ui_egui::panels::library_problem::blocks_file_drops(app)
}

/// One UI pass's queues, drained by [`drain_ready`].
pub struct Drained<D, P> {
    pub dropped: Vec<D>,
    /// `None` when new file handles were refused behind a dialog: the originals already stored
    /// stay queued for a later pass (as before), rather than importing behind that dialog.
    pub pending: Option<Vec<P>>,
}

/// The actual queue drain `WebApp` runs each UI pass: while [`drops_wait`] holds, neither queue is
/// touched. Once ready, the dropped-file queue is taken first; if those handles are refused behind
/// a dialog (`dialog_open`), the stored originals are left queued as well.
pub fn drain_ready<Q: QueuedDrops>(queues: &mut Q, dialog_open: bool) -> Option<Drained<Q::Dropped, Q::Pending>> {
    if queues.drops_wait() {
        return None;
    }
    let dropped = queues.take_dropped();
    let pending = if !dropped.is_empty() && dialog_open { None } else { Some(queues.take_pending()) };
    Some(Drained { dropped, pending })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{QueuedDrops, drain_ready, drops_wait};
    use lightcraft_engine::Session;
    use lightcraft_ui_egui::panels::library_problem::LibraryProblem;
    use lightcraft_ui_egui::state::Dialog;
    use lightcraft_ui_egui::{LightcraftApp, Services};

    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    type Asked = std::sync::Arc<std::sync::Mutex<Vec<std::sync::mpsc::Sender<Vec<String>>>>>;

    fn app_with_library(tag: &str) -> Result<(Scratch, LightcraftApp), String> {
        let dir = std::env::temp_dir().join(format!("lc-web-drop-gate-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        let scratch = Scratch(dir);
        let mut session = Session::new().with_fs();
        session.open_library(&scratch.0, false).map_err(|error| error.to_string())?;
        Ok((scratch, LightcraftApp::new(session, Services::default())))
    }

    /// Stands in for `WebApp`: the same shared orchestration over two real queues, so a test sees
    /// that a held window leaves both queues untouched, that the ready path drains both, and that
    /// a refused new drop keeps the already stored originals queued.
    struct FakeQueues {
        app: LightcraftApp,
        dropped: Vec<String>,
        pending: Vec<String>,
    }

    impl FakeQueues {
        fn new(app: LightcraftApp) -> Self {
            Self { app, dropped: vec!["dropped-photo".into()], pending: vec!["web/hash/photo.png".into()] }
        }
    }

    impl QueuedDrops for FakeQueues {
        type Dropped = String;
        type Pending = String;

        fn drops_wait(&self) -> bool {
            drops_wait(&self.app)
        }

        fn take_dropped(&mut self) -> Vec<Self::Dropped> {
            std::mem::take(&mut self.dropped)
        }

        fn take_pending(&mut self) -> Vec<Self::Pending> {
            std::mem::take(&mut self.pending)
        }
    }

    #[test]
    fn stored_drops_wait_through_the_choosers_new_library_prompt_and_drain_after_it() {
        let (_dir, mut app) = app_with_library("chooser").expect("create chooser test library");
        let mut problem = LibraryProblem::choose_later(&app.ui.settings);
        if let Some(c) = problem.chooser.as_mut() {
            c.over = Some(app.session.library_generation());
        }
        app.library_problem = Some(problem);
        let mut queues = FakeQueues::new(app);

        // The list's own New Library name prompt hides the list but still owns the library.
        queues.app.ui.dialog = Some(Dialog::TextPrompt {
            title: "New Library".into(),
            hint: "Library name".into(),
            value: String::new(),
            command: "app.newLibrary".into(),
            params: serde_json::json!({}),
            key: "name".into(),
        });
        assert!(drain_ready(&mut queues, false).is_none(), "the name prompt holds both queues");
        assert_eq!((queues.dropped.len(), queues.pending.len()), (1, 1), "the queued drop survived the prompt");

        // Cancelling the prompt brings the list back; it still owns the drop until it goes.
        queues.app.ui.dialog = None;
        assert!(drain_ready(&mut queues, false).is_none(), "the list is back and still holds the queue");
        queues.app.library_problem = None;
        let got = drain_ready(&mut queues, false).expect("the queues should be ready");
        assert_eq!(got.dropped, vec!["dropped-photo".to_string()]);
        assert_eq!(got.pending, Some(vec!["web/hash/photo.png".to_string()]));
        assert!(queues.dropped.is_empty() && queues.pending.is_empty(), "both queues drained once the window was gone");
    }

    #[test]
    fn a_refused_new_drop_behind_a_dialog_keeps_stored_originals_queued() {
        let (_dir, app) = app_with_library("refused").expect("create refused-drop test library");
        // not held by the gate: the dialog alone refuses the newly dropped handles
        let mut queues = FakeQueues::new(app);
        let got = drain_ready(&mut queues, true).expect("the queues should be ready");
        assert_eq!(got.dropped, vec!["dropped-photo".to_string()]);
        assert_eq!(got.pending, None, "the refused drop must not drain originals behind the dialog");
        assert_eq!(queues.pending.len(), 1, "the earlier originals stayed queued");

        // Nothing new was dropped: originals read from an earlier drop still finish, as before.
        let got = drain_ready(&mut queues, true).expect("the queues should be ready");
        assert!(got.dropped.is_empty());
        assert_eq!(got.pending, Some(vec!["web/hash/photo.png".to_string()]));
        assert!(queues.pending.is_empty());
    }

    #[test]
    fn a_no_library_library_picker_holds_both_queues() {
        let mut app = LightcraftApp::new(Session::new().with_fs(), Services::default());
        let asked: Asked = Default::default();
        app.services.picker = Some(Box::new(move |_| {
            let (tx, rx) = std::sync::mpsc::channel();
            asked.lock().unwrap().push(tx);
            Ok(rx)
        }));
        // New Library's name prompt was confirmed; the asynchronous place picker is still open.
        assert_eq!(app.run("app.newLibrary", serde_json::json!({"name": "Queued"})).unwrap(), serde_json::Value::Null);
        assert!(app.session.library.is_none());
        app.library_problem = Some(LibraryProblem::choose_later(&app.ui.settings));
        let mut queues = FakeQueues::new(app);
        assert!(drain_ready(&mut queues, false).is_none(), "a library picker that will open one holds both queues");
        assert_eq!((queues.dropped.len(), queues.pending.len()), (1, 1), "nothing was drained while the picker is up");
    }

    #[test]
    fn a_no_library_window_lets_the_existing_drop_path_consume_the_queues() {
        let mut app = LightcraftApp::new(Session::with_demo(), Services::default());
        app.library_problem = Some(LibraryProblem::new("/no/such/library", "missing"));
        let mut queues = FakeQueues::new(app);
        assert!(!drops_wait(&queues.app), "with no library open the existing drop path is unchanged");
        let got = drain_ready(&mut queues, false).expect("the queues should be ready");
        assert_eq!(got.dropped, vec!["dropped-photo".to_string()]);
        assert_eq!(got.pending, Some(vec!["web/hash/photo.png".to_string()]));
        assert!(queues.dropped.is_empty() && queues.pending.is_empty(), "the host consumed both queues");

        queues.app.notices.push("The settings file could not be read and was reset.".into());
        queues.dropped.push("late-drop".into());
        queues.pending.push("web/hash/late.png".into());
        assert!(drain_ready(&mut queues, false).is_none(), "a prompt still owns input");
        assert_eq!((queues.dropped.len(), queues.pending.len()), (1, 1), "the prompt left both queues alone");
    }
}
