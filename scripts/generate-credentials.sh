#!/usr/bin/env bash
# Puts the Google OAuth client into the build, so a distributed copy can sign in.
#
# **This is the only way credentials reach somebody who is not the developer.** `Credentials::resolve`
# tries an environment variable and `~/.config/facet/google-client.json` first, and both of those are files
# on one machine. What this writes is the third source, the one that travels with the binary.
#
# It copies the download to `crates/facet-core/resources/google-client.json`, which is gitignored, so neither
# value is ever committed. Not because either is confidential -- under a Desktop OAuth client the "secret"
# is not a secret, and PKCE is what actually protects the exchange -- but so a release build and a developer
# build can point at different projects without editing code, and so the repository stays publishable.
#
# **`crates/facet-core/build.rs` names this file in `rerun-if-changed`**, so Cargo re-runs it whenever the
# file appears, changes or goes, and the build compiles the file in when it is there and leaves it out when
# it is not. That is the answer to both Swift traps in docs/google-oauth-setup.md: a cached existence check
# that went on saying no after the file arrived, and a build copy that outlived the file.
#
# **File present means credentials are bundled, file absent means they are not**, so a fresh clone with no
# credentials builds exactly as it always did. This script therefore **removes** the file when it has
# nothing to put there, rather than leaving a stale one behind from a project somebody has stopped using.
#
# Source, in the same order the app itself tries:
#   1. $FACET_GOOGLE_CLIENT_JSON
#   2. ~/.config/facet/google-client.json
#
# Both are the Google console's own download, unedited, so there is nothing to transcribe.
#
#   scripts/generate-credentials.sh            # write it, or remove it if there is no source
#   scripts/generate-credentials.sh --check    # say what would happen, write nothing
#
# Exits 0 whether or not credentials were found: a fork with no Google project builds a working app that
# simply cannot sign in, and the App tab says so in words (`facet_core::google::section`).
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="crates/facet-core/resources/google-client.json"
CHECK=0
[ "${1:-}" = "--check" ] && CHECK=1

check_usable() {
    python3 - "$1" <<'ENDPY'
import json, sys
try:
    with open(sys.argv[1]) as handle:
        document = json.load(handle)
except Exception as error:
    sys.stderr.write("  not readable as JSON: %s\n" % error)
    sys.exit(1)
installed = document.get("installed")
if installed is None:
    have = ", ".join(sorted(document)) or "nothing"
    sys.stderr.write('  no "installed" object; found %s. A "web" key means the wrong client type.\n' % have)
    sys.exit(1)
if not installed.get("client_id") or not installed.get("client_secret"):
    sys.stderr.write('  "installed" is missing client_id or client_secret\n')
    sys.exit(1)
print(installed["client_id"])
ENDPY
}

source_file=""
if [ -n "${FACET_GOOGLE_CLIENT_JSON:-}" ]; then
    candidate="${FACET_GOOGLE_CLIENT_JSON/#\~/$HOME}"
    [ -f "$candidate" ] && source_file="$candidate"
fi
if [ -z "$source_file" ] && [ -f "$HOME/.config/facet/google-client.json" ]; then
    source_file="$HOME/.config/facet/google-client.json"
fi

if [ -z "$source_file" ]; then
    if [ "$CHECK" = "1" ]; then
        echo "no credentials found; $OUT would be removed"
        exit 0
    fi
    if [ -f "$OUT" ]; then
        rm -f "$OUT"
        echo "No Google credentials found, so the bundled $OUT was removed."
    else
        echo "No Google credentials found. This build will not be able to sign in to Google."
    fi
    echo "  Looked at: \$FACET_GOOGLE_CLIENT_JSON, then ~/.config/facet/google-client.json"
    echo "  See docs/google-oauth-setup.md. Everything else in the app works without them."
    exit 0
fi

# Not `client_id=$(check_usable ...)` alone: `set -e` does not fire on a failing command substitution in an
# assignment, so the status has to be taken deliberately or an unusable file would be copied in silently
# (see the "Nothing fails silently" rule in CLAUDE.md).
set +e
client_id=$(check_usable "$source_file")
status=$?
set -e
if [ "$status" -ne 0 ]; then
    echo "error: $source_file is not a usable Desktop OAuth client download (exit $status)." >&2
    exit 1
fi

readable_source=$(printf '%s' "$source_file" | sed "s|$HOME|~|")

if [ "$CHECK" = "1" ]; then
    echo "would write $OUT from $readable_source (client id ending ...${client_id: -14})"
    exit 0
fi

# Copied verbatim rather than rewritten into some smaller shape of our own. It is already exactly what
# `facet_core::google::Credentials::from_json` reads, so the bundled resource and the two developer overrides are one
# reader and one format, with no second parser to drift away from the first.
cp "$source_file" "$OUT"

echo "Wrote $OUT from $readable_source (client id ending ...${client_id: -14})."
