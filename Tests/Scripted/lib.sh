#!/bin/bash
# Shared machinery for every script in this folder. Sourced, never run.
#
# What a script written against this looks like: `start "what is being checked"`, then a series of
# `check` calls, then `finish`. Everything else here exists to make those three honest.
#
# Three rules are baked in rather than left to each script to remember:
#
#   1. **It refuses to run against production.** These scripts write real rows -- categories, segments,
#      time entries -- and there is no undo. The database must be the test one.
#   2. **Evidence comes from the database, not from the screen.** A check waits for a `debug_log` row or
#      queries a table. Asking a person "did it work?" records their optimism.
#   3. **Every wait is baselined.** `mark` records the newest `debug_log` id *before* the action, and
#      `wait_for` only looks at rows after it. Without that, a matching row from ten minutes ago passes
#      a step that did nothing -- and this database is used by a person as well as by these scripts, so
#      it is never safe to assert on a total count.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# **`SUPPORT`, `DB`, `DEBUG_DB`, `APP` and `BINARY` all come from `platform.sh`**, along with the one
# variable that decides them, because where the app keeps its database and what its binary is called are
# facts about the machine rather than about this suite. A step written here says what it wants done; that
# file says how this platform does it.
#
# **The trace lives in its own file**, beside the app's rather than inside it, so a check that waits on a `debug_log`
# row reads a different database from one that reads `device_event`. `dsql` is that database's `sql`, and which one a
# query wants is decided by which table it names -- see the two of them below.
#
# It is not a symlink like `appdata.sqlite` and does not need to be: it holds no data worth keeping between runs, and
# `switch-database.sh` repoints the app's file without this one caring which side it lands on.
source "$REPO_ROOT/Tests/Scripted/platform.sh" || exit 2

PASSED=0
FAILED=0
SCRIPT_NAME="$(basename "${BASH_SOURCE[1]:-unknown}" .sh)"
FAILURES=""

# The durable record. `logs/screen.txt` is this run and only what was printed; `logs/testlog.sqlite` keeps
# every run, and keeps the app's own debug_log rows, which the next clean rebuild of test.sqlite destroys.
# Sourced after `DB` is set, because it reads from it.
source "$(dirname "${BASH_SOURCE[0]}")/testlog.sh"

# ---------------------------------------------------------------------------- output

blue()   { printf '\033[1;34m%s\033[0m\n' "$*"; }
green()  { printf '\033[0;32m%s\033[0m\n' "$*"; }
red()    { printf '\033[0;31m%s\033[0m\n' "$*"; }
grey()   { printf '\033[0;90m%s\033[0m\n' "$*"; }
yellow() { printf '\033[1;33m%s\033[0m\n' "$*"; }

# **Blue is the script talking; grey is something being shown.** The two sit at different indents already -- a
# narration line at the check's own two spaces, a value read back under the verdict -- but at one colour they
# read as one stream, and a "watching for 16s..." line looks like evidence for the check above it. So narration
# goes through `step`, and `grey` is left for the value.
step() { blue "  $*"; }

# Stops the run and waits for the person running it. Answers 0 for yes, 1 for anything else.
#
#     action_required "Sign in to Google" "1. A browser opens." "2. Approve the account."
#
# **Drawn big on purpose.** Almost every check here runs untouched, so a run is something you start and
# come back to. A step that needs hands has to survive being scrolled past, which a sentence among two
# hundred lines of PASS does not.
#
# **Reads the terminal directly.** run.sh sends stdout through `tee` into a process substitution, so a
# prompt printed the ordinary way can sit in a buffer while the run looks like it has hung. `/dev/tty` is
# the terminal itself whatever stdout has been pointed at.
#
# **Non-interactive answers no**, rather than waiting for a person who is not there. A run with no
# terminal is CI or a pipe, and a step needing hands is one it should skip and say so.
#
# **Only `y` or `n` is an answer, and anything else asks again.** This used to treat every key that was
# not a yes as a no, which is a mistyped character silently choosing the destructive one: on 2026-08-18 a
# stray `t` skipped the whole Google calendar section, and the run finished green having not checked it.
# A skip is a real answer somebody may want to give, so it keeps its key -- what it no longer has is every
# other key on the keyboard as a synonym. EOF still ends it, so a closed pipe cannot loop for ever.
# ---------------------------------------------------------------------------- waiting on a person
#
# **Seconds this script spent waiting for a person, rather than for the app.** Kept apart from the script's own
# time because they are different measurements answering different questions: how long the app took is worth
# watching for a regression, and how long somebody took to answer a prompt is worth nothing at all except that it
# should not be counted against the app. A device script that usually takes two minutes and took thirty because
# whoever was running it went to make coffee looks exactly like one that has broken.
#
# Accumulated here rather than at the call sites: every prompt in this file goes through one of the four helpers
# below, so a new script asking for something gets its wait discounted without knowing this exists.
HUMAN_SECONDS=0

# Adds the time since `$1` (a `$SECONDS` reading taken before the prompt went up) to the run's human total.
note_human_wait() {
    HUMAN_SECONDS=$(( HUMAN_SECONDS + SECONDS - ${1:-$SECONDS} ))
}

action_required() {
    local title="$1"
    shift
    echo ""
    yellow "##############################################################################"
    yellow "##"
    yellow "##  ACTION REQUIRED"
    yellow "##"
    yellow "##  $title"
    yellow "##"
    local line
    for line in "$@"; do
        yellow "##    $line"
    done
    yellow "##"
    yellow "##############################################################################"
    echo ""

    if [ ! -r /dev/tty ]; then
        step "no terminal to ask, so this is being skipped"
        return 1
    fi

    local answer="" asked=$SECONDS
    while true; do
        printf '  Answer y (go ahead) or n (skip this): '
        read -r answer < /dev/tty || { note_human_wait "$asked"; return 1; }
        case "$answer" in
            y | Y | yes | YES | Yes) note_human_wait "$asked"; echo ""; return 0 ;;
            n | N | no | NO | No) note_human_wait "$asked"; echo ""; return 1 ;;
            *) red "  '$answer' is not an answer here. Type y or n." ;;
        esac
    done
}

# Holds the run until the person running it says go. Unlike `action_required` there is nothing to decide:
# any answer continues, and the return is always 0.
#
# **For the point where something has just happened and is worth watching.** A script that detects a state
# change and races straight on gives nobody time to bring a window forward or start looking, and the part
# they wanted to see is over before they find it. This stops instead, says what it found, and waits.
#
# Reads `/dev/tty` for the same reason `action_required` does: run.sh pipes stdout through tee, so an
# ordinary prompt can sit in a buffer while the run looks like it has hung. With no terminal it carries on
# rather than blocking a run nobody is watching.
wait_for_dev() {
    local title="$1"
    shift
    echo ""
    yellow "##############################################################################"
    yellow "##"
    yellow "##  PAUSED -- $title"
    yellow "##"
    local line
    for line in "$@"; do
        yellow "##    $line"
    done
    yellow "##"
    yellow "##############################################################################"
    echo ""

    if [ ! -r /dev/tty ]; then
        step "no terminal to wait on, so carrying straight on"
        return 0
    fi
    # **No key in particular**, unlike `action_required`: there is nothing being decided here, so there is no
    # wrong key to press and nothing a mistyped one could choose. Asking for `y` would only be a rule with
    # no answer behind it.
    local asked=$SECONDS
    printf '  Press Return when you are ready to carry on: '
    read -r _ < /dev/tty || true
    note_human_wait "$asked"
    echo ""
    return 0
}

# Asks for something only a pair of hands can do, then watches the database until it has happened.
#
#     ask_and_detect "SELECT message FROM debug_log WHERE ... ;" \
#         "Turn the cube to the Break face" \
#         "Break is face 8; the app is watching for it"
#
# Answers 0 once the query returns anything at all, and 1 when there is no terminal to ask.
#
# **Detected, not confirmed**, which is the archive's rule and the reason nothing here says "have you done
# that? (y/n)". A person answering yes records their optimism; the app's own row records the cube. It is
# also what makes a mis-turn harmless: the query names the face that was asked for, so turning the cube to
# some other one simply does not satisfy it and the banner is still on screen saying which one.
#
# **No timeout, deliberately.** A physical action takes as long as the person takes, and a run that failed
# because they answered the door would be failing about the wrong thing. `actions.py`
# reached the same place from the other direction: `act_ask_user_or_detect` treats `timeout_seconds = 0` as
# "wait indefinitely" and says why in the same words. It reports that it is still waiting every half minute,
# so an unattended run reads as waiting rather than as hung.
#
# **A run with no terminal skips rather than blocking.** That is CI or a pipe, and there is nobody there to
# turn anything -- the same answer `action_required` gives, for the same reason.
# The query to hand `ask_and_detect` when the cube must be **on** a face and still there: `on_face_now <mark> <face>`.
#
# **Two clauses, and they answer two different questions.** `debug_log_id > mark` means *a turn is required*, so this
# is not already satisfied by the face the cube happens to be resting on -- without it `ask_and_detect` would skip the
# ask entirely. The `MAX` clause means *and that is where it still is*, which is the half a plain match cannot do: a
# face row, once written, is there for ever, so a person who lands on the wanted face and keeps turning satisfies a
# plain match and the script carries on against a cube somewhere else. The failures that follow are then about
# whatever the next check reads.
#
# A wrong turn *before* the right one is harmless either way, which is `ask_and_detect`'s own note: the row it writes
# is not the row being waited for. This is about the turn after the right one.
#
# **Not for every ask.** `62`'s first one deliberately matches any face, because it is discovering which face 627 is,
# and `57`'s polls `device_event` for a segment rather than a face row. Both mean something else and keep their own
# query.
on_face_now() {
    printf "SELECT message FROM debug_log WHERE debug_log_id = (SELECT MAX(debug_log_id) FROM debug_log WHERE tag = 'face' AND message LIKE 'Face %% is up') AND debug_log_id > %s AND message = 'Face %s is up';" "$1" "$2"
}

