#!/usr/bin/env bash
# Puts the Google OAuth client into the build, so a distributed copy can sign in.
#
# **This is the only way credentials reach somebody who is not the developer.** `GoogleCredentials.resolve`
# tries an environment variable and `~/.config/facet/google-client.json` first, and both of those are files
# on one machine. What this writes is the third source, the one that travels with the binary.
#
# It copies the download to `Sources/FacetCore/Resources/google-client.json`, which is gitignored, so neither
# value is ever committed. Not because either is confidential -- under a Desktop OAuth client the "secret"
# is not a secret, and PKCE is what actually protects the exchange (see `GoogleOAuthRules.pkce`) -- but so a
# release build and a developer build can point at different projects without editing code, and so the
# repository stays publishable without a second thought.
#
# **A resource rather than a generated Swift file, and that is not a stylistic choice.** The Swift-file
# version was built first: a gitignored source file, with `Package.swift` checking whether it existed and
# defining `HAS_BUNDLED_CREDENTIALS` when it did. It fails silently, which is the one thing it must not do.
# SwiftPM caches the manifest, so the existence check does not re-run when this script creates the file --
# measured on 2026-08-30: generator runs, `swift build` succeeds, define absent, credentials quietly not in
# the binary and nothing anywhere saying so. `Package.swift` already `.process`es the whole `Resources`
# directory, and that is a directory scan at build time rather than a manifest-time decision, so a file
# appearing in it is picked up with no cache to defeat.
#
# **File present means credentials are bundled, file absent means they are not**, and the app finds that out
# at runtime rather than at compile time -- so a fresh clone with no credentials builds exactly as it always
# did. This script therefore **removes** the resource when it has nothing to put there, rather than leaving
# a stale one behind from a project somebody has stopped using.
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
# simply cannot sign in, and the App tab says so in words (`GoogleAccountRules.noCredentialsNote`).
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="Sources/FacetCore/Resources/google-client.json"
CHECK=0
[ "${1:-}" = "--check" ] && CHECK=1

# Copies SwiftPM has already made into the resource bundle.
#
# **SwiftPM does not prune a resource that has gone from the source directory** (measured 2026-08-30:
# delete it, `swift build`, and the copy under `.build/.../FacetCore_FacetCore.bundle/` is still there).
# Removing the source alone would therefore leave a build still carrying credentials the developer has
# just taken away -- and swift-bundler builds the `.app` from those same products. Adding credentials
# needs no such help: a new file in a processed directory is picked up on the next build.
prune_built_copies() {
    local found=0
    while IFS= read -r stale; do
        rm -f "$stale"
        found=1
    done < <(find .build -name "google-client.json" -path "*FacetCore*" 2>/dev/null)
    [ "$found" = "1" ] && echo "  also removed the copies SwiftPM had already put in .build"
    return 0
}

# The console's download for a Desktop client nests everything under `installed`. A `web` key means the
# wrong client type was made, and is refused here rather than half-accepted -- the same judgement
# `GoogleCredentials.fromJSON` makes at runtime, checked here as well so the failure lands at build time
# where somebody is watching, instead of as an app that cannot sign in for reasons nobody can see.
# Prints the client id, so the caller can say which project it took without printing the secret.
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
    prune_built_copies
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
# `GoogleCredentials.fromJSON` reads, so the bundled resource and the two developer overrides are one
# reader and one format, with no second parser to drift away from the first.
cp "$source_file" "$OUT"

echo "Wrote $OUT from $readable_source (client id ending ...${client_id: -14})."
