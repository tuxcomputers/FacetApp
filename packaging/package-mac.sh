#!/usr/bin/env bash
# Builds Facet.app for macOS and wraps it in a disk image.
#
#   packaging/package-mac.sh                    build for every installed macOS target, make dist/Facet-<version>-macos.dmg
#   packaging/package-mac.sh --without-google   package a build that cannot sign in to Google
#
# The app is universal when both aarch64-apple-darwin and x86_64-apple-darwin are installed
# (rustup target add x86_64-apple-darwin), and single-architecture otherwise.
#
# The Google OAuth client is bundled through scripts/generate-credentials.sh. The file it writes is removed
# again on exit when it was not there before, so a developer build afterwards is built as it was.
#
# The app is signed with the identity scripts/codesign-identity.sh finds, and ad hoc when there is none. It is
# not notarized, so macOS asks the user to confirm the first launch.
#
# Output: dist/Facet.app, dist/Facet-<version>-macos.dmg and dist/Facet-<version>-macos.dmg.sha256.
set -euo pipefail

cd "$(dirname "$0")/.."
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

fail() { echo "error: $*" >&2; exit 1; }

WITHOUT_GOOGLE=0
for argument in "$@"; do
    case "$argument" in
        --without-google) WITHOUT_GOOGLE=1 ;;
        *) echo "usage: packaging/package-mac.sh [--without-google]" >&2; exit 2 ;;
    esac
done

[ "$(uname -s)" = "Darwin" ] || fail "this builds the macOS app and has to run on a Mac"

VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
[ -n "$VERSION" ] || fail "no workspace version found in Cargo.toml"

# ---------------------------------------------------------------------------------------------- credentials
CREDENTIALS="crates/facet-core/resources/google-client.json"
HAD_CREDENTIALS=0
[ -e "$CREDENTIALS" ] && HAD_CREDENTIALS=1
leave_as_found() {
    if [ "$HAD_CREDENTIALS" = "0" ]; then rm -f "$CREDENTIALS"; fi
}
trap leave_as_found EXIT

scripts/generate-credentials.sh
if [ -e "$CREDENTIALS" ]; then
    echo "==> Google client bundled"
elif [ "$WITHOUT_GOOGLE" = "1" ]; then
    echo "==> warning: no Google client found, so this build cannot sign in to Google"
else
    fail "no Google client to bundle, so the app could not sign in. Put the console download at ~/.config/facet/google-client.json, or pass --without-google"
fi

# ---------------------------------------------------------------------------------------------- build
TARGETS=()
for triple in aarch64-apple-darwin x86_64-apple-darwin; do
    if rustup target list --installed | grep -qx "$triple"; then TARGETS+=("$triple"); fi
done
[ "${#TARGETS[@]}" -gt 0 ] || fail "no macOS Rust target is installed"
[ "${#TARGETS[@]}" -eq 2 ] || echo "==> warning: only ${TARGETS[*]} is installed, so the app will not be universal"

export MACOSX_DEPLOYMENT_TARGET=11.0
BINARIES=()
for triple in "${TARGETS[@]}"; do
    echo "==> Building facet-mac for $triple"
    cargo build --release --locked -p facet-mac --target "$triple" || fail "the build for $triple failed"
    BINARIES+=("target/$triple/release/facet-mac")
done

# ---------------------------------------------------------------------------------------------- bundle
APP="dist/Facet.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

if [ "${#BINARIES[@]}" -eq 2 ]; then
    lipo -create "${BINARIES[@]}" -output "$APP/Contents/MacOS/facet-mac" || fail "lipo failed"
else
    cp "${BINARIES[0]}" "$APP/Contents/MacOS/facet-mac"
fi
strip -x "$APP/Contents/MacOS/facet-mac" || fail "strip failed"

sed "s/@VERSION@/$VERSION/g" packaging/macos/Info.plist.in > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist" >/dev/null || fail "Info.plist is not valid"

# The icon: Facet.small.svg below 128 px, where the lighter lines of Facet.svg thin out, Facet.svg from there.
ICONSET="dist/Facet.iconset"
rm -rf "$ICONSET"
mkdir -p "$ICONSET"
render() {   # <svg> <pixels> <file>
    inkscape "$1" -w "$2" -h "$2" --export-filename="$ICONSET/$3" >/dev/null 2>&1 || fail "inkscape could not draw $3"
    [ -s "$ICONSET/$3" ] || fail "inkscape wrote nothing for $3"
}
render Facet.small.svg 16 icon_16x16.png
render Facet.small.svg 32 icon_16x16@2x.png
render Facet.small.svg 32 icon_32x32.png
render Facet.small.svg 64 icon_32x32@2x.png
render Facet.svg 128 icon_128x128.png
render Facet.svg 256 icon_128x128@2x.png
render Facet.svg 256 icon_256x256.png
render Facet.svg 512 icon_256x256@2x.png
render Facet.svg 512 icon_512x512.png
render Facet.svg 1024 icon_512x512@2x.png
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Facet.icns" || fail "iconutil failed"
rm -rf "$ICONSET"

cp LICENSE.md NOTICE.md "$APP/Contents/Resources/"

# ---------------------------------------------------------------------------------------------- sign
IDENTITY=$(scripts/codesign-identity.sh 2>/dev/null) || IDENTITY=""
if [ -n "$IDENTITY" ]; then
    echo "==> Signing as $IDENTITY"
else
    echo "==> warning: no codesigning identity, so the app is signed ad hoc"
    IDENTITY="-"
fi
codesign --force --sign "$IDENTITY" --timestamp=none "$APP" || fail "codesign failed"
codesign --verify --strict --verbose=2 "$APP" || fail "the signature does not verify"

# ---------------------------------------------------------------------------------------------- disk image
DMG="dist/Facet-$VERSION-macos.dmg"
STAGE="dist/dmg-stage"
rm -rf "$STAGE" "$DMG" "$DMG.sha256"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
cp packaging/macos/ReadMe.txt "$STAGE/Read Me.txt"
cp LICENSE.md NOTICE.md "$STAGE/"
if ! output=$(hdiutil create -volname "Facet" -srcfolder "$STAGE" -ov -format UDZO -fs HFS+ "$DMG" 2>&1); then
    fail "hdiutil failed: $output"
fi
rm -rf "$STAGE"
[ -s "$DMG" ] || fail "no disk image was written"

(cd dist && shasum -a 256 "$(basename "$DMG")" > "$(basename "$DMG").sha256")

echo ""
echo "==> $DMG"
lipo -info "$APP/Contents/MacOS/facet-mac"
ls -lh "$DMG" | awk '{print "    size " $5}'
cat "$DMG.sha256"