ask_and_detect() {
    local query="$1" title="$2"
    shift 2

    # **Asked only when there is something to ask for.** The query is the whole of what this waits on, so a query that
    # already answers describes a cube that is already where it needs to be -- and putting a banner this size in front
    # of somebody for a turn they do not have to make is how a prompt that is meant to stop them becomes one they
    # learn to look past. `00-setup` is the case it was written for: the run starts wherever the cube was left, and
    # much of the time that is already the face it wants.
    #
    # **Safe for the callers that want a change**, because they scope their query to rows newer than a mark taken a
    # line earlier, so nothing can already answer it. The one that does not scope is the one that means "be in this
    # state", and that is exactly the caller this is for.
    local already
    already=$(anysql "$query")
    if [ -n "$already" ]; then
        step "already there, so nothing to ask: $already"
        return 0
    fi

    echo ""
    yellow "##############################################################################"
    yellow "##"
    yellow "##  OVER TO YOU -- THE CUBE NEEDS TURNING"
    yellow "##"
    yellow "##  $title"
    yellow "##"
    local line
    for line in "$@"; do
        yellow "##    $line"
    done
    yellow "##"
    yellow "##  Nothing to press: this is watching the database and carries on by itself."
    yellow "##"
    yellow "##############################################################################"
    echo ""

    if [ ! -r /dev/tty ]; then
        step "no terminal to ask, so this is being skipped"
        return 1
    fi

    local waited=0 found="" asked=$SECONDS
    while true; do
        found=$(anysql "$query")
        if [ -n "$found" ]; then
            # **The whole of this wait is a person's**, not the app's: nothing here happens until somebody turns
            # the cube, and the poll is only how the script notices that they have.
            note_human_wait "$asked"
            step "seen: $found"
            return 0
        fi
        sleep 1
        waited=$((waited + 1))
        [ $((waited % 30)) -eq 0 ] && yellow "  still waiting ($((waited))s) -- $title"
    done
}


# Asked once a run: is the TimeFlip here? Answers 0 for yes, 1 for anything else, and every script after
# the first gets that answer back without asking again.
#
#     device_required || { fail "no TimeFlip was made available"; finish; exit $?; }
#
# **Several scripts need the cube and one person is answering for all of them.** Asking each time treats
# them as separate questions when they are one, and the repetition is what makes it worse than useless: a
# prompt drawn this big is meant to stop somebody, and the third identical one in two minutes is the one
# they answer without reading. Nothing between them changed the answer.
#
# **This is the archive's shape, massaged.** The previous suite's `00-test-setup` established the device once
# for a whole run and every feature checklist after it simply assumed the result -- neither
# `02b-reset-device` nor `09b-device-rename` asks whether the cube is there. What does not carry over is
# how: that was a setup checklist of its own, driven by a Python supervisor holding state between steps.
# There is no supervisor here, so the gate lives where the need does and the first script to want the cube
# is the one that asks.
#
# **A no is remembered too**, which is half of it: declining because the cube is in another room should
# skip the device scripts, not ask again in thirty seconds whether it has come back. It is also what makes
# this safe to be the only prompt: one no covers the wipe as well, since the reset script asks nothing of
# its own and simply does not run.
#
# **Kept in a file, because each script is its own process.** run.sh runs them as `bash "$script"`, so a
# variable set in one is gone before the next starts and an export only ever travels downwards.
# `TESTLOG_RUN_ID` is the run's own identity and reaches every script -- exported by run.sh, or made by
# `testlog_script_start` for a script run on its own -- so a file naming the run it answered for is
# set-once-per-run with nothing to tidy up: the next run reads an id that is not its own and asks.
DEVICE_GATE="logs/device-gate"

# **The gate is read here and asked for in `00-setup`.** One prompt, at the top of the run, before anything has
# been driven -- rather than at whichever device script happened to run first. Two things follow. Somebody who says
# no is told straight away instead of twenty minutes in, and every script below this can simply read the answer:
# there is no path here that stops to ask, so no script can accidentally become the one that prompts.
#
# `50-device-scan` is the only caller that acts on a no, and it stops the run. Everything after it needs a cube, so
# there is nothing left to run and no reason for each script to re-check.
device_required() {
    local run="${TESTLOG_RUN_ID:-}" answered="" remembered=""

    # **No identity, no memory.** Everything `testlog.sh` does is best-effort and may answer nothing at all, and
    # a run with no id must then ask rather than fall back to a shared one: two runs both calling themselves `0`
    # would have the second inherit the first's answer, about a cube that left the room hours ago.
    if [ -n "$run" ] && [ -r "$DEVICE_GATE" ]; then
        read -r answered remembered < "$DEVICE_GATE" || true
    fi
    [ -n "$run" ] && [ "$answered" = "$run" ] && [ "$remembered" = "yes" ]
}

# Asks, once, and writes the answer against this run's id. **Only `00-setup` calls this.** It is separate from
# `device_required` so that reading the answer cannot turn into asking for it: a script that reaches a prompt
# nobody is watching hangs the run, and the whole point of asking at the top is that somebody is still there.
ask_about_the_device() {
    local run="${TESTLOG_RUN_ID:-}"
    mkdir -p "$(dirname "$DEVICE_GATE")" 2>/dev/null || true

    # **Everything the run does to the cube, said once, because this is the only time it is asked.** That
    # includes the two things somebody agreeing to "is your device nearby" has plainly not agreed to on the
    # face of it: the PIN is changed, and the cube is wiped. Both are stated in full here rather than asked
    # about again later, so this prompt has to be worth reading -- it is the whole of the consent.
    if action_required \
        "May this run use your TimeFlip? It will be RESET to factory settings." \
        "THIS WIPES THE CUBE. The setup below erases everything stored on the device --" \
        "face colours, task settings, its name and its PIN -- back to factory defaults," \
        "and that cannot be undone. The cube comes back on the vendor PIN 000000, which is" \
        "the first one Facet presents, so pairing it again afterwards is one press of Scan," \
        "but anything you had set on the device itself is gone." \
        "" \
        "**It is wiped now, at the start, and again by 52-device-reset.** This run begins by" \
        "pairing the cube and resetting it, so every script after this one starts from a" \
        "factory cube the app has never seen." \
        "" \
        "The runs also change its PIN. They present 000000, and a cube answering to it is put" \
        "on six random digits kept in the login Keychain." \
        "" \
        "Then:" \
        "1. Flip the cube onto any face -- a sleeping cube does not advertise, so it cannot be found." \
        "2. Check Bluetooth is on." \
        "3. Press y and leave everything alone; the rest runs by itself." \
        "" \
        "Asked once for the whole run, here at the top. Every script that needs the cube takes" \
        "this answer, so leave it where it is until the run finishes." \
        "" \
        "The FIRST time Facet ever scans, macOS asks whether it may use Bluetooth. That" \
        "is once, not once per run or per build: after it is allowed the app just scans." \
        "If the prompt does appear, allow it -- until you do the radio never answers." \
        "" \
        "Answer anything else and the run stops at 50-device-scan. The scripts before it" \
        "still run, and CI will refuse the branch until somebody with a cube runs it."; then
        [ -n "$run" ] && printf '%s yes\n' "$run" > "$DEVICE_GATE"
        return 0
    fi

    [ -n "$run" ] && printf '%s no\n' "$run" > "$DEVICE_GATE"
    return 1
}

