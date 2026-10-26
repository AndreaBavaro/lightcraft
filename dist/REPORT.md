# LightCraft v0.14.0 fork: TLS crate bumps (tls-bumps branch)

Date: 2026-10-26. Fork: https://github.com/AndreaBavaro/lightcraft, branch `tls-bumps`, based on upstream tag v0.14.0 (03c88c166c054c009db2a47187d59991f6232b85). Not pushed, no PR.

## What changed (Cargo.lock only, no manifest edits)

| Crate | Before | After | Advisories closed |
|---|---|---|---|
| rustls | 0.23.45 | 0.23.46 | RUSTSEC-2026-0346, 0347, 0348, 0349 |
| rustls-webpki (0.103 line) | 0.103.15 | 0.103.16 | RUSTSEC-2026-0382 |
| webpki-roots | 1.0.9 | 1.0.10 | RUSTSEC-2026-0381 |

Commands (run one at a time; `cargo update` accepts one `--precise` per call):

```
cargo update -p rustls@0.23.45 --precise 0.23.46
cargo update -p rustls-webpki@0.103.15 --precise 0.103.16
cargo update -p webpki-roots@1.0.9 --precise 1.0.10
```

The first `cargo update` also flipped tempfile's `getrandom` entry from 0.4.3 to 0.3.4 (both remain in the lockfile). That hunk was reverted by hand to keep the change to the three TLS crates; `cargo metadata --locked` passes on the result. Lockfile diff against v0.14.0:

```diff
diff --git a/Cargo.lock b/Cargo.lock
index 10bb99e6..f96d3340 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5263,13 +5263,13 @@ dependencies = [
 
 [[package]]
 name = "rustls"
-version = "0.23.45"
+version = "0.23.46"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "0d41d731c7d2f962d1ccc364cec258de3c0e93b38c2fb3ba97ac74513048d634"
+checksum = "48e13bd8c0e9365c43cfa5c9e8f9ad49d3c8444926c9aac819e0e4dc503c8fdf"
 dependencies = [
  "once_cell",
  "rustls-pki-types",
- "rustls-webpki 0.103.15",
+ "rustls-webpki 0.103.16",
  "subtle",
  "zeroize",
 ]
@@ -5326,9 +5326,9 @@ dependencies = [
 
 [[package]]
 name = "rustls-webpki"
-version = "0.103.15"
+version = "0.103.16"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "f3c3cf1d8b1e7d4927e2d154c3fcb02979afb9939629c62cd9048d4f07b60ac2"
+checksum = "4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281"
 dependencies = [
  "ring",
  "rustls-pki-types",
@@ -6377,9 +6377,9 @@ dependencies = [
 
 [[package]]
 name = "webpki-roots"
-version = "1.0.9"
+version = "1.0.10"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "7dcd9d09a39985f5344844e66b0c530a33843579125f23e21e9f0f220850f22a"
+checksum = "1c1c8e20c8a38fc7d65abfe74eb7280e4aa667e8eec8bc84996d2874810748d9"
 dependencies = [
  "rustls-pki-types",
 ]
```

## cargo audit (cargo-audit 0.22.2, advisory DB as of 2026-10-26)

| | Before | After |
|---|---|---|
| Vulnerabilities | 16 | 10 |
| Warnings (unmaintained 5, unsound 1, yanked 1) | 7 | 7 |

Gone after the bump: RUSTSEC-2026-0381 (webpki-roots), RUSTSEC-2026-0382 on rustls-webpki 0.103.x, RUSTSEC-2026-0346/0347/0348/0349 (rustls).

Still open, all outside this brief's scope:

- RUSTSEC-2026-0382 and 0049/0098/0099/0104 on rustls-webpki **0.102.8**. This second copy comes from rustls-rustcrypto 0.0.2-alpha, the TLS provider `lightcraft-fetch` uses (crates/fetch/src/http.rs). The provider uses it only for `webpki::alg_id` constants in its signature-verification shims; certificate path validation runs in rustls's own verifier on the 0.103.16 copy. Confidence high from reading the source; not formally proven. Fixing it means moving the provider to a newer rustls-rustcrypto or to ring/aws-lc, which the brief marks as upstream's design call.
- RUSTSEC-2026-0387 crypto-bigint 0.5.5 and RUSTSEC-2023-0071 rsa 0.9.10 (also via the RustCrypto provider).
- RUSTSEC-2026-0367/0368/0369 zip 7.2.0.
- Warnings unchanged: paste, ttf-parser, zbus_names, zvariant, zvariant_derive unmaintained; tokio unsound; clipboard-win yanked.

Full JSON: `dist/audit-before.json`, `dist/audit-after.json`.

