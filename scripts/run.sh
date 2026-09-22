#!/usr/bin/env bash
#
# Build if anything changed, then run Facet. Works on macOS and on Linux.
#
# Cargo already decides whether a build is needed, so this does not try to be cleverer than it. What
# this adds is the things that are easy to get wrong by hand: finding cargo, picking the crate for
# the platform, and not ending up with two icons in the menu bar.
#
# Runs in the foreground so the debug log streams to the terminal. Ctrl-C stops it, and so does Quit
# on the menu.

# ---------------------------------------------------------------------------------------------
# The shell, before anything that needs a modern one.
#
# The order matters and is the point: check the bash already running first, and only if that is too
# old go looking for a newer one, and only if that fails as well say anything. The Linux box is on
# 5.2 and sails straight through without ever seeing a macOS-flavoured warning about Homebrew.
# ---------------------------------------------------------------------------------------------
if [ "${BASH_VERSINFO[0]:-0}" -lt 4 ]; then
    # In practice only macOS reaches here: it still ships bash 3.2 as /bin/bash, from 2007, and
    # `env bash` finds that one whenever Homebrew is not first on PATH.
    for candidate in /opt/homebrew/bin/bash /usr/local/bin/bash /usr/bin/bash /bin/bash; do
        [ -x "$candidate" ] || continue
        candidate_major="$("$candidate" -c 'echo "${BASH_VERSINFO[0]}"' 2>/dev/null)" || continue
        case "$candidate_major" in ''|*[!0-9]*) continue ;; esac
        [ "$candidate_major" -ge 4 ] || continue
        exec "$candidate" "$0" "$@"
    done

    printf 'error: this script needs bash 4 or newer and is running under %s.\n' \
        "${BASH_VERSION:-an unknown bash}" >&2
    printf '       No newer bash was found either.\n' >&2
    if [ "$(uname -s)" = "Darwin" ]; then
        printf '       On macOS: brew install bash\n' >&2
    else
        printf '       Install one through your package manager.\n' >&2
    fi
    exit 1
fi

set -o errexit
set -o nounset
set -o pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# ---------------------------------------------------------------------------------------------
# The platform, asked of the one file that decides it.
#
# **Not worked out again here.** Tests/Scripted/platform.sh is where the suite decides what every path
# is, and a script that answered the question for itself would be a second copy of a fact that can
# disagree with the first. It already has: a script that hardcoded the macOS directory resolved every
# path under a directory that does not exist on Linux and reported a database nobody had asked about.
# ---------------------------------------------------------------------------------------------
if [ ! -r Tests/Scripted/platform.sh ]; then
    echo "error: Tests/Scripted/platform.sh is missing, and it is what decides where things are." >&2
    exit 1
fi
# shellcheck source=../Tests/Scripted/platform.sh
source Tests/Scripted/platform.sh

crate="$CRATE"
data_dir="$SUPPORT"
swift_process="$LEGACY_PROCESS_NAME"

profile="debug"
cargo_args=()
app_args=()

usage() {
    cat <<'USAGE'
usage: scripts/run.sh [-r] [-b] [-c] [-- <args passed to the app>]

  -r, --release    build and run the optimised binary instead of the debug one
  -b, --rebuild    throw target/ away first, so the next build is from scratch
  -c, --clean      delete the local databases, after asking. This is about DATA, not the build:
                   -b is the one that cleans build output
  -h, --help       this

Anything after -- goes to the app. Cargo decides whether a build is needed; there is no separate
build step to remember.
USAGE
}

# Delete the local databases, after showing exactly what would go and asking.
#
# **These are Facet's own now.** They were TimeFlip's when this script was written, back when the two
# apps shared a directory; the rename gave Facet `~/Library/Application Support/Facet` and left TimeFlip
# its own, so nothing here can reach the other app's data any more. What it *can* reach is
# production.sqlite, which is where this app's real recorded time will land, so the asking stays.
#
# It guards on Facet rather than on TimeFlip for the same reason: the process that has these files open
# is this one.
clean_databases() {
    if pgrep -x "$crate" >/dev/null 2>&1; then
        echo "error: $crate is running and has these databases open. Quit it first." >&2
        exit 1
    fi

    local found=()
    local db suffix f
    for db in appdata production test debug prod; do
        for suffix in "" "-wal" "-shm"; do
            [ -e "$data_dir/$db.sqlite$suffix" ] && found+=("$db.sqlite$suffix")
        done
    done

    if [ ${#found[@]} -eq 0 ]; then
        echo "Nothing to clean: no databases in $data_dir"
        return
    fi

    echo "This deletes the following from $data_dir:"
    for f in "${found[@]}"; do
        printf '    %8s  %s\n' "$(du -h "$data_dir/$f" | cut -f1)" "$f"
    done
    echo
    echo "WARNING: production.sqlite is where this app records real time. Deleting it loses that time."
    echo "         debug.sqlite is only the trace and costs nothing; test.sqlite is rebuilt by"
    echo "         scripts/switch-database.sh test -clean. The backup/ directory is left alone."
    printf 'Continue? [y/N] '

    # The terminal where there is one, because stdin may be the script itself. Falling back to stdin
    # rather than failing keeps this answerable when there is no tty, and an unusable /dev/tty is
    # caught by trying it rather than by testing for it: the node can exist and still refuse to open.
    local confirm=""
    # The braces matter: the shell reports a failed `< /dev/tty` itself, before a trailing 2>/dev/null
    # on the read would apply, so the redirection has to be inside the group being silenced. Without
    # them a machine with no usable tty prints "Device not configured" and looks broken.
    if ! { read -r confirm < /dev/tty; } 2>/dev/null; then
        read -r confirm || confirm=""
    fi

    case "$confirm" in
        [yY]|[yY][eE][sS])
            for f in "${found[@]}"; do
                rm -f "$data_dir/$f"
            done
            echo "Deleted ${#found[@]} file(s)."
            ;;
        *)
            echo "Aborted; nothing was deleted."
            exit 1
            ;;
    esac
}

