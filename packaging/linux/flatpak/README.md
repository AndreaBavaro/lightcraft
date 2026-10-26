# From-source Flatpak packaging

The manifests in this directory package LightCraft. `cargo-sources.json` is the
app-specific offline dependency inventory generated from `Cargo.lock`.

Generator implementation and regression tests live in
[craft-common-actions](https://github.com/storytold/craft-common-actions/tree/main/scripts/flatpak).
Use a checkout of its `main` branch alongside this app. From the LightCraft root:

```sh
git clone --branch main https://github.com/storytold/craft-common-actions.git ../craft-common-actions
python3 ../craft-common-actions/scripts/flatpak/generate-cargo-sources.py Cargo.lock -o packaging/linux/flatpak/cargo-sources.json
flatpak-builder --user --install --force-clean build-flatpak packaging/linux/flatpak/ai.storyteller.lightcraft.yml
flatpak run ai.storyteller.lightcraft
```

Regenerate after lockfile changes. The shared tool verifies its upstream generator,
keeps multiple git revisions and unused Cargo patches distinct, and replaces the
inventory only after successful generation. See its README for Python dependencies,
cache placement, cancellation behavior and owner-only output permissions.

The current inventory passed checksum verification and locked offline Cargo metadata
resolution. A full Flatpak SDK 25.08 build remains required before release.
The shared generator must land on common-actions main before the command above is available.

## Inventory validation record


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