# Pairs a cube from scratch, with the Device tab already on show. Answers 0 once the app says `Paired with`.
#
# **Three scripts needed this and had two copies of it**, which is the point at which it stops being repetition and
# starts being a place for them to drift apart. `51-device-connect` keeps its own, deliberately: that one is the
# script whose subject *is* connecting, and every step of it is a check rather than a means to an end.
#
# **From scratch, forgetting first**, which is now only for the scripts whose subject is a pairing. `00-setup` pairs a
# cube to wipe it, `51-device-connect` makes the pairing the rest of the run uses, and `52`, `53`, `54`, `55` and `56`
# reach this through `restore_the_pairing` at the *end*, putting back the one they gave up. Nothing calls it to arrange
# a cube it merely needs -- see `require_a_paired_cube`.
#
# **The caller decides what a failure means**, because it differs: an unusable radio says nothing about the app, and a
# cube that would not answer is the bench having stopped working. So this says what happened in the log and answers
# with which of the two it was:
#
#   0  paired
#   2  the radio cannot be used, which says nothing about the app -- `PAIR_REASON` holds the app's own words
#   1  everything else: nothing answered the scan, no row to press, or the PIN was refused
pair_a_cube() {
    PAIR_REASON=""
    PAIR_STATUS=0

    # **The Scan button has to be on screen, and this is checked rather than assumed.** `press` swallows everything --
    # its output, its exit code, and the case where the element is not in the tree at all -- so pressing a button that
    # is not there does nothing and says nothing. The wait below then times out after a full minute and reports the
    # radio as the culprit, which is a diagnosis pointing away from the fault: on 2026-08-22 `57-cube-pause` reached
    # here without ever having opened the Settings window, sat for 60 seconds, and skipped itself blaming a
    # permission prompt that was not there.
    if [ -z "$(element device-scan)" ] && [ -z "$(element device-forget)" ]; then
        PAIR_REASON="neither Scan nor Forget is on screen -- open the Settings window on the Device tab first"
        PAIR_STATUS=1
        return 1
    fi

    if [ -n "$(element device-forget)" ]; then
        step "already paired; forgetting first so this pairs its own cube"
        press device-forget
        sleep 1
    fi

    # **A scan already listening is waited out, not pressed into.** The Scan button is the one button either way:
    # pressing it during a scan stops that scan instead of starting one, so the wait below then sits out its whole
    # minute for a `Scan started` row nothing is going to write, and answers with the radio. That is a diagnosis
    # pointing away from the fault for the second time in this function -- run 170 (2026-09-07) stopped here, because
    # `56-manual-mode` runs a scan to prove the radio came back and the window it leaves behind outlasted the sleep
    # that was meant to cover it.
    # **A scan already listening is waited out, not pressed into.** The Scan button is the one button either way:
    # pressing it during a scan stops that scan instead of starting one, so the wait below then sits out its whole
    # minute for a `Scan started` row nothing is going to write, and answers with the radio. That is a diagnosis
    # pointing away from the fault for the second time in this function -- run 170 (2026-09-07) stopped here, because
    # `56-manual-mode` runs a scan to prove the radio came back and the window it leaves behind outlasted the sleep
    # that was meant to cover it.
    case "$(element device-scan)" in
        *"Stop Scan"*)
            step "a scan is already running; waiting for it to end before starting one"
            if ! wait_for "$(mark)" "%Scan timed out%" 25 >/dev/null; then
                PAIR_REASON="a scan was already running and had not ended 25s later, so this could not start one of its own"
                PAIR_STATUS=1
                return 1
            fi
            ;;
    esac

    local since row
    since=$(mark)
    press device-scan
    sleep 0.5

    step "waiting for the radio to come up..."
    if ! wait_for "$since" "%Scan started%" 60 >/dev/null; then
        PAIR_REASON=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Scan unavailable:%' ORDER BY debug_log_id DESC LIMIT 1;")
        [ -n "$PAIR_REASON" ] && { PAIR_STATUS=2; return 2; }
        PAIR_REASON="the radio never answered in 60s -- is the macOS Bluetooth permission prompt waiting?"
        PAIR_STATUS=1
        return 1
    fi

    step "listening for advertisements..."
    # Bounded just past the scan's own window (`BluetoothRadio.timeoutSeconds`, fifteen seconds), for the reason
    # `50-device-scan` carries: waiting less than the radio listens calls a scan empty while it is still running, and
    # waiting longer waits for something nothing is looking for any more.
    if ! wait_for "$since" "%: peripheral %" 18 >/dev/null; then
        # The scan is stopped on the way out: leaving the radio listening behind a script that has given up is what
        # the timeout exists to prevent, and this path is reached before it fires.
        press device-scan
        PAIR_REASON="the scan ran its full 15 seconds and no TimeFlip answered it -- is the cube awake?"
        PAIR_STATUS=1
        return 1
    fi

    row=$(tree | grep -m1 -o "device-scan-result-[0-9A-Za-z:-]*")
    if [ -z "$row" ]; then
        press device-scan
        PAIR_REASON="the app logged a device but drew no row to press"
        PAIR_STATUS=1
        return 1
    fi

    since=$(mark)
    press "$row"
    step "pairing..."
    if ! wait_for "$since" "Paired with %" 60 >/dev/null; then
        PAIR_REASON="the cube was found but would not pair -- it may be on a PIN this app cannot present"
        PAIR_STATUS=1
        return 1
    fi

    # **A pairing is enough to make the app follow the cube again** (2026-08-29): timing by hand is read from `paired`
    # at the point of use, so writing that row is the whole of it and nothing has to be restarted.
    #
    # **The relaunch below is kept for the link, not for the mode.** What the scripts after this one inherit is a cube
    # the app has actually reached -- a login, a face, a charge and a status -- and the relink is what guarantees one
    # rather than leaving them to a connection that may or may not still be up. It also puts the lock and the pause a
    # quit applies back where they were.
    #
    # **What it looks like when this is missing** is not a pairing failure, which is why it is worth the words: the
    # cube pairs, connects, and answers, and the Faces tab draws the *manual* session instead of the cube's face. Run
    # 84 (2026-08-23) failed on `55-device-face` reading `Limit 3` off manual face 14 where it wanted `Meeting` off
    # cube face 2, with every device script before it green.
    #
    # Restarting is exactly what the app tells a user to do, and doing it here rather than in every caller keeps the
    # meaning of "pair a cube" whole: pair it, and be a launch that uses it.
    if ! relink_a_cube; then
        PAIR_REASON="paired, but the launch restarted to use it did not reach the cube again within 90s"
        PAIR_STATUS=1
        return 1
    fi
    # Put the window back where the caller left it. Every one of them opens Settings on the Device tab before calling
    # this and carries on using it afterwards.
    open_settings
    select_tab Device
    return 0
}

# **Takes the link down and lets the app bring it back up, without touching the pairing.**
#
# For the three scripts whose subject is what happens *as* a link comes up: the charge pulled on connecting (`54`),
# the face read as the link opens (`55`), the clock set and the characteristics found after the login (`57`). None of
# that can be asserted against a connection that is already up, because those rows are older than any mark the script
# can take -- and none of it needs a new pairing, which is what calling `pair_a_cube` for it would cost.
#
# **A quit and a launch, which is the app's own reconnect.** `quit_app` lets the cube go and the launch after it
# reaches for the cube on record, running the same login as a fresh pairing does: `DeviceLogin` is what asks for the
# charge, the face and the state, and it runs on every connect rather than only on a first one. `53-device-reconnect`
# is the script that proves that path, so everything relying on it here is already covered.
#
# **It is also how the run gets a link rather than only a pairing.** Following the cube needs no relaunch since
# 2026-08-29 -- the app reads `paired` when it is asked, so it follows one the moment it is paired -- but what the
# range wants is a cube actually reached, with a face, a charge and a status behind it. `51-device-connect` ends with
# this for that reason, and hands the rest of the range a launch that has already talked to the pairing it made.
#
# **What it leaves behind: an unlocked, running cube, and it has to undo a quit to get there.** The app pauses and
# locks the cube on its way out ("Quit: the cube is paused and locked"), so the launch this makes inherits one -- and
# a cube that arrives stopped is a cube every script after it has to cope with. Coping is what the branches were:
# `55`, `57`, `60` and `62` each carried an `if` asking what state the hardware had been left in, which is a check
# that may or may not run and, on run 117, one that did not.
#
# So it is undone here, once, and every script downstream is entitled to a cube that is unlocked and counting. **Not a
# guess about the state**: a quit always locks and always pauses, so the toggle below is aimed at a known state rather
# than at whatever it finds. `57` locks the cube itself, that being its subject; nothing else has to think about it.
#
# **The window is the caller's business, not this function's**: `51` ends with it shut and `57` never opens one, while
# `pair_a_cube` and the two scripts that read the Device tab afterwards put it back themselves.
relink_a_cube() {
    quit_app
    sleep 1
    local relaunched
    relaunched=$(mark)
    ensure_app_running
    # Waited for rather than assumed: every caller goes straight on to something that needs the link up.
    wait_for "$relaunched" "%: loggedIn" 90 >/dev/null || return 1

    free_the_cube || return 1
    return 0
}

# Takes off the lock and the pause a quit put on, so what follows inherits a cube that is counting.
#
# **Aimed at a known state rather than at whatever it finds.** The app pauses and locks the cube on its way out
# ("Quit: the cube is paused and locked"), always and both, so the dropdown's Lock item -- which unlocks and resumes in
# one gesture (`CubeLock.resume`) -- undoes exactly the pair that was applied. Nothing is asked first, because there is
# nothing to ask.
#
# **Every quit has to come through here, and that is the whole of the invariant.** `relink_a_cube` calls it, so
# everything reached through `pair_a_cube` or `restore_the_pairing` is covered; `58-wrong-pin` quits on its own terms
# and calls it directly. Run 119 (2026-08-27) is what one uncovered quit costs: `58` left the cube locked and paused,
# `59` had no reason to care, and `60-device-backlog` took its cube out of range already stopped -- so the figure it
# expects to go on counting stood still, and the failure read as the app having stopped following a cube.
free_the_cube() {
    local freeing
    freeing=$(mark)
    # **The app has to have asked the cube what state it is in before that menu item means anything.**
    # `DeviceLogin.askWhatStateItIsIn` is what makes the Lock line honest, and its own comment says why: a freshly
    # connected app would otherwise have to guess which way the item reads, and the guess it makes -- unlocked -- is
    # exactly wrong for the cube this app locked on the way out last time. The answer is a `0x10` round trip and
    # lands a second or so after the login.
    #
    # **Without this wait the item is pressed while it still says Lock, and the cube gets locked instead of freed.**
    # Measured on Linux 2026-09-20: `Asking the cube what state it is in` at :02, `Menu item clicked: Lock` at :03,
    # and `The cube is locked and paused` at :03 -- the answer arriving after the press it should have decided. The
    # race is not Linux-specific; that platform is simply where it was lost first.
    wait_for "$freeing" "The cube is %locked and %" 25 >/dev/null || {
        red "  the app never read the cube state, so the Lock item cannot be trusted either way"
        return 1
    }
    click_left
    sleep 0.8
    # **Through the menu port**, because this is a status-item item and not a window control. `press` searches the
    # accessibility tree, which on Linux the tray is not in at all -- it is a D-Bus object -- so this reported
    # `nothing in the tree matches name toggle-cube-lock` while the item sat in the menu (2026-09-20). The six
    # presses in the check scripts were converted when `menu_press` was added and this one, being inside a helper,
    # was missed.
    menu_press toggle-cube-lock
    wait_for "$freeing" "The cube is unlocked" 20 >/dev/null || return 1
    wait_for "$freeing" "The cube is running" 20 >/dev/null || return 1
    # **And then on the table, which is a second thing rather than the same one said twice.** The rows above say the
    # cube confirmed the commands; what the callers of this actually inherit is the app's record of the cube, and that
    # is written by the history fetch the resume sets off -- a couple of hundred milliseconds later, because a resumed
    # cube files a fresh event and the app has to go and read it. Until it lands, the newest open segment is still the
    # paused one this just undid.
    #
    # **Measured on 2026-08-28.** `57-cube-pause` asserts exactly this query at the top, failed on it with `1`, and the
    # row that would have answered `0` was written 300ms after the check ran. The race has always been here and was
    # simply always won; twelve face colours now go out on every connect, and the resume confirming between them moved
    # the fetch late enough to lose it. The same query as the assertion on purpose, so what this promises and what
    # that checks cannot come to differ.
    if ! wait_sql "0" \
        "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY device_event_id DESC LIMIT 1;" \
        15 >/dev/null
    then
        red "  the cube was unlocked and resumed, but device_event still shows it stopped"
        return 1
    fi
    step "unlocked and counting again, which is what every script after this one is entitled to"
    return 0
}

