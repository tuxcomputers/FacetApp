# ksni 0.3.6, vendored

Copied from crates.io ksni 0.3.6 (Unlicense) and used through `[patch.crates-io]` in the workspace `Cargo.toml`.

The one change adds the Ayatana label, which XApp and Ayatana tray hosts draw beside the icon:

- `src/dbus_interface.rs`: the `XAyatanaLabel` and `XAyatanaLabelGuide` properties, and the `XAyatanaNewLabel`
  signal. The label is the tray's `title()`.
- `src/service.rs`: `XAyatanaNewLabel` sent alongside `NewTitle` whenever the title changes.

Each change is marked `Facet:` in a comment. Drop the vendored copy once upstream publishes the label.
