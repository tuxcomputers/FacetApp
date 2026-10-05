#!/usr/bin/env bash
# Builds the Linux release and packs it as a tarball.
#
#   packaging/package-linux.sh                    build, make dist/Facet-<version>-linux-<arch>.tar.gz
#   packaging/package-linux.sh --without-google   package a build that cannot sign in to Google
#
# Has to run on Linux: the radio adapter talks to BlueZ over D-Bus, which does not cross-compile. Needs
# build-essential, pkg-config, libdbus-1-dev and libfontconfig-dev.
#
# The Google OAuth client is bundled through scripts/generate-credentials.sh. The file it writes is removed
# again on exit when it was not there before.
#
# The binary needs at least the glibc of the machine that built it, and the tarball's README.txt says which
# version that was.
#
# Output: dist/Facet-<version>-linux-<arch>.tar.gz and its .sha256.
set -euo pipefail

cd "$(dirname "$0")/.."
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

fail() { echo "error: $*" >&2; exit 1; }

WITHOUT_GOOGLE=0
for argument in "$@"; do
    case "$argument" in
        --without-google) WITHOUT_GOOGLE=1 ;;
        *) echo "usage: packaging/package-linux.sh [--without-google]" >&2; exit 2 ;;
    esac
done

[ "$(uname -s)" = "Linux" ] || fail "this builds the Linux app and has to run on Linux"

VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
[ -n "$VERSION" ] || fail "no workspace version found in Cargo.toml"
ARCH=$(uname -m)

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
    fail "no Google client to bundle, so the app could not sign in. Put the console download at ~/.config/facet/google-client.json, set FACET_GOOGLE_CLIENT_JSON, or pass --without-google"
fi

# ---------------------------------------------------------------------------------------------- build
echo "==> Building facet-linux"
cargo build --release --locked -p facet-linux || fail "the build failed"
BINARY="target/release/facet-linux"
[ -x "$BINARY" ] || fail "no binary at $BINARY"

NAME="Facet-$VERSION-linux-$ARCH"
STAGE="dist/$NAME"
TARBALL="dist/$NAME.tar.gz"
rm -rf "$STAGE" "$TARBALL" "$TARBALL.sha256"
mkdir -p "$STAGE"

cp "$BINARY" "$STAGE/facet-linux"
strip "$STAGE/facet-linux" || fail "strip failed"

GLIBC=$(objdump -T "$STAGE/facet-linux" | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/GLIBC_//' | sort -V | tail -1)
[ -n "$GLIBC" ] || fail "could not read the glibc version the binary needs"
echo "==> needs glibc $GLIBC or newer"

cp Facet.svg "$STAGE/facet.svg"
cp packaging/linux/facet.desktop "$STAGE/facet.desktop"
cp LICENSE.md NOTICE.md "$STAGE/"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@GLIBC@/$GLIBC/g" -e "s/@ARCH@/$ARCH/g" packaging/linux/README.txt > "$STAGE/README.txt"

# ---------------------------------------------------------------------------------------------- pack
tar -C dist -czf "$TARBALL" "$NAME" || fail "tar failed"
[ -s "$TARBALL" ] || fail "no tarball was written"
rm -rf "$STAGE"
(cd dist && sha256sum "$(basename "$TARBALL")" > "$(basename "$TARBALL").sha256")

echo ""
echo "==> $TARBALL"
ls -lh "$TARBALL" | awk '{print "    size " $5}'
cat "$TARBALL.sha256"
echo "==> shared libraries the binary links"
ldd "$BINARY" | awk '{print "    " $1}' | sort