# ---------------------------------------------------------------------------- one pairing, for the whole device range
#
# **The device scripts run on the cube `51-device-connect` pairs, and every one of them leaves a cube behind.** A
# script that needs a connection inherits the live one rather than forgetting it and pairing again, and a script whose
# subject *is* giving a cube up -- the reset in `52`, the checked forgets in `53`, `54`, `55` and `56` -- calls
# `restore_the_pairing` before it finishes so the next one still has what it expects.
#
# **This is the suite being read as a sequence, which is what it is.** Each script already starts from the state the
# one above it left; pairing defensively at the top was the one place that was not true, and it cost a forget, a
# ten-second scan, a pairing and a relaunch in each of eight scripts to arrive at a cube that was already sitting
# there connected. What it bought was a single script surviving being run on its own, which is not a promise this
# folder makes about anything else it depends on: `09-report` needs the entries `06` records, and nothing in it
# arranges them.
#
# **So running one on its own is the caller's problem**, and `--keep` is how it is done. What this does instead is
# make the failure immediate and say which state is missing, because the alternative is silent and late: with
# nothing paired the Reset and Forget buttons are simply not on the tab, `press` says nothing about an element that
# is not there, and the first wait after it times out sixty seconds later blaming the radio.
#
# **Not a check.** A precondition is a statement about the bench, not a verdict on the app, and counting it would put
# the state the hardware was left in into `EXPECTED_CHECKS` (`57-cube-pause` records the run that was refused for
# exactly that). It stops the run instead.
require_a_paired_cube() {
    local paired connected
    paired=$(setting paired paired)
    if [ "$paired" = "1" ]; then
        # **A launch that has just started is still on its way to the cube, and that is not a bench fault.** Reaching
        # a paired cube again is a scan, a connect and a login; `ensure_app_running` waits for the *process* to appear
        # and not for the radio, so a script run on its own reads this a second later and finds it false.
        #
        # Run 115 (2026-08-27): `run.sh --keep 62` refused outright on a cube that was paired, awake, and about forty
        # seconds from connected, because the run before it had quit the app on its way out. In the suite this costs
        # nothing -- the script above has left the link up, so the first poll answers -- and 90s is the same budget
        # `relink_a_cube` gives the same wait.
        connected=$(wait_sql "1" "SELECT json_extract(setting_value, '\$.connected') FROM setting WHERE setting_name = 'connection';" 90)
    fi
    [ "$paired" = "1" ] && [ "$connected" = "1" ] && return 0

    red "  no cube is connected, so $1"
    if [ "$paired" = "1" ]; then
        red "  a cube is paired but the app did not reach it in 90s -- is it awake, and is Bluetooth on?"
    else
        red "  the device scripts run on the cube 51-device-connect pairs, and each one leaves it behind."
        red "  Run the suite in order, or pair a cube on the Device tab before running this one on its own."
    fi
    finish
    exit 2
}

# **Puts back the pairing this script gave up**, for the next script in the range, which inherits rather than arranges.
#
# **Not a check either**, for `require_a_paired_cube`'s reason: whether a cube can be paired again is not the subject
# of any script that calls this, and every one of them has already asserted the giving-up that made it necessary.
#
# **But never a skip, which is the part that cost a run.** `00-setup` has already confirmed a TimeFlip is here, so from
# that point on "no cube could be paired" is a bench that has stopped working, not an answer. Passed over quietly it
# read as green: on 2026-08-22 `55-device-face` tested nothing at all because a busy database dropped one write of
# `recordPairing`, and the run finished `outcome: passed`. So this stops the run and exits non-zero, because
# everything after it would otherwise fail at a cube that is not there and report it as the app being wrong.
restore_the_pairing() {
    if pair_a_cube; then
        step "the cube is paired again, which is what the next script starts from"
        return 0
    fi
    red "  the pairing this script gave up could not be put back: $PAIR_REASON"
    red "  every script after this one needs it, so the run stops here rather than failing at a cube that is gone"
    finish
    exit 1
}

start() {
    echo ""
    blue "=============================================================================="
    blue "$SCRIPT_NAME: $*"
    blue "=============================================================================="
    testlog_script_start "$SCRIPT_NAME" "$*" "${EXPECTED_CHECKS:-0}"
}

# **What is being checked is printed before the verdict, on its own line.**
#
#     a segment is open going into the quit (id 113)
#       PASS
#
# Two lines rather than one because a run is watched as well as read afterwards: several checks here wait
# on a network round trip or a ten-second timer, and a single line printed on the way out leaves the
# terminal silent for as long as the slowest step takes, with nothing saying which step it is. The
# description goes out first, and the verdict lands under it when the answer is known.
#
# `LAST` is what the description was, so the summary at the end can name a failure without every caller
# repeating itself.
LAST=""
LAST_STARTED=0
LAST_LOG_MARK=0
LAST_EXPECTED=""
LAST_ACTUAL=""
announce() {
    LAST="$*"
    # Stamped here rather than at the verdict, so the log records how long the check actually waited. The
    # slow ones are the ones worth knowing about: a check that takes 45s is usually timing out.
    LAST_STARTED=$(date +%s)
    LAST_LOG_MARK=$(app_log_mark)
    LAST_EXPECTED=""
    LAST_ACTUAL=""
    printf '  %s\n' "$*"
}

verdict_pass() { PASSED=$((PASSED + 1)); green "    PASS"; testlog_check pass; }

# **A failed check stops its script there and then.** Every check starts from the state the ones above it
# left, so once one has failed the rest are being run against a state nobody intended and their verdicts
# mean nothing. Carrying on produced exactly that on 2026-08-16: a rename that never happened was
# reported as one failure, and the check that read the name back was reported as a second, as though two
# things were wrong.
#
# The cost is real and worth knowing: independent failures further down are not found until the next run.
# That same run had a daily limit fault with nothing to do with the rename, and stopping at the first
# would have hidden it. `--keep-going` is there for the pass where finding everything matters more than
# reading it cleanly.
FAIL_FAST="${FAIL_FAST:-1}"
verdict_fail() {
    FAILED=$((FAILED + 1))
    FAILURES="$FAILURES
  - $LAST${1:+ ($1)}"
    red "    FAIL${1:+  $1}"
    testlog_check fail "${1:-}"

    if [ "$FAIL_FAST" = "1" ]; then
        red "    stopping here: everything below starts from the state this check just failed to reach"
        grey "    (Tests/Scripted/run.sh --keep-going runs the rest anyway)"
        finish
        exit 1
    fi
}

# `pass "what was checked"` and `fail "what was checked"` for the cases a script decides for itself.
pass() { [ $# -gt 0 ] && announce "$*"; verdict_pass; }
fail() { [ $# -gt 0 ] && announce "$*"; verdict_fail; }

# One setting's value, by row name and JSON key: `setting paired paired`, `setting connection connected`.
#
# **Here rather than in each script that wants it.** Three scripts each defined this privately -- 08, 53 and 56,
# byte for byte identical -- and `58-wrong-pin` was written against it without carrying a fourth copy. Every script
# is its own `bash` process, so 58 had no such function: its first check read an empty string, failed, and stopped
# the whole run on runs 71 and 77. A helper that several scripts each redefine is a helper the next script will be
# missing, and the failure looks like the app being wrong rather than the harness.
setting() { sql "SELECT json_extract(setting_value, '\$.$2') FROM setting WHERE setting_name = '$1';"; }

# ---------------------------------------------------------------------------- arranging, as opposed to checking
#
# **For a script that sets things up rather than testing them, where the whole script is one verdict.** `00-setup`
# is the only one, and it answers once at the bottom: `step` (with the output helpers above) narrates what it did,
# `trouble` records what it could not do, and neither writes a `check_result` row.
#
# **Defined here rather than in the script**, because `seed-private.sh` needs them too and a second copy is how
# `58-wrong-pin` came to call a `setting` helper that three other scripts each defined privately -- it ran zero
# checks and stopped run 71. One definition, both callers.
#
# Why not just use `check` for each seed: a setup step that passes says nothing about the app, so counting twenty
# of them puts twenty green lines at the top of a run and inflates the total into coverage that does not exist.
# And a seed that fails must not stop at the first one -- somebody about to leave the room for twenty minutes
# needs every reason the ground is wrong, not the earliest.
TROUBLE=""
trouble() {
    TROUBLE="$TROUBLE
      - $*"
    red "  could not: $*"
}

# `check "name" "expected" "actual"` -- the common shape, so a failure prints both sides without every
# script formatting its own message.
check() {
    announce "$1"
    LAST_EXPECTED="$2"
    LAST_ACTUAL="$3"
    if [ "$2" = "$3" ]; then
        verdict_pass
    else
        verdict_fail "expected '$2', got '$3'"
    fi
}

check_contains() {
    local name="$1" haystack="$2" needle="$3"
    announce "$name"
    case "$haystack" in
        *"$needle"*) verdict_pass ;;
        *)
            # Truncated: the haystack is often the whole accessibility tree, and printing it buries the
            # failure it is supposed to explain.
            local shown="${haystack:0:160}"
            [ "${#haystack}" -gt 160 ] && shown="$shown..."
            verdict_fail "no '$needle' in '$shown'"
            ;;
    esac
}

# Neither passed nor failed: the thing could not be checked here, and saying so is more honest than a
# tick. A skipped check does not fail the script -- an account nobody has connected is not a defect --
# but it is printed loudly enough that nobody reads the run as fuller coverage than it was.
# **There is no `skip`, deliberately, and calling one is meant to be an error.** A skip is a check reporting that it
# could not answer, and a run full of them reads green while proving nothing: on 2026-08-22 `55-device-face` skipped
# its entire self and the run still stamped `outcome: passed`. So a missing cube, an unusable radio, an unconnected
# Google account and a prompt nobody answered are failures -- they say what is needed, and the run stays red until it
# is there. See `README.md`, under the order.
#
# The counter and its plumbing stay, in `finish`, the stamp and `scripts/check_interactive_checklists.sh`. They are
# now a guard rather than a feature: if anything ever manages to record a skip again, every one of them still refuses
# the run, and the number is where it will show.

# Ends the script and decides its exit status. Non-zero on any failure, which is what lets run.sh stop
# rather than carry on into scripts whose starting state the failure just invalidated.
FINISHED=0
finish() {
    # Called twice where a script does `fail ...; finish; exit 1` and the failure already stopped it. The
    # second call must report the same verdict and not write the record again.
    if [ "$FINISHED" = "1" ]; then
        [ "$FAILED" -eq 0 ]
        return
    fi
    FINISHED=1

    # Before anything is printed, because this is where the app's own debug_log rows are copied out of
    # test.sqlite -- and a script that exits straight after `finish` would otherwise take them with it.
    testlog_script_finish "$PASSED" "$FAILED"
    echo ""
    if [ "$FAILED" -eq 0 ]; then
        green "$SCRIPT_NAME: $PASSED passed, 0 failed"
    else
        red "$SCRIPT_NAME: $PASSED passed, $FAILED FAILED$FAILURES"
    fi

    # **A script that ran the wrong number of checks has not passed, however green every one of them was.**
    # `EXPECTED_CHECKS` is what the script says it does; `PASSED` is what it did. They part company when a
    # branch nobody meant to take was taken -- a conditional that silently skipped a section, an early exit,
    # a helper that returned before its checks -- and every check that never ran reports nothing at all. That
    # is the failure this exists to catch, because it is the one that looks exactly like success.
    #
    # Reported here and enforced in CI off the stamp, rather than turned into a failed check: a check would
    # need a description of its own and would itself be counted, which is a number that changes the number
    # it is checking.
    if [ "${EXPECTED_CHECKS:-0}" -gt 0 ] && [ "$PASSED" != "$EXPECTED_CHECKS" ]; then
        red "$SCRIPT_NAME: expected $EXPECTED_CHECKS passing check(s), got $PASSED"
        # **The two directions mean opposite things**, so the line says which happened rather than leaving it to
        # be worked out from two numbers. Fewer is the case this exists to catch: a branch nobody meant to take,
        # and every check on the other side of it silently not running. More is the count being out of date, or a
        # helper quietly adding verdicts of its own -- which is what `apply_private_seeds` was doing to 00-setup
        # on run 73, two checks that belonged to the seeding rather than to the app.
        if [ "$PASSED" -lt "$EXPECTED_CHECKS" ]; then
            red "    $(( EXPECTED_CHECKS - PASSED )) check(s) never ran -- a branch was taken that skips them"
        else
            red "    $(( PASSED - EXPECTED_CHECKS )) more check(s) ran than this script says it has"
        fi
        return 1
    fi
    [ "$FAILED" -eq 0 ]
}

# ---------------------------------------------------------------------------- the database

# Refuses production, and says how to switch. Every script calls this before it touches anything.
require_test_database() {
    if [ ! -e "$DB" ]; then
        red "No database at $DB. Run scripts/switch-database.sh test first."
        exit 2
    fi
    local target
    target="$(readlink "$DB" || echo "$DB")"
    case "$target" in
        *test.sqlite) step "database: test.sqlite" ;;
        *)
            red "REFUSING TO RUN: appdata.sqlite points at '$target', not test.sqlite."
            red "These scripts write real rows and nothing undoes them."
            red "Switch with: scripts/switch-database.sh test"
            exit 2
            ;;
    esac
}

