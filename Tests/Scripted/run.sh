#!/bin/bash
# Runs the scripted checks, in order, and writes everything to logs/screen.txt as well as the terminal.
#
#   Tests/Scripted/run.sh                  everything, in order, against a brand new test database
#   Tests/Scripted/run.sh --keep           against the test database as it stands, rows and all
#   Tests/Scripted/run.sh 04               just the scripts whose name contains "04"
#   Tests/Scripted/run.sh categories       substring matching works on the name too
#   Tests/Scripted/run.sh --keep-running   leave the app up at the end (for looking at what failed)
#
# **A clean database by default.** These scripts create categories and time entries and delete nothing,
# so run after run the test database fills with them and every list gets longer. Starting from the DDL
# each time means a run says what the app does from nothing, rather than what it does on top of whatever
# the last fortnight left -- and a check that only passes because of a row an earlier run happened to
# make is a check that will fail for somebody else.
#
# `--keep` is for the opposite need: looking at what a failed run left, or keeping a Google account
# connected between runs. **A clean database has no Google account in it**, so 10-google-calendar skips
# unless you sign in again or pass `--keep`. The refresh token is in the Keychain and survives, but the
# account the app reads is in the database and does not.
#
# **The order matters.** Each script leaves the app in a state the next one can start from, and the
# early ones check the things the later ones depend on -- there is no point testing the Report tab's
# totals if recording a time entry is broken, and the failure would be reported in the wrong place.
# So a failing script stops the run.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

# **Which machine this is, before anything else.** Every path, every process name, the stamp this run
# writes and the way the app is quit all come from here -- see `platform.sh` for why the step is shared and
# only the method differs.
source Tests/Scripted/platform.sh || exit 2

# **Refused here rather than discovered halfway down.** Without an app to drive, a run would rebuild a
# database, start a log, and then fail on the first thing that needs a window -- leaving a half-written
# record of a run that never had anything to test. One clear refusal is better than that.
#
# **This stopped refusing on Linux of its own accord**, which is what it was written to do: it asks
# SwiftPM whether the executable product exists rather than being told, so item 11 adding `FacetLinux`
# turned it green with nothing here to remember to change (confirmed 2026-09-20).
platform_app_is_declared
case $? in
    0) ;;
    1)  echo "There is no app to drive on this platform, so this suite cannot run here."
        echo "Package.swift declares no executable product for $PLATFORM."
        echo "platform.sh resolves $SUPPORT and would write $STAMP."
        exit 2 ;;
    *)  echo "Cannot tell whether there is an app to drive; the reason is above."
        exit 2 ;;
esac

mkdir -p logs
# Everything this run prints goes to logs/screen.txt as well, freshly overwritten each time, so there is
# one file to read afterwards (or to watch with `tail -f` while it runs).
#
# **Colour on the terminal, plain text in the file.** The colours are escape sequences, and a file full of
# them opens in an editor as `ESC[0;32m    PASS` -- the thing the colour was meant to make readable, made
# worse. So the copy going to the file has them stripped on the way through, live, rather than the file
# being cleaned up at the end: the point of it is being watchable while a run happens, and a run that is
# interrupted is exactly when somebody wants to read it.
#
# perl rather than sed because it is what portably understands `\e`; both ship with macOS.
exec > >(tee >(perl -pe 's/\e\[[0-9;]*m//g' > logs/screen.txt)) 2>&1

KEEP_RUNNING=0
KEEP_DATABASE=0
FILTER=""
for arg in "$@"; do
    case "$arg" in
        --keep-running) KEEP_RUNNING=1 ;;
        --keep) KEEP_DATABASE=1 ;;
        # A failed check stops its script, since everything below it starts from a state that was never
        # reached. This runs the rest anyway, for the pass where finding every independent fault at once
        # matters more than reading the result cleanly.
        --keep-going) export FAIL_FAST=0 ;;
        *) FILTER="$arg" ;;
    esac
done

scripts=()
for script in Tests/Scripted/[0-9][0-9]-*.sh; do
    [ -e "$script" ] || continue
    if [ -n "$FILTER" ]; then
        case "$script" in *"$FILTER"*) ;; *) continue ;; esac
    fi
    scripts+=("$script")
done