## Build

Done as `packaging/macos/package.sh` does for a release, with ad-hoc signing (no MACOS_SIGN_IDENTITY) and aarch64 only (only that target is installed; a universal build needs `rustup target add x86_64-apple-darwin`). Toolchain: stable 1.98.1 (the shared CI gate installs `stable`; `rust-version = "1.95"` is the minimum, not a pin). macOS 26.6.2, Xcode command line tools.

```
CARGO_BUILD_JOBS=4 bash packaging/macos/package.sh --arch aarch64
```

which runs `cargo build --release --locked -p lightcraft -p lightcraft-cli --features lightcraft/heif,lightcraft-cli/heif --target aarch64-apple-darwin` with craft-fonts 3d9ab3a1 embedded (cloned to ~/Projects/craft-fonts by the script), then assembles, signs and verifies the bundle. Release build time 7m 18s. Log: `dist/build.log`.

Outputs:

- `dist/LightCraft.app` (113 MB, ad-hoc signed, `codesign --verify --strict --deep` passes; right-click Open on first launch)
- `dist/release/lightcraft-0.14.0-macos-aarch64.dmg` (54 MB)
- `dist/release/lightcraft-cli-0.14.0-macos-aarch64.zip` (50 MB)

Info.plist carries LightCraftBuildCommit 596f66d749eb1747e8981e12f0f08018936a68b1 (the lockfile commit). `strings` on the executable shows rustls-0.23.46 and rustls-webpki-0.103.16 and no 0.23.45 / 0.103.15.

SHA256:

| File | SHA256 |
|---|---|
| dist/LightCraft.app/Contents/MacOS/LightCraft (the executable) | 8d3d37c315fcdfb16ff32ff505f7b7ff03a0632310b2861b83bebf2fa43eb5d2 |
| dist/LightCraft.app, all files (sha256 of the sorted per-file sha256 list) | 83f2052236f4032fa163481f7c6ecd368746facd2cd087d53d7c3b0f0a9d399d |
| dist/release/lightcraft-0.14.0-macos-aarch64.dmg | f0b1ebbb3313b56d8ae818c2ee552b8d978ce0c321d5cf0fa3223d747c63fd47 |

Reproduce the bundle hash: `cd dist/LightCraft.app && find . -type f | LC_ALL=C sort | xargs shasum -a 256 | shasum -a 256`.

## Tests

The CI gate runs `cargo xtask ci`, whose test step is the two commands below (xtask sets CARGO_BUILD_JOBS, RUST_TEST_THREADS and line-table debuginfo; the same values were set by hand). Run on the fork checkout:

```
CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo test --workspace --lib --bins --tests
CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo test --workspace --doc
```

Result: both exit 0. 83 test binaries, 4879 passed, 0 failed, 41 ignored.

Not run from `xtask ci`: fmt, clippy, the heif clippy step and parity (no source changed; the heif test binaries are covered by the workspace run only without the `heif` feature). The ui-egui tests ran in a headless session (no GPU adapter tests requested; `--hardware` not passed).

Per binary (passed / failed / ignored); every passed test by name is in `dist/tests-passed.txt`, ignored ones in `dist/tests-ignored.txt`, full output in `dist/test.log`:

| Binary | Passed | Failed | Ignored |
|---|---|---|---|
| lightcraft unittests | 83 | 0 | 0 |
| lightcraft_catalog unittests | 249 | 0 | 2 |
| lightcraft_cli unittests | 0 | 0 | 0 |
| t tests/it/main.rs | 26 | 0 | 0 |
| lightcraft_codecs unittests | 79 | 0 | 0 |
| t tests/it/main.rs | 55 | 0 | 1 |
| xl_tail_heap tests/jxl_tail_heap.rs | 1 | 0 | 0 |
| lightcraft_color unittests | 22 | 0 | 0 |
| lightcraft_denoise unittests | 26 | 0 | 2 |
| t tests/it/main.rs | 0 | 0 | 1 |
| lightcraft_denoise_core unittests | 40 | 0 | 1 |
| lightcraft_develop unittests | 30 | 0 | 0 |
| lightcraft_engine unittests | 1327 | 0 | 6 |
| t tests/it/main.rs | 0 | 0 | 3 |
| lightcraft_faces unittests | 76 | 0 | 3 |
| lightcraft_fetch unittests | 25 | 0 | 0 |
| lightcraft_geocode unittests | 8 | 0 | 0 |
| lightcraft_geom unittests | 44 | 0 | 0 |
| lightcraft_gpu unittests | 34 | 0 | 7 |
| t tests/it/main.rs | 19 | 0 | 1 |
| emory tests/memory.rs | 1 | 0 | 0 |
| hutdown tests/shutdown.rs | 1 | 0 | 0 |
| hutdown_first tests/shutdown_first.rs | 1 | 0 | 0 |
| lightcraft_heif unittests | 25 | 0 | 0 |
| lightcraft_inpaint unittests | 5 | 0 | 0 |
| lightcraft_lensdb unittests | 20 | 0 | 0 |
| lightcraft_mcp unittests | 51 | 0 | 1 |
| t tests/it/main.rs | 7 | 0 | 0 |
| lightcraft_merge unittests | 25 | 0 | 0 |
| t tests/it/main.rs | 20 | 0 | 0 |
| lightcraft_meta unittests | 116 | 0 | 0 |
| t tests/it/main.rs | 4 | 0 | 0 |
| lightcraft_pipeline unittests | 205 | 0 | 2 |
| t tests/it/main.rs | 2 | 0 | 1 |
| lightcraft_preview unittests | 26 | 0 | 0 |
| lightcraft_raster unittests | 27 | 0 | 0 |
| lightcraft_raw unittests | 461 | 0 | 3 |
| t tests/it/main.rs | 134 | 0 | 0 |
| r2_heap tests/sr2_heap.rs | 1 | 0 | 0 |
| lightcraft_scenes unittests | 5 | 0 | 0 |
| lightcraft_segment unittests | 10 | 0 | 0 |
| lightcraft_sam3_worker unittests | 0 | 0 | 0 |
| t tests/it/main.rs | 3 | 0 | 0 |
| lightcraft_slideshow unittests | 13 | 0 | 0 |
| lightcraft_sysmem unittests | 3 | 0 | 0 |
| lightcraft_testkit unittests | 4 | 0 | 0 |
| lightcraft_tiff unittests | 12 | 0 | 0 |
| t tests/it/main.rs | 16 | 0 | 0 |
| lightcraft_ui_egui unittests | 1447 | 0 | 4 |
| t tests/it/main.rs | 8 | 0 | 0 |
| hutdown tests/shutdown.rs | 1 | 0 | 0 |
| lightcraft_web unittests | 30 | 0 | 0 |
| task unittests | 50 | 0 | 0 |
| doctests lightcraft_catalog | 0 | 0 | 0 |
| doctests lightcraft_codecs | 0 | 0 | 0 |
| doctests lightcraft_color | 0 | 0 | 0 |
| doctests lightcraft_denoise | 0 | 0 | 0 |
| doctests lightcraft_denoise_core | 0 | 0 | 0 |
| doctests lightcraft_develop | 0 | 0 | 0 |
| doctests lightcraft_engine | 0 | 0 | 0 |
| doctests lightcraft_faces | 0 | 0 | 0 |
| doctests lightcraft_fetch | 0 | 0 | 0 |
| doctests lightcraft_geocode | 0 | 0 | 0 |
| doctests lightcraft_geom | 0 | 0 | 0 |
| doctests lightcraft_gpu | 0 | 0 | 0 |
| doctests lightcraft_heif | 0 | 0 | 0 |
| doctests lightcraft_inpaint | 0 | 0 | 0 |
| doctests lightcraft_lensdb | 0 | 0 | 0 |
| doctests lightcraft_mcp | 0 | 0 | 0 |
| doctests lightcraft_merge | 0 | 0 | 0 |
| doctests lightcraft_meta | 0 | 0 | 0 |
| doctests lightcraft_pipeline | 0 | 0 | 0 |
| doctests lightcraft_preview | 0 | 0 | 0 |
| doctests lightcraft_raster | 0 | 0 | 0 |
| doctests lightcraft_raw | 0 | 0 | 0 |
| doctests lightcraft_scenes | 0 | 0 | 0 |
| doctests lightcraft_segment | 0 | 0 | 0 |
| doctests lightcraft_slideshow | 0 | 0 | 0 |
| doctests lightcraft_sysmem | 0 | 0 | 0 |
| doctests lightcraft_testkit | 1 | 0 | 0 |
| doctests lightcraft_tiff | 0 | 0 | 0 |
| doctests lightcraft_ui_egui | 0 | 0 | 3 |
| doctests lightcraft_web | 0 | 0 | 0 |

## Open items

1. Push `tls-bumps` to the fork? Not done; needs Andrea's go-ahead. No upstream PR either.
2. rustls-webpki 0.102.8 (RUSTSEC-2026-0382 and four older) stays via rustls-rustcrypto; a provider change is upstream's call.
3. The build is aarch64 only and ad-hoc signed; it will not pass Gatekeeper on another Mac.
4. The release build ran at load 6 with another session's Playwright run active (gate refused on the Chrome count only); the test run was sequential after it.