# **Waits for a writer rather than failing at one.** The app's database is `journal_mode=delete`, so a
# write locks the file against readers outright -- and every check here reads `debug_log` at exactly the
# moment the app is busiest writing it. Without a timeout, sqlite gives up instantly: the read prints
# `Error: in prepare, database is locked (5)` to stderr, returns **empty on stdout**, and the check then
# fails against a haystack of `''` as though the row were missing.
#
# That is not hypothetical and it is not rare. On 2026-08-17 run 28 it failed
# `the accepted answer is 0x02` while the row it wanted was sitting in the table: the login writes
# sixteen rows in 440ms (the PIN rotation, then the Device Information reads), and the check lands in
# the middle of them. Ten seconds is far longer than any burst the app produces, so this waits rather
# than races, and a genuine empty result still means what it says.
sql() { sqlite3 -cmd ".timeout 10000" "$DB" "$1"; }

# The same, against the trace's database. **Every query naming `debug_log` goes through this**, and the split is worth
# saying plainly because it used to be one file: a `sql` call asking for `debug_log` now answers nothing at all rather
# than failing loudly, since sqlite reports "no such table" on stderr and an empty result on stdout -- which reads
# exactly like a row that has not been written yet.
#
# The timeout is the same 10 seconds and for a milder version of the same reason. The app writes this file constantly
# while a run polls it, but nothing else does, so the contention that made the app's own timeout necessary is largely
# what moving the table removed.
dsql() { sqlite3 -cmd ".timeout 10000" "$DEBUG_DB" "$1"; }

# Runs a query a *caller* supplied, against whichever database it is asking about.
#
# **For the three helpers that take a whole query rather than a pattern** -- `ask_and_detect`, `wait_sql` and
# `wait_for_value`. Every other call site names its table when it is written, so it can pick `sql` or `dsql` then;
# these cannot, because the table is in the argument. Deciding from the text keeps the caller writing one query and
# not also having to say where it lives.
#
# A query joining `debug_log` to an app table would be answered wrongly rather than refused, and there is no way to
# answer it at all -- they are separate files. There are none today (checked when the log moved out, 2026-08-22), and
# a check that needs both should wait for the row and then read the table, which is what `wait_sql` already exists to
# do.
anysql() {
    case "$1" in
        *debug_log*) dsql "$1" ;;
        *)           sql "$1" ;;
    esac
}