if [ ${#scripts[@]} -eq 0 ]; then
    echo "No scripts matched${FILTER:+ '$FILTER'}."
    exit 2
fi

echo "Facet scripted checks -- $(date '+%Y-%m-%d %H:%M:%S')"
echo "$(git rev-parse --abbrev-ref HEAD) at $(git rev-parse --short HEAD)"

# A run starts with the cube's whereabouts unknown. `00-setup` asks once and writes the answer here against this
# run's id, so the file already invalidates itself; this makes that unconditional, for the case where the run ids
# start again because somebody deleted logs/testlog.sqlite.
rm -f logs/device-gate

# The app holds the database open, so it goes first whichever way this run is going: rebuilding under a
# running app would leave it writing to a file nothing points at any more.
if platform_app_is_running; then
    echo "Quitting the running app first."
    # **Said out loud when it does not work.** run.sh does not source lib.sh, so it has no `click_left`; what it
    # must not do is what every call site used to and throw the error away. A click that fails here is why the
    # `platform_kill_app` below is reached, and knowing that is the difference between a tidy quit and a killed
    # app whose quit sequence never ran. `platform_quit_app` keeps that reporting on both platforms.
    platform_quit_app
    sleep 1.5
    platform_app_is_running && platform_kill_app
    sleep 0.5
fi

if [ "$KEEP_DATABASE" -eq 1 ]; then
    echo "Keeping the test database as it stands (--keep)."
else
    # **Before the rebuild, because afterwards there is nothing left to read.** A Google account is
    # connected once, by hand, through a browser; losing it to every clean run would mean either doing
    # that again each time or leaving the sync untested. 00-setup puts it back once the new database
    # exists. Silent when there is nothing to capture, which is the normal case.
    Tests/Scripted/00-setup.sh --capture || true

    echo "Rebuilding test.sqlite from the DDL, so this run starts from nothing."
    if ! scripts/switch-database.sh test -clean; then
        echo "Could not rebuild the test database; refusing to run against whatever is there instead."
        exit 2
    fi
    # **The trace goes with it, and is rebuilt here rather than left to the app.** It is a separate file now, so
    # `switch-database.sh` does not touch it, and a run that said it was starting from nothing would otherwise inherit
    # every row the last one wrote.
    #
    # Rebuilt immediately, because waiting for the app to do it is a race this run lost on 2026-08-22: `01-launch`
    # calls `mark` before the app it is about to check has started, `sqlite3` creates the file it was pointed at, and
    # the query then fails with "no such table". Every check after that measured from an empty baseline and produced a
    # malformed query. The suite owns its databases; the app is what is being tested, not what prepares the ground.
    rm -f "$DEBUG_DB"
    for ddl in database/5*.sql; do
        { echo "PRAGMA foreign_keys = ON;"; cat "$ddl"; } | sqlite3 "$DEBUG_DB"
    done

    if [ ! -f "$HOME/.config/facet/scripted-seed.json" ]; then
        echo "Note: a new database has no Google account, so 10-google-calendar will FAIL."
        echo "      Connect one on the App tab once; it is captured and reseeded from then on."
    fi
fi

echo "${#scripts[@]} script(s) to run"

# The durable record, alongside screen.txt. See Tests/Scripted/testlog.sh for what it holds and why one
# overwritten text file is not enough to work out why something failed.
# `DB` and `DEBUG_DB` are `platform.sh`'s, sourced at the top: testlog.sh reads the app's log out of the
# trace's file, so both have to be settled before it is sourced, and they are.
source Tests/Scripted/testlog.sh
TESTLOG_RUN_ID=$(testlog_run_start "$((1 - KEEP_DATABASE))" "$FILTER" "run.sh $*")
export TESTLOG_RUN_ID
echo "Recording this run as run_id $TESTLOG_RUN_ID in logs/testlog.sqlite"

ran=0
failed=""
for script in "${scripts[@]}"; do
    if bash "$script"; then
        ran=$((ran + 1))
    else
        failed="$script"
        break
    fi
done

# **One question decides it: did every script run every check it declares?**
#
# `passed` is a word somebody reads at the top of the stamp and stops reading, so it has to mean the whole run
# answered. `00-setup` wrote a row for every script before any of them started, each carrying the count it declares,
# so a script that failed and a script that was never reached are the same shape in the table: `passed` short of
# `expected`. Counting those covers both, and covers the case neither an outcome nor a failure count can show --
# a script that ran, went green, and quietly did less than it says it does.
#
# This replaced a skip count that could no longer be anything but zero. On 2026-08-22 a run wrote `outcome: passed`
# with `1 skipped` under it and the script that skipped was `55-device-face`, which had tested nothing whatsoever;
# the shape of that fault is now caught by arithmetic rather than by a verdict the scripts have to remember to use.
short=$(testlog_short_scripts "$TESTLOG_RUN_ID")
if [ -n "$failed" ] || [ "${short:-0}" -gt 0 ]; then
    outcome=failed
else
    outcome=passed
fi
[ "${short:-0}" -gt 0 ] && echo "$short script(s) ran fewer checks than they declare."
testlog_run_finish "$TESTLOG_RUN_ID" "$outcome" "$ran"

# The committed half of the record. Written either way, because a stamp that only appeared on success
# would let a failing branch keep an older passing one -- which is the staleness it exists to catch.
# **Named for the machine that produced it**, because a run only says what works where it ran.
# `platform.sh` decides which, and `check_interactive_checklists.sh` reads both.
testlog_stamp "$TESTLOG_RUN_ID" "$STAMP"

echo ""
echo "=============================================================================="
if [ -n "$failed" ]; then
    printf '\033[0;31m%s\033[0m\n' "STOPPED at $failed after $ran script(s) passed."
    echo "The rest were not run: each one starts from the state the last one left."
else
    printf '\033[0;32m%s\033[0m\n' "ALL $ran SCRIPT(S) PASSED."
fi
echo "This output is also in logs/screen.txt"
echo "=============================================================================="

if [ "$KEEP_RUNNING" -eq 0 ]; then
    # Left running only when asked, so a failed run can be looked at. Otherwise the app goes away:
    # a status item left behind is a second instance's worth of confusion next time.
    platform_app_is_running && {
        platform_quit_app
        sleep 1
        platform_app_is_running && platform_kill_app
    }
fi

# **After the quit, not before it.** Each script copies its own window as it ends, so nothing covers what the app
# logs once the checks stop: the cube reconnecting, and the shutdown just above. Copying here is what puts the end
# of a run into the record, and the end is what a run that died on its way out is read for.
testlog_copy_run_tail "$TESTLOG_RUN_ID"

# The plain-text copy is written by a process of its own, so give it a moment to drain before this shell
# exits and the pipe goes with it. Without this the last few lines can be missing from the file -- and
# the last few lines are the summary.
sleep 0.5

[ -z "$failed" ]