while [ $# -gt 0 ]; do
    case "$1" in
        -r|--release) profile="release"; cargo_args+=("--release"); shift ;;
        -b|--rebuild)
            echo "==> Removing target/ for a build from scratch"
            rm -rf target
            shift
            ;;
        -c|--clean) clean_databases; shift ;;
        --help|-h) usage; exit 0 ;;
        --) shift; app_args=("$@"); break ;;
        *)
            echo "error: unknown option $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

# On the Mac, ~/.cargo/bin is on no dotfile's PATH, so a bare `cargo` fails with "command not found"
# while the toolchain is perfectly healthy. The Linux box does source it from .bashrc and .profile,
# but a shell with no environment still would not, so look either way.
if ! command -v cargo >/dev/null 2>&1; then
    if [ -r "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    elif [ -x "$HOME/.cargo/bin/cargo" ]; then
        PATH="$HOME/.cargo/bin:$PATH"
    fi
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "error: no cargo on PATH and none at ~/.cargo/bin/cargo. Install it from https://rustup.rs" >&2
    exit 1
fi

binary="target/$profile/$crate"

# Two copies in the menu bar is confusing rather than dangerous today, and it stops being merely
# confusing the moment this app opens a database. Replace the running one rather than adding to it.
if pgrep -x "$crate" >/dev/null 2>&1; then
    echo "==> Stopping the copy already running"
    pkill -x "$crate" || true
    for _ in {1..20}; do
        pgrep -x "$crate" >/dev/null 2>&1 || break
        sleep 0.1
    done
    if pgrep -x "$crate" >/dev/null 2>&1; then
        echo "error: the running $crate would not stop; quit it from its menu and try again" >&2
        exit 1
    fi
fi

# **Two menu bars, two directories, and only the first is a problem.** Since the rename the two apps keep
# separate data, so the warning is about the menu bar being confusing rather than about anything being at
# risk. TimeFlip still owns the time that has actually been recorded, this app not being able to track any.
if pgrep -x "$swift_process" >/dev/null 2>&1; then
    echo "note: $swift_process is running too, so there will be two icons in the menu bar."
    echo "      They keep separate databases; that one owns the time recorded so far."
fi

echo "==> Building $crate ($profile)"
cargo build --locked ${cargo_args[@]+"${cargo_args[@]}"} -p "$crate"

if [ ! -x "$binary" ]; then
    echo "error: the build reported success but $binary is not there" >&2
    exit 1
fi

# **Signed with a real certificate where there is one, and left ad-hoc where there is not.**
#
# This is what stops macOS asking for Keychain access after every single rebuild. Cargo leaves a
# linker-applied ad-hoc signature whose designated requirement is the binary's own hash, so each build
# is a different application to the Keychain and the permission granted to the last one matches
# nothing. A certificate makes the requirement stable, so Always Allow is answered once and holds.
#
# Re-signed on every run rather than only after a rebuild: cargo rewrites the binary whenever it
# rebuilds, taking the signature with it, and `codesign` on an already-signed unchanged binary is
# cheap. Failure is reported and does not stop the launch, because an unsigned app still runs and the
# only cost is the prompt.
if [ "$(uname -s)" = "Darwin" ]; then
    identity="$(scripts/codesign-identity.sh || true)"
    if [ -n "$identity" ]; then
        if codesign --force --sign "$identity" --timestamp=none "$binary" 2>/tmp/facet-codesign.err; then
            echo "==> Signed as: $identity"
        else
            echo "warning: signing failed, so this build is ad-hoc and the Keychain will ask again:" >&2
            sed 's/^/    /' /tmp/facet-codesign.err >&2
        fi
        rm -f /tmp/facet-codesign.err
    else
        echo "note: no codesigning identity, so this build is ad-hoc signed."
        echo "      macOS will ask for Keychain access again after every rebuild."
    fi
fi

echo "==> Running $binary"
echo "    Right click the menu bar icon for the menu. Ctrl-C here also stops it."
echo
exec "$binary" ${app_args[@]+"${app_args[@]}"}