# The next unused name in a numbered family: `next_name Timer` answers `Timer 1` on a database built from
# nothing, and `Timer 18` on one that already holds seventeen.
#
# **Because an active category name has to be unique.** These scripts create categories and delete none,
# so a fixed name works once and collides on every `--keep` run afterwards -- the app would raise its
# "already exists" alert and the create would be refused, failing a check that has nothing to do with what
# is being tested. The names used to carry a clock stamp for that reason, which worked and made every
# category unreadable (`Scripted 09:14:22`) and every failure message different from the last.
#
# The number is a high-water mark, not a count. It reads the largest number already used and goes one
# past, so a gap left by a deleted row is simply never reused -- which is what makes it safe to derive
# other names from it (`$NAME renamed`, `$NAME reactivate`) without checking each one.
next_name() {
    local prefix="$1" highest
    highest=$(sql "SELECT IFNULL(MAX(CAST(SUBSTR(category_name, LENGTH('$prefix') + 2) AS INTEGER)), 0)
                     FROM category
                    WHERE category_name GLOB '$prefix [0-9]*';")
    printf '%s %s' "$prefix" "$(( ${highest:-0} + 1 ))"
}

# The newest debug_log id right now. Every wait is measured from one of these -- see rule 3 above.
# **Answers 0 rather than nothing when the query cannot run at all**, which is not the same as the `IFNULL` above.
# That handles an empty table; this handles no table, no file, or a locked one -- sqlite3 prints its error to stderr
# and nothing to stdout, and an empty mark then goes straight into the next query as `debug_log_id > AND ...`. On
# 2026-08-22 that turned one missing table into forty "near AND: syntax error" lines and a run that measured every
# check from a baseline it never had.
mark() {
    local id
    id=$(dsql "SELECT IFNULL(MAX(debug_log_id), 0) FROM debug_log;")
    printf '%s' "${id:-0}"
}

# Waits for a debug_log row after `since` whose message matches `pattern` (a SQL LIKE, so % is the
# wildcard). Prints the message it found, empty on timeout.
#
# Polling the table rather than sleeping a fixed time: the app writes the row when the thing actually
# happened, so this is as fast as the app is and still correct on a slow machine.
wait_for() {
    local since="$1" pattern="$2" timeout="${3:-15}"
    # **An apostrophe in the pattern is doubled, because the pattern goes inside a SQL string literal.** `The cube's
    # clock is set` closes the quote at `cube` and the rest becomes syntax, so sqlite3 refuses the query, prints
    # nothing to stdout, and this polls an empty answer for its whole timeout before reporting that the app never
    # wrote a row it wrote immediately. Doubling is SQL's own escape and is what sqlite3 expects.
    #
    # Measured 2026-08-22: `57-cube-pause` failed on "no debug_log row matching 'Setting the cube's clock to %' within
    # 30s" with that exact row sitting in the table, 170ms after the mark. Three of its checks carried an apostrophe
    # and none of them could ever have matched. Escaped here rather than in each script, so the next pattern with one
    # in it simply works.
    #
    # **The error was never hidden, and that is the part worth remembering.** `sql` does not redirect stderr, so
    # `Error: in prepare, near "s": syntax error` went to the terminal and into `logs/screen.txt` -- three hundred
    # times, once per poll. What failed was not the diagnosis being unavailable but the verdict pointing away from it:
    # the FAIL line said the app had written no row. A loud signal beside a confident wrong summary is read as noise.
    pattern=${pattern//\'/\'\'}
    local waited=0 found=""
    while [ "$waited" -lt "$((timeout * 10))" ]; do
        found=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND message LIKE '$pattern' ORDER BY debug_log_id LIMIT 1;")
        [ -n "$found" ] && { printf '%s' "$found"; return 0; }
        sleep 0.1
        waited=$((waited + 1))
    done
    return 1
}

# Waits for a query to answer `expected`, and answers whatever it last saw. Polled like `wait_for`, and
# for the same reason: the app writes when the thing actually happened, so this is as fast as the app is
# and still correct on a slow machine.
#
#     wait_sql "0" "SELECT COUNT(*) FROM device_event WHERE finalised = 0;"
#
# **For the state behind a log row, which does not land with it.** A debug_log row and the write it
# describes are two separate statements, and the app writes the row *first* in at least one place that
# matters: `DailyLimitWatch` records "Daily limit reached" and then calls `stopTiming()` to close the
# segment. A check that waits for the row and then reads the table once is racing that gap, and on
# 2026-08-17 run 29 it lost -- `the open segment was closed` wanted 0 and got 1, on the third of three
# identical iterations, having passed the first two.
#
# So: wait for a row to know a thing began, and wait on the table to know it finished.
wait_sql() {
    local expected="$1" query="$2" timeout="${3:-10}"
    local waited=0 answer=""
    while [ "$waited" -lt "$((timeout * 10))" ]; do
        answer=$(anysql "$query")
        [ "$answer" = "$expected" ] && { printf '%s' "$answer"; return 0; }
        sleep 0.1
        waited=$((waited + 1))
    done
    printf '%s' "$answer"
    return 1
}

# `expect_log "name" "$since" "pattern"` -- the shape most checks take.
expect_log() {
    local name="$1" since="$2" pattern="$3" timeout="${4:-15}"
    local found
    # Announced before the wait, not after it. This is the one that can sit for a minute waiting on
    # Google, and a terminal that says nothing for a minute looks stuck rather than busy.
    announce "$name"
    if found=$(wait_for "$since" "$pattern" "$timeout"); then
        verdict_pass
        grey "          $found"
    else
        verdict_fail "no debug_log row matching '$pattern' within ${timeout}s"
    fi
}

# ---------------------------------------------------------------------------- the radio, and putting it back
#
# **Turning Bluetooth off is the only way to put a paired cube out of reach without carrying it out of the building**,
# and nothing on this machine may turn a radio off on somebody's behalf. Two scripts need it -- `56-manual-mode` and
# `60-device-backlog` -- so the machinery lives here rather than in whichever of them was written first.
#
# **Whatever happens, the radio has to be back on when the script ends.** Every way out -- a check failing, a prompt
# declined, somebody pressing Ctrl-C -- is a way out that would otherwise leave it off, and `99-quit` wipes the cube
# so this run's timings cannot reach production: an unwiped cube puts them in front of the next launch against the
# **real** database. Hence a trap rather than a call at each exit, the exits that matter being the ones nobody thought
# of.
BLUETOOTH_IS_OFF=0

# Whether the radio is on, asked of the system rather than remembered. `BLUETOOTH_IS_OFF` only says a script turned it
# off, which stops being true the moment somebody turns it back on -- so the way out asks, which is what stops a prompt
# appearing in front of somebody who has already done the thing it is asking for (2026-08-22: a check failed, and this
# asked for a radio that had been back on for a minute).
# **Through the port**, which is where the platform difference lives. This used to run
# `system_profiler SPBluetoothDataType` on both platforms -- a macOS tool -- so on Linux it printed nothing,
# fell through to its catch-all, and answered "off" for a radio that was on. `00-setup` then asked the owner
# to turn on Bluetooth they had already turned on, failed, and stopped the run (2026-09-20).
#
# **Only "on" is treated as on.** `platform_bluetooth_is_on` also answers *cannot tell*, and `require_bluetooth`
# below is where that is given different words; everything else asking this question wants a plain yes or no,
# and for those an unknown radio is not one it may proceed on.
bluetooth_is_on() {
    platform_bluetooth_is_on
    [ $? -eq 0 ]
}

restore_bluetooth() {
    [ "$BLUETOOTH_IS_OFF" = "1" ] || return 0
    BLUETOOTH_IS_OFF=0
    if bluetooth_is_on; then
        step "Bluetooth is back on already, so there is nothing to ask for"
        return 0
    fi
    echo ""
    yellow "##############################################################################"
    yellow "##"
    yellow "##  TURN BLUETOOTH BACK ON"
    yellow "##"
    yellow "##  This script turned it off and is ending with it still off."
    yellow "##  99-quit wipes the cube so this run's timings cannot reach production,"
    yellow "##  and it cannot do that with the radio off."
    yellow "##"
    yellow "##############################################################################"
    echo ""
    [ -r /dev/tty ] || return 0
    local asked=$SECONDS
    printf '  Press Return once Bluetooth is back on: '
    read -r _ < /dev/tty || true
    note_human_wait "$asked"
    echo ""
}

# Arms the trap. Called by a script before it asks for the radio to be turned off, not by `lib.sh` itself: a trap set
# for every script would be one set for the twenty-odd that never touch the radio.
watch_bluetooth() {
    trap restore_bluetooth EXIT INT TERM
}

# Makes sure the radio is on before a script that needs it starts, asking for it if it is not. Answers 0 once it is
# on, 1 when nobody turned it on or there was no terminal to ask.
#
# **The one state a run can begin in that no script can fix for itself, and that the suite creates.** `56` and `60`
# ask for the radio to be turned off, and both arm `restore_bluetooth` to ask for it back however they end -- which
# covers a failed check, a script exiting, and Ctrl-C. What it cannot cover is the run being killed outright or the
# terminal being closed, and then the radio is still off the next time somebody starts a run.
#
# **What that looked like without this.** The diagnosis was always right and never actionable: `pair_a_cube` failed
# and put `Scan unavailable: bluetoothOff` in a setup step nobody is being asked to act on, then `50-device-scan`
# stopped the run a minute later with the same words. Both correct, neither asking for the one thing that would fix
# it.
#
# **Asked of the system rather than of the app**, and before anything is pressed: a radio that is off leaves every
# button on the Device tab reading perfectly correctly, so there is nothing on screen to check and no row to wait
# for -- the app has not been asked to do anything yet.
require_bluetooth() {
    platform_bluetooth_is_on
    local radio=$?
    [ "$radio" -eq 0 ] && return 0

    # **Cannot tell is not off, and must not put a banner in front of somebody.** The probe says this when
    # the tool it asks is missing or there is no controller at all, and asking for a radio to be turned on in
    # either case is advice that cannot be followed. The app's own scan is the better answer anyway: it
    # reports `Scan unavailable: bluetoothOff` from the radio itself, and `50-device-scan` is where that
    # lands. So this says what it could not establish and lets the run go on to find out.
    if [ "$radio" -eq 2 ]; then
        yellow "  cannot tell whether Bluetooth is on -- no bluetoothctl, or no controller"
        yellow "  carrying on: if the radio really is off, the scan will say so from the app itself"
        return 0
    fi

    if ! action_required \
        "Turn Bluetooth ON" \
        "This run needs the radio and it is off. A run that turns it off asks for it back on its way out;" \
        "one that was killed outright cannot, so it is still off from last time." \
        "Nothing has been pressed yet: this is asked before the app is given anything to fail at."
    then
        return 1
    fi

    # **Asked again rather than taken on trust**, which is this suite's first principle and cheap here: somebody can
    # answer y to a banner without having done the thing, and every check after this would then fail about the app.
    bluetooth_is_on
}

# `expect_colours "name" "name cyan, glyph label, figure red"` -- one check on what the status item is drawn in.
#
# **The only way a script can see any of this.** The accessibility tree carries no colour at all, so the whole of the
# menu bar colour scheme -- cyan while the app times by hand, green while a cube does, yellow once it cannot be heard,
# red on a spent limit -- is invisible to `ax-dump.py`. `MenuBarController` writes what it drew into `debug_log`
# instead, and the row is the evidence. `12-daily-limit` used to say plainly that it could only check the spoken half.
#
# **A state read, not a wait, and deliberately not baselined.** The row is written when the colours *change* rather
# than per draw, so the newest one is what is on screen now -- which is the question a check here is asking. A
# baselined wait would be the wrong shape twice over: it would fail where the colour is already right and was
# therefore never written again, and it would pass on a row that has since been replaced.
#
# Polled, because the redraw follows the app noticing rather than the script asking.
expect_colours() {
    local name="$1" want="Menu bar: $2" timeout="${3:-15}" seen
    announce "$name"
    seen=$(wait_sql "$want" "SELECT message FROM debug_log WHERE tag = 'status' ORDER BY debug_log_id DESC LIMIT 1;" "$timeout")
    if [ "$seen" = "$want" ]; then
        verdict_pass
        grey "          $seen"
    else
        verdict_fail "expected '$2', got '${seen:-no status row at all}'"
    fi
}

# Waits for a query to return `want`, for a side effect that lands in a table rather than the log.
wait_for_value() {
    local query="$1" want="$2" timeout="${3:-15}"
    local waited=0
    while [ "$waited" -lt "$((timeout * 10))" ]; do
        [ "$(anysql "$query")" = "$want" ] && return 0
        sleep 0.1
        waited=$((waited + 1))
    done
    return 1
}

# ---------------------------------------------------------------------------- the app

is_running() { platform_app_is_running; }

# Builds if the app is missing or older than the sources, then launches it.
#
# **The build is not optional and not a convenience.** A binary older than the change under test passes
# and proves nothing; that has cost this project an hour once already (see Tests/Methods.md, Method 1).
#
# **The step is the same on both platforms and only the method differs**, which is what `platform.sh` is
# for. Is it up; is it stale; build it; launch it; wait for it -- that sequence is the check, and it reads
# the same here whether the build underneath is `swift-bundler` making a signed `.app` or `swift build`
# producing a bare executable.
ensure_app_running() {
    if is_running; then
        step "app: already running"
        if platform_warn_if_unsigned; then
            step "note: the running app is ad-hoc signed, so anything reading the Keychain (Google sync)"
            blue "        will stall on a prompt. Quit it and let this rebuild, or use scripts/run.sh."
        fi
        return 0
    fi

    # **Asked before building rather than discovered by a failing build.** On Linux there is no executable
    # product in the manifest yet (item 11), and "no product named FacetMac" reported as a build failure
    # reads as something broken rather than as something not written.
    platform_app_is_declared
    case $? in
        0) ;;
        1)  red "  there is no app to build on this platform yet"
            blue "        Package.swift declares no executable product here. See docs/linux-port.md item 11."
            exit 2 ;;
        *)  red "  cannot tell whether there is an app to build; the reason is above"
            exit 2 ;;
    esac

    # **Newer sources mean a stale binary, on both platforms.** `find -newer` is the whole test and it is
    # POSIX, so the one line covers a `.app` bundle's executable and a bare one alike.
    if [ ! -x "$BINARY" ] || [ -n "$(find Sources -newer "$BINARY" -name '*.swift' -print -quit 2>/dev/null)" ]; then
        if [ -x "$BINARY" ]; then
            step "building (sources are newer than the binary)..."
        else
            step "building (there is no binary yet)..."
        fi
        # **Said out loud.** The output used to be discarded, so a failed build reported only that it had
        # failed, and the reason -- usually one line -- had to be reproduced by hand. `platform_build_app`
        # prints it and returns non-zero; what it means is decided here.
        if ! platform_build_app; then
            red "  the build failed; running an old binary would prove nothing"
            exit 2
        fi
    fi

    step "launching $(platform_binary_built_at)"
    platform_launch_app
    local waited=0
    while [ "$waited" -lt 100 ]; do
        is_running && { sleep 1; return 0; }
        sleep 0.1
        waited=$((waited + 1))
    done
    red "  the app did not start"
    exit 2
}

