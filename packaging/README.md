# Packaging

Builds the two downloads the website offers. Output goes to `dist/`, which is gitignored.

| Script | Runs on | Makes |
|---|---|---|
| `package-mac.sh` | a Mac | `dist/Facet-<version>-macos.dmg`, holding a universal `Facet.app` |
| `package-linux.sh` | Linux | `dist/Facet-<version>-linux-<arch>.tar.gz` |

Both write a `.sha256` beside the file, take the version from `[workspace.package]` in `Cargo.toml`, and
bundle the Google OAuth client through `scripts/generate-credentials.sh` (the file it writes is removed
again afterwards when it was not there before). They stop when there is no client to bundle, unless given
`--without-google`.

This folder is outside what `scripts/check-scripted-stamps.sh` watches, so changing it does not make the
scripted-suite stamps stale.

## macOS

- **Universal** when both `aarch64-apple-darwin` and `x86_64-apple-darwin` are installed
  (`rustup target add x86_64-apple-darwin`), single-architecture otherwise. Built with
  `MACOSX_DEPLOYMENT_TARGET=11.0`, which `Info.plist.in` repeats as `LSMinimumSystemVersion`.
- **`Info.plist.in`** makes the app an accessory (`LSUIElement`), as the menu bar app is, and carries
  `NSBluetoothAlwaysUsageDescription`, without which macOS ends the process the first time it uses
  Bluetooth.
- **Signed** with the identity `scripts/codesign-identity.sh` finds, ad hoc when there is none. **Not
  notarized**: there is no Developer ID identity on the build machine, so on another Mac the first launch
  needs Open Anyway under System Settings, Privacy & Security. `macos/ReadMe.txt`, which goes into the disk
  image, says so.
- **A new identity for the Keychain.** The bundle's identifier is `au.com.tux.facet`, where a bare binary
  from `scripts/run.sh` has none of its own, so the first launch of the bundle on a machine that has run the
  bare binary asks once for access to the stored device PIN.
- The icon is `Facet.small.svg` below 128 px and `Facet.svg` from there, drawn with Inkscape.

## Linux

- **Has to be built on Linux**, because the radio adapter talks to BlueZ over D-Bus. It needs
  `build-essential`, `pkg-config`, `libdbus-1-dev` and `libfontconfig-dev`.
- **The glibc of the build machine is the minimum for every user.** The script reads the version the binary
  asks for and writes it into the tarball's `README.txt`. Built on Linux Mint 22.3 it is 2.39.
- `linux/facet.desktop` and the icon are in the tarball for a user to install by hand. Nothing installs them.

## Not done

Neither build has been run on a clean machine. Nothing is notarized, there is no `.deb` or AppImage, and
nothing uploads the files anywhere: they are copied into the website repo's `public/download/` by hand.
