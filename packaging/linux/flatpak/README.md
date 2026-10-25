# From-source Flatpak packaging

`ai.storyteller.lightcraft.yml` builds LightCraft and `lightcraft-cli` from this
repository inside the Flatpak SDK. Flatpak builders have no network access, so
`cargo-sources.json` lists every crate and git source with a fixed checksum or
commit. Regenerate it after changing `Cargo.lock`:

```sh
./packaging/linux/flatpak/generate-cargo-sources.py
flatpak-builder --user --install --force-clean build-flatpak packaging/linux/flatpak/ai.storyteller.lightcraft.yml
flatpak run ai.storyteller.lightcraft
```

## Regeneration wrapper

`generate-cargo-sources.py` uses the upstream
[`flatpak-cargo-generator.py`](https://github.com/flatpak/flatpak-builder-tools/blob/9d86d336a39b9af38b86f114b6ce5820fe260f91/cargo/flatpak-cargo-generator.py)
pinned to commit `9d86d336a39b9af38b86f114b6ce5820fe260f91` with SHA-256
`0a2db6be87d75910facef28ab46d4d6460802e8419ab850d0caa6a364d26b380`. It downloads
the script, verifies that hash, applies the two reviewed patches below, and
invokes it with the current Python interpreter. The upstream script is not
vendored; it is MIT-licensed
([upstream licence](https://github.com/flatpak/flatpak-builder-tools/blob/9d86d336a39b9af38b86f114b6ce5820fe260f91/LICENSE)).
The patched script's hash is also checked before execution.

Requirements: Python 3.9+ with `aiohttp` and `tomlkit` (`PyYAML` is optional and
only needed for upstream's YAML output), plus `git` and network access to fetch
the generator and git dependencies. Crate archive URLs and checksums are read
from Cargo.lock; Flatpak downloads those archives when materializing the sources. The generator clones git
repositories into `$XDG_CACHE_HOME/lightcraft-flatpak-cargo` by default, or the
directory passed with `--cache-dir`; the wrapper refuses a cache inside the
checkout or `CARGO_TARGET_DIR`. `cargo-sources.json` is written to a temporary
file and atomically renamed only after the generator succeeds, so a failed run
leaves the previous inventory untouched.

## Why two patches

1. **Keep pinned revisions distinct.** Upstream keys `[source]` replacement
   entries by the canonical repository URL and then adds `rev`/`tag`/`branch`.
   Two different revisions of the same repository therefore overwrite each
   other. The patch keys the entry by the full source URL without its `#`
   fragment, so `https://github.com/storytold/craft-libs?rev=...` remains a
   separate source for each craft-libs revision.
2. **Include `[patch.unused]`.** `Cargo.lock` records patched packages that are
   not in the resolved graph under `patch.unused`. Upstream iterates only
   `package`, which omits crates such as `egui_demo_lib`; `--locked` offline
   dependency resolution then fails because Cargo still loads those patched manifests. The
   patch also walks `patch.unused` entries.

No third patch is needed: upstream's `git_repo_name()` already appends the
first seven characters of the commit, so the two craft-libs checkouts get
different destinations.

## Offline validation performed

The committed inventory was generated from the current `Cargo.lock` and then
validated without contacting the network: every git/archive/shell/inline entry
was materialized as Flatpak would, crate archives were checked against their
recorded SHA-256 values, and the generated Cargo config was pointed at the
materialized vendor directory. With an isolated `CARGO_HOME` and
`CARGO_NET_OFFLINE=true`, `cargo --config <generated-config> metadata --offline
--locked --format-version 1` exited 0. The two craft-libs revisions
(`82a24b006eefa036b43c09a67fb9a3fbd17ffe63` and
`25da1314d443171b4adeaad2cd9454b273f35c6f`) are present as separate git sources
and `[source]` entries. The checked-in `cargo-sources.json` has SHA-256
`c7f515d1178bb79b2cabdfd419ad375a04ff5d84ba9d08b1b34970a460cd0f2c`.

This validates the source inventory and offline metadata resolution only. A
full `flatpak-builder` build against the `org.freedesktop.Sdk` 25.08 runtime and
Rust extension is still required before release; it was not performed here.

Run the wrapper failure-path regression tests with:

```sh
python3 -B packaging/linux/flatpak/test_generate_cargo_sources.py
```