# ---------------------------------------------------------------------------- clicking the status item
#
# **A real mouse event at a screen point**, because the status item is not in `AXMenuBar` and cannot be
# pressed by name (Tests/Methods.md). `scripts/status-item-click.py` is the whole of that layer; these wrap
# it so two things stop going wrong, both measured on run 78 (2026-08-23).
#
# **The wait, because macOS merges clicks and the app reads the same interval.** Two clicks inside the
# double-click interval arrive as one double, and `MenuBarController` waits out `NSEvent.doubleClickInterval`
# before deciding a right-click was a pause rather than a lock. `57-cube-pause` fired five clicks in 1.7
# seconds and the system swallowed two pairs into doubles -- "The waiting cube pause was dropped: a second
# click made it a lock", twice -- and the click after them produced no event at all. So every click here
# waits the system's own threshold plus a margin first. A deliberate double is unaffected: the pair inside it
# is generated by the python in one go, not by two calls to this.
#
# **And no silence.** Every call site used to end `>/dev/null 2>&1`, discarding the output, the error and the
# exit code alike, so a click that never happened was indistinguishable from one the app ignored. On run 78
# that cost a 20-second timeout reported against the cube when nothing had clicked at all. A failure here is
# printed and returned.
STATUS_CLICK_GAP=""
status_click_gap() {
    if [ -z "$STATUS_CLICK_GAP" ]; then
        # The system setting the app itself reads. Unset is the common case and means the 0.5s default.
        local threshold
        threshold=$(defaults read -g com.apple.mouse.doubleClickThreshold 2>/dev/null || true)
        STATUS_CLICK_GAP=$(python3 -c "print(round(float('${threshold:-0.5}') + 0.35, 2))" 2>/dev/null || echo 0.85)
    fi
    printf '%s' "$STATUS_CLICK_GAP"
}

# **Posted is not received, and the difference is the whole of run 78's failure.** `CGEventPost` hands an event
# to the window server and returns; nothing in its exit status says an app took it. A click can be posted
# perfectly and reach nobody -- the item moved between the accessibility read and the post, a menu was already
# open and swallowed it, the app was busy. All of those exit 0.
#
# So the app's own record is what counts as proof. Every click `MenuBarController` handles writes a
# `Status item clicked:` row, so this waits for one and retries once if none arrives. That turns "an event was
# posted" into "the app handled a click", which is exactly what was assumed and untrue on run 78.
#
# **Retrying can double a gesture, so it is bounded and it is loud.** The row is written synchronously in the
# handler, so three seconds is generous; but if the first click did land and its row merely arrived late, the
# retry is a second real click -- and on the right half two singles are a pause and then a resume. One retry
# only, and the total number of clicks the app recorded is reported whenever a retry was needed, so a doubled
# gesture is visible in the log rather than left to be deduced from behaviour further down.
click_status_item() {
    local opening output status landed attempt=0 rows
    opening=$(mark)
    while [ "$attempt" -lt 2 ]; do
        attempt=$((attempt + 1))
        local before
        before=$(mark)
        sleep "$(status_click_gap)"
        output=$(python3 scripts/status-item-click.py "$@" 2>&1)
        status=$?
        if [ "$status" -ne 0 ]; then
            red "  the status item click failed (exit $status)${output:+: $output}"
            return 1
        fi

        landed=$(wait_for "$before" "Status item clicked:%" 3)
        if [ -n "$landed" ]; then
            if [ "$attempt" -gt 1 ]; then
                rows=$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $opening AND message LIKE 'Status item clicked:%';")
                yellow "  the click landed on attempt $attempt, and the app recorded ${rows:-?} click(s) in all"
            fi
            return 0
        fi
        red "  the click was posted but the app recorded no click (attempt $attempt): ${output:-no output}"
    done

    red "  the status item never received a click, so whatever it was meant to do has not happened"
    return 1
}

# The status item's own line, which is where the lock badge shows up. Matched through the spoken description rather
# than the drawn title: the badge is an image attachment and every attachment is the same character in text, so a
# title cannot tell a lock apart from a category icon. `setAccessibilityLabel(title.spoken)` spells it out, which is
# what makes it assertable at all.
#
# **Here rather than in whichever script wanted it first**, for the reason the radio helpers above are: it was
# byte-identical in four of them and a fifth was about to be written, which is the point at which a copy stops being
# cheaper than a shared line.
status_item() {
    platform_status_item
}

# **The four gestures, and only the first has a counterpart on both platforms.**
#
# A macOS status item exposes no accessible action at all, so its menu has to be opened with a real
# mouse event and the two halves of it do different things. An `AppIndicator` publishes one activation
# and the panel owns everything else, so there is no right half here for the app to distinguish and
# nothing it could listen for.
#
# `click_left` is a no-op on Linux rather than a refusal, and that is the honest answer instead of a
# convenient one: what it exists to establish is that the menu is reachable, and the tray menu is a
# D-Bus object whose items can be read and chosen without it ever being shown. `menu_press` works
# whether or not this was called. The other three refuse loudly -- a gesture that silently did nothing
# would make the wait after it time out and blame the cube, which `CLAUDE.md` records costing a run
# twenty seconds and a wrong diagnosis.
click_left() {
    case "$PLATFORM" in
        mac)   click_status_item ;;
        linux) return 0 ;;
    esac
}
click_right()        { platform_click_right; }
double_click_left()  { case "$PLATFORM" in mac) click_status_item --double ;; *) platform_click_right --double ;; esac; }
double_click_right() { case "$PLATFORM" in mac) click_status_item --right --double ;; *) platform_click_right --double ;; esac; }

# `menu_press <identifier>` -- choose an item of the status item's menu, by the identifier
# `StatusItemMenu.Identifier` gives it. Named that way on both platforms even though only one of them
# carries the identifier to the menu; `platform_menu_press` is where that difference lives.
menu_press() {
    local output status
    output=$(platform_menu_press "$1")
    status=$?
    [ "$status" -ne 0 ] && red "  choosing the menu item $1 failed (exit $status)${output:+: $output}"
    return $status
}

# Quits through the menu, which is the app's own way out and the only one that runs the quit sequence.
quit_app() {
    is_running || return 0
    close_settings
    # **Through the port, which is what item 24 of `docs/linux-port.md` was.** This used to click the
    # status item and press `quit-app` itself, both macOS-only and the second of them with
    # `>/dev/null 2>&1` -- the swallowed failure `CLAUDE.md` names twice. So `run.sh` quit through the
    # port and every check script quit through this older copy, and on Linux the older copy could not
    # work at all.
    local output status
    output=$(platform_quit_app)
    status=$?
    [ -n "$output" ] && printf '%s\n' "$output" | sed 's/^/  /'
    [ "$status" -ne 0 ] && red "  the menu quit failed (exit $status); falling back to a kill below"
    local waited=0
    while [ "$waited" -lt 100 ]; do
        is_running || return 0
        sleep 0.1
        waited=$((waited + 1))
    done
    step "the menu quit did not take; killing"
    platform_kill_app
}

# ---------------------------------------------------------------------------- driving the window

# `press <identifier>` -- clicks the named element, and **says so when it cannot**.
#
# **Silence here has cost this suite two debugging sessions, and they looked nothing like a press failing.** Pressing
# something that is not on screen did nothing and reported nothing, so the wait after it timed out and blamed whatever
# it was waiting on: `57-cube-pause` reported the radio for a click that never happened (2026-08-23), and `00-setup`
# reported that a cube would not take an auto-pause of 0 when nothing had reached the app at all (run 137,
# 2026-08-29). Both diagnoses pointed away from the fault, which is worse than no diagnosis.
#
# **The output is still discarded on success**, which is the honest half of the old shape: `ax-press.py` narrates what
# it pressed and that is noise in a run of hundreds. What is kept is the failure, printed and returned -- `CLAUDE.md`
# calls this out by name, and the rule is that discarding output is fine and discarding the failure is not.
#
# **The status is returned rather than made fatal.** Callers that want to stop check it; the rest get a red line in
# the log where their fault actually happened rather than thirty seconds later somewhere else.
press() {
    local output status
    output=$(platform_press "$1")
    status=$?
    [ "$status" -ne 0 ] && red "  the press of $1 failed (exit $status)${output:+: $output}"
    return $status
}

press_title() {
    local output status
    output=$(platform_press_title "$1")
    status=$?
    [ "$status" -ne 0 ] && red "  the press of the button titled $1 failed (exit $status)${output:+: $output}"
    return $status
}

# Posts Return, for the one thing a value cannot be written into: an inline edit that commits on the key
# rather than on a button (Tests/Methods.md Method 10). The field being typed into holds focus, so there
# is nowhere else for the key to land.
#
# **Escape is never posted, here or anywhere.** It reaches whatever has focus, and what has focus is
# often the terminal session driving the app.
#
# **The app is brought to the front first.** A posted key event goes to whoever is frontmost, not to
# whoever the script last addressed -- every other helper here works through accessibility, which does not
# care about that, so this is the one place where being in front matters at all. Without it a Return can
# land in the terminal running the suite, and the symptom is a commit that produced no log row and no
# error: exactly the shape of a rename that silently did nothing on 2026-08-16.
# **The flags are set to nothing explicitly, and that is not belt and braces.** A keyboard event built
# from a NULL source inherits the session's current modifiers, so a Return posted after anything that
# left one held goes out as a chord -- and a chord is not what commits an edit. Measured on run 145:
# `post_key v --command` set the Command flag and never released the key, `press` and `set_field` post no
# events at all so nothing disturbed it, and this Return arrived as command-Return two minutes later. The
# evidence said the field was focused and held the right text, which is a commit that never happened
# looking exactly like an app that ignored it. `ax-key.py` no longer leaves a modifier held; this makes it
# so that nothing else can either.
press_return() {
    # **Through the port, rather than the inline AppleScript-and-Quartz this used to be.** What that
    # spelled out is exactly what `ax-key.py` and `at-key.py` each already do for their own platform:
    # bring the app to the front, clear any modifier the session left held, and post the key. Keeping a
    # second copy of it here is what let the two drift on run 145.
    local output status
    output=$(platform_key return)
    status=$?
    [ "$status" -ne 0 ] && red "  posting Return failed (exit $status)${output:+: $output}"
    return $status
}
press_desc() { platform_press_desc "$1" >/dev/null 2>&1; }

# `post_key v --command` -- a real keystroke with modifiers, to whatever holds focus.
#
# **The only way to exercise a keyboard shortcut**, and the reason is the same one `MainMenu` is written around:
# `NSApplication.sendEvent` offers a key-down to the main menu before anything else sees it, and it is the menu item
# found there that turns ⌘V into `paste(_:)` on the field editor. Nothing accessibility can do goes near that path --
# `set_field` writes the value outright and `press` presses a control -- so a menu that had quietly stopped carrying
# the item would leave every one of those checks passing.
#
# Everything `press_return`'s comment says about posting a key applies here too: it lands wherever focus is, so the
# app is brought to the front by `ax-key.py` itself and the caller has to have opened the edit already.
post_key() {
    local output status
    output=$(platform_key "$@")
    status=$?
    [ "$status" -ne 0 ] && red "  posting the key $* failed (exit $status)${output:+: $output}"
    return $status
}
# The unfocused write, for a field something has already clicked into. Reports a failure for the reason the two
# above do: a value that never reached the field fails later, somewhere else, as something it is not.
set_field() {
    local output status
    output=$(platform_set_field "$1" "$2")
    status=$?
    [ "$status" -ne 0 ] && red "  writing $2 into $1 failed (exit $status)${output:+: $output}"
    return $status
}

# For a field nothing has clicked into, where the Return that follows has to commit an edit.
#
# **`set_field` alone is not enough there.** It writes AXValue, which changes what the field shows without
# opening an editing session, so a `press_return` afterwards goes to whatever does have focus and the
# value is never committed -- the field displays the new number and the table keeps the old one. A
# category name escapes this because pressing the name makes the field first responder itself; a daily
# limit does not, and typed into with `set_field` it silently did nothing (measured 2026-08-16: the tree
# showed `category-limit-3 value=45` while the table still held 0).
# The same, for writing into a field: `ax-set.py` exits non-zero with the reason on stderr -- the element not being
# in the tree, or refusing to take focus, which is what a disabled field does -- and both used to be thrown away.
set_field_focused() {
    local output status
    output=$(platform_set_field_focused "$1" "$2")
    status=$?
    [ "$status" -ne 0 ] && red "  writing $2 into $1 failed (exit $status)${output:+: $output}"
    return $status
}
tree()       { platform_tree; }

# `on_tab <identifier>` -- how many elements carry exactly this identifier, 0 or 1 for anything named once.
#
# **Use this rather than `grep -c "id=X"` wherever one identifier is a prefix of another**, which is more places
# than it looks. A stepper names its field `device-auto-pause` and its two buttons `-up` and `-down`, so the loose
# grep answers 3; `device-scan` sits in front of `-all`, `-status` and every `-result-<uuid>`, so it answers 3 as
# well and more once a scan has found something. A check expecting 1 then fails while the tab is perfectly correct,
# and a check expecting 0 passes for the wrong reason. Measured on a running app, 2026-08-22.
on_tab() { tree | grep -cE "id=$1(\$|[[:space:]])" || true; }

# The buttons of the alert that is up, in the order they are drawn, joined with `|` so one check can
# assert the whole set rather than probing for each.
#
# **Read off the sheet, not off the window.** Several checks here turn on a button being *absent*, and
# a grep over the tree would count a `Cancel` belonging to something else -- reporting the button as
# present when the alert never offered it. Empty when no sheet is up, which fails such a check instead
# of passing it silently.
# **Sorted, so the answer does not depend on which way the platform walks the dialogue.**
#
# The two report opposite orders for the same arrangement and neither is wrong. NSAlert's accessibility tree
# enumerates its buttons last-added-first, so macOS answers `Delete Calendar|Cancel` for a dialogue whose choices
# read `["Cancel", "Delete Calendar"]`; GTK packs them in order and is walked left to right, so Linux answers
# `Cancel|Delete Calendar` for the same dialogue drawn the same way round. What these checks are about is which
# buttons are offered -- several of them turn on one being *absent* -- and that is a set rather than a sequence.
#
# `LC_ALL=C` so the order is the same on both machines whatever locale they are in. `04-categories` had already
# sorted at one call site for this reason; doing it here is that fix made general.
alert_buttons() { platform_alert_buttons | LC_ALL=C sort | tr '\n' '|' | sed 's/|$//'; }

# Whether a sheet is up at all. Worth its own check before opening the next one: an alert nobody
# dismissed is modal, so every later press lands on nothing and the failures arrive somewhere else.
alert_is_open() { platform_alert_buttons >/dev/null 2>&1; }

# Answers the sheet that is up, by the title of one of its buttons.
#
#     press_sheet "Reset Device"      # agrees
#     press_sheet Cancel              # declines
#
# **Scoped to the sheet, and that is not tidiness.** A confirmation names its agreeing button after the
# control that opened it, so `Reset Device` matches two elements -- the sheet's, and the one still behind
# it. `press` searches the whole tree, finds the one underneath first, and presses *that*: the sheet goes
# unanswered and a second one opens on top of it. Measured 2026-08-17, and it read as a reset that
# silently did nothing. See Tests/Methods.md Method 12.
press_sheet() { platform_press_sheet "$1" >/dev/null 2>&1; }

# One element's line from the tree, so a check reads the thing it is about rather than searching the
# whole window -- and a failure prints that line rather than several hundred.
# **Matched to the end of the identifier, not to a space after it.** This was `grep -m1 "id=$1 "`, which requires
# something to follow -- and a line whose last token is the identifier then never matches. On macOS every element
# has a title or a value after its id, so it never came up; on Linux a control with neither ends the line there, and
# `pair_a_cube` reported that the Scan button was not on screen while looking straight at it (2026-09-20).
#
# **The trailing space was doing real work and is kept as an alternative**, which is why this is not simply
# `grep "id=$1"`: `device-scan` is a prefix of `device-scan-all`, `-status` and every `-result-<id>`, so a loose
# match answers about the wrong element. `on_tab` already spells it this way and its comment records the same trap.
element() { tree | grep -m1 -E "id=$1($|[[:space:]])" || true; }

# `window_width <identifier>` -- how wide a window is on screen, in points, or empty if it is not there.
#
# **The only thing in this suite that reads geometry**, and the reason it can exist at all is that
# `ax-dump.py --frames` reports each element's position and size. Nothing else here needs it: what a script
# usually wants to know is whether an element is present and what it says, and both come off the plain tree.
#
# **What it is for is a fault `swift test` cannot see.** A pane's constraints are checked hermetically against a
# container of a fixed width, so a control that demands more width than the window has looks perfectly correct
# there -- the container simply obliges. On screen the *window* obliges instead, and grows. Measured 2026-09-04:
# selecting the App tab widened the window by two hundred points, because a wrapping footnote asks for its whole
# text on one line, and `AppSettingsPane.fittingSize` reported the same number before and after the fix.
window_width() {
    platform_tree_frames \
        | grep -m1 "id=$1 " \
        | sed -n 's/.*size=w:\([0-9]*\)\.*[0-9]* .*/\1/p'
}

# Whether the tree holds a string, **asked without a pipeline**, which is the whole point of it existing.
#
# **`something | grep -q ...` is not a reliable test under `set -o pipefail`, and this file sets it.** `grep -q`
# exits the moment it matches, the command on the left is then killed by SIGPIPE while it is still writing, and
# pipefail reports the pipeline as that signal -- 141, not 0 -- so a match reads as a miss. It is a race, so it
# fails intermittently, which is worse than failing always: measured 2026-08-25 against a signed app bundle,
# `codesign -dvvv ... | grep -q TeamIdentifier` answered "not found" on 18 runs out of 20.
#
# That one was visible, printing a note saying the app was ad-hoc signed when it was not, and writing `ad-hoc`
# into every run in `logs/testlog.sqlite`. The ones here were not: `settings_is_open` answering false on an open
# window opens a second one, and `wait_for_element` answering false polls until it times out and blames the app.
#
# A `case` on a captured string cannot do any of that. Every status test on the tree goes through this.
tree_has() {
    case "$(tree)" in
        *"$1"*) return 0 ;;
        *) return 1 ;;
    esac
}

settings_is_open() { tree_has "close-settings"; }

# Waits for an element to appear, rather than sleeping a guessed amount and hoping.
#
# **Pressing something that rebuilds the pane leaves a gap.** A refused rename reloads the whole list, so
# for a moment the row being addressed is a view on its way out, and a press lands on nothing. A fixed
# sleep either loses that race or wastes time on every run that would not have. This lost it on
# 2026-08-16: the name cell was still a button when the field was written to, so nothing was committed and
# no log row was written at all -- a failure with no evidence in it beyond the tree.
wait_for_element() {
    local identifier="$1" timeout="${2:-5}" waited=0
    while [ "$waited" -lt "$((timeout * 5))" ]; do
        tree_has "id=$identifier " && return 0
        sleep 0.2
        waited=$((waited + 1))
    done
    return 1
}

open_settings() {
    settings_is_open && return 0
    click_left || return 1
    sleep 0.5
    menu_press open-settings
    sleep 1
    settings_is_open
}

close_settings() {
    settings_is_open || return 0
    press close-settings
    sleep 0.5
}

# **Two genuinely different gestures behind one step**, which is why it goes through the port rather
# than through `press_desc`.
#
# On macOS the tabs carry no `AXIdentifier` and a Settings tab button is matched on its description
# instead -- `Tests/Methods.md` Method 2's finding, still true of this app's window. On Linux a
# `page tab` implements no Action interface at all, so there is nothing to press: `queryAction()`
# raises, and the tab is *selected* through the Selection interface of the tab list above it
# (Method 20). `press_desc` on this side would have found the tab and then failed to do anything with
# it, which is the shape of failure this suite spends its comments warning about.
select_tab() {
    local output status
    output=$(platform_select_tab "$1")
    status=$?
    [ "$status" -ne 0 ] && red "  switching to the $1 tab failed (exit $status)${output:+: $output}"
    sleep 0.7
    return $status
}
