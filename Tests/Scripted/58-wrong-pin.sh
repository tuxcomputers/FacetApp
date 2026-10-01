#!/bin/bash
# A paired cube that refuses this app's PIN: the not-found notice, Rescan while the PIN is still wrong, and taking
# Time by Hand. Then the real PIN goes back into the store and a fresh launch reaches the cube.
#
# **The cube is in range and answering, and refuses.** `56-manual-mode` puts the cube out of reach; this puts a PIN
# the cube is not on into the store. The reconnect presents the stored PIN and then the vendor PIN
# (`login::reconnect_candidates`), each on its own connection, so a wrong stored PIN leaves nothing the cube accepts.
# The launch ends in the same "The TimeFlip was not found" notice a cube out of range gets. What differs is the
# `debug_log` row, `Not the paired cube: <label> refused the PIN...`.
#
# **Starts and ends with the cube paired, connected, unlocked and running on Break.** The quit before the wrong PIN
# goes in leaves the cube paused and locked, and `free_the_cube` frees it after the last relaunch.
#
# **The real PIN is read out of the trace before anything is changed.** The app logs every password write in the
# Bluetooth trace as `password withResponse: <hex> (<pin>)`, the PIN being no secret here, so the write just before
# the newest `PIN accepted` row names the PIN the cube is on. Nothing is changed when it cannot be read.
#
# **One PIN store: the secret store item at service `au.com.tux.facet.cube`, account `pin`**, through the `keyring`
# crate. On macOS that is a generic password in the login Keychain with those as its service and account; on Linux a
# Secret Service item in the default collection with attributes `service` and `username`, labelled
# `keyring:pin@au.com.tux.facet.cube`.
#
# **The Keychain on macOS, and what may prompt.** The wrong PIN and the real one are both written over the app's own
# item with `security add-generic-password -U`, which changes the item's data and keeps its access list, so the app
# stays the trusted reader of its own item. An item that `security` creates afresh trusts only `security`, and the
# app's read of that raises a Keychain prompt, which is why the item is never deleted here. Whether `security` may
# change the data of an item the app owns without a prompt of its own has not been measured. Both writes are bounded
# at 20 seconds, and the app gives a read up after 30 (`timed::look_up`), logging `The stored PIN could not be read`,
# which this script reports as a prompt. A prompt left on screen stays there after its caller is killed and needs a
# person to cancel it. On Linux nothing prompts while the login collection is unlocked.
#
# **The real PIN is put back by a trap on EXIT, INT and TERM**, so a failed check or Ctrl-C still restores it. A kill
# the trap cannot catch, or a write the store refuses, leaves the store on the wrong PIN while the cube is on the real
# one. The banner in `restore_pin` says so and prints the command that puts it back. Taking the batteries out also
# recovers it: the cube returns to 000000, the next launch presents the wrong PIN and then 000000, the cube accepts
# 000000, and the app moves it onto a new PIN of its own and stores that.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/swiftParity`:
#
# - **No `config.json`.** The Swift app kept a PIN in the file as well as the Keychain and presented both; the Rust app
#   has the one store, so everything about the file and about the two stores disagreeing is gone.
# - **The notice writes its own rows**: `Notice shown: TITLE, offering CHOICES` in place of `Offering manual mode:`,
#   and `Notice answered: CHOICE` in place of `Rescan chosen;` and `Time by hand chosen;`.
# - **No row for the order devices are asked in, none for the scan starting, and no `Launch mode:` row**, so those
#   checks are gone.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database

# Not run on macOS: it rewrites the PIN item with the `security` tool, and the app's next read of that item raises a
# Keychain prompt that an unattended run cannot answer. It runs on Linux, whose secret store does not prompt. A script
# that declares no checks is not counted as short, and the stamp shows this one with none.
if [ "$PLATFORM" = "mac" ]; then
    EXPECTED_CHECKS=0
    start "a cube that refuses this app's PIN: not run on macOS"
    step "not run on macOS: the Keychain prompts when the app reads an item the security tool has rewritten"
    finish
    exit 0
fi

ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=32
start "a cube that refuses this app's PIN: the not-found notice, Rescan, and timing by hand"

require_a_paired_cube "there is no paired cube for the PIN to be wrong for"

PIN_SERVICE="au.com.tux.facet.cube"
PIN_ACCOUNT="pin"
PIN_LABEL="keyring:pin@au.com.tux.facet.cube"
# The PIN the cube is on, read from the trace below. Empty until then.
PIN_REAL=""
PIN_WRONG=""
# 1 while the store holds a PIN the cube is not on.
PIN_BROKEN=0

# `bounded <seconds> <stdin> <command...>`: runs the command with `stdin` as its standard input, and kills it once
# `seconds` have passed. Answers the command's own status, or 124 when it was killed.
bounded() {
    local seconds="$1" input="$2" pid waited=0
    shift 2
    printf '%s' "$input" | "$@" &
    pid=$!
    while kill -0 "$pid" 2>/dev/null; do
        if [ "$waited" -ge "$((seconds * 10))" ]; then
            kill "$pid" 2>/dev/null
            wait "$pid" 2>/dev/null
            return 124
        fi
        sleep 0.1
        waited=$((waited + 1))
    done
    wait "$pid"
}

# `write_pin <pin>`: puts `pin` into the app's PIN item. Answers 0 once the store took it, and otherwise prints why
# and answers non-zero.
write_pin() {
    local pin="$1" output status
    case "$PLATFORM" in
        mac)   output=$(bounded 20 "" security add-generic-password -U -s "$PIN_SERVICE" -a "$PIN_ACCOUNT" -w "$pin" 2>&1) ;;
        linux) output=$(bounded 20 "$pin" secret-tool store --label="$PIN_LABEL" service "$PIN_SERVICE" username "$PIN_ACCOUNT" 2>&1) ;;
    esac
    status=$?
    [ "$status" -eq 0 ] && return 0
    if [ "$status" -eq 124 ]; then
        red "  writing the PIN item did not finish in 20s: a keyring prompt is probably on screen and needs answering"
    else
        red "  writing the PIN item failed (exit $status)${output:+: $output}"
    fi
    return "$status"
}

# What the store says about the PIN item. On Linux: the number of matching items and the secret, as `<n> item, <pin>`.
# On macOS: `present` or `absent`, from the item's attributes alone, since reading the secret back through `security`
# is a read by a program the item does not trust, and that prompts. A probe whose failure is the answer.
store_reading() {
    local count secret
    case "$PLATFORM" in
        mac)
            security find-generic-password -s "$PIN_SERVICE" -a "$PIN_ACCOUNT" >/dev/null 2>&1 \
                && echo present || echo absent ;;
        linux)
            count=$(bounded 20 "" secret-tool search --all service "$PIN_SERVICE" username "$PIN_ACCOUNT" 2>&1 \
                | grep -c '^\[/' || true)
            secret=$(bounded 20 "" secret-tool lookup service "$PIN_SERVICE" username "$PIN_ACCOUNT" 2>/dev/null)
            printf '%s item, %s\n' "${count:-0}" "$secret" ;;
    esac
}

# What `store_reading` answers when the store holds `pin` and nothing else.
store_expected() {
    case "$PLATFORM" in
        mac)   echo present ;;
        linux) printf '1 item, %s\n' "$1" ;;
    esac
}

# The six-digit PIN a `password withResponse: <hex> (<pin>)` row carries, or empty when it carries none.
pin_in() {
    printf '%s\n' "$1" | sed -n 's/^password withResponse: .*(\([0-9]\{6\}\))$/\1/p'
}

# The PIN the first password write after `since` presented, or empty when there was none.
first_presented() {
    pin_in "$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $1 AND message LIKE 'password withResponse: %' ORDER BY debug_log_id LIMIT 1;")"
}

# The command that puts the real PIN back by hand, for the banner.
restore_command() {
    case "$PLATFORM" in
        mac)   printf 'security add-generic-password -U -s %s -a %s -w %s' "$PIN_SERVICE" "$PIN_ACCOUNT" "$PIN_REAL" ;;
        linux) printf 'printf %%s %s | secret-tool store --label=%s service %s username %s' \
                   "$PIN_REAL" "$PIN_LABEL" "$PIN_SERVICE" "$PIN_ACCOUNT" ;;
    esac
}

# Puts the real PIN back while the store holds the wrong one. Does nothing otherwise, so it is safe to call twice.
restore_pin() {
    [ "$PIN_BROKEN" = "1" ] || return 0
    if write_pin "$PIN_REAL"; then
        PIN_BROKEN=0
        step "the PIN item holds the cube's own PIN again ($PIN_REAL)"
        return 0
    fi
    echo ""
    yellow "##############################################################################"
    yellow "##"
    yellow "##  THE STORED PIN IS STILL WRONG"
    yellow "##"
    yellow "##  The store holds $PIN_WRONG and the cube is on $PIN_REAL. Every launch"
    yellow "##  is refused by the cube until it is put back. Put it back with:"
    yellow "##    $(restore_command)"
    yellow "##"
    yellow "##  Or take the cube's batteries out: it goes back to 000000, and the next"
    yellow "##  launch logs in on 000000 and moves the cube onto a new PIN it stores."
    yellow "##"
    yellow "##############################################################################"
    echo ""
    return 1
}
trap restore_pin EXIT
trap 'restore_pin; exit 130' INT TERM

# ---------------------------------------------------------------------------- the PIN the cube is on

accepted=$(dsql "SELECT IFNULL(MAX(debug_log_id), 0) FROM debug_log WHERE message = 'PIN accepted';")
PIN_REAL=$(pin_in "$(dsql "SELECT message FROM debug_log WHERE debug_log_id < ${accepted:-0} AND message LIKE 'password withResponse: %' ORDER BY debug_log_id DESC LIMIT 1;")")
case "$PIN_REAL" in
    000000 | "")
        fail "the PIN the cube is on is in the trace, and is not the vendor PIN (read ${PIN_REAL:-nothing})"
        finish
        exit 1
        ;;
esac
if [ "$PLATFORM" = "linux" ] && [ "$(store_reading)" != "$(store_expected "$PIN_REAL")" ]; then
    fail "the trace and the store agree on the PIN before anything is changed (store: $(store_reading))"
    finish
    exit 1
fi
pass "the PIN the cube is on is read from the trace, so it can be put back ($PIN_REAL)"

PIN_WRONG=123457
[ "$PIN_WRONG" = "$PIN_REAL" ] && PIN_WRONG=123458

# ---------------------------------------------------------------------------- the PIN goes wrong
#
# With the app shut, so the next launch is the first read of the new value.

quit_app
sleep 1

PIN_BROKEN=1
if ! write_pin "$PIN_WRONG"; then
    fail "the store took a PIN the cube is not on"
    finish
    exit 1
fi
check "the store holds a PIN the cube is not on ($PIN_WRONG)" "$(store_expected "$PIN_WRONG")" "$(store_reading)"

# ---------------------------------------------------------------------------- found, and refused

launched=$(mark)
ensure_app_running

expect_log "a paired app goes looking for its cube" "$launched" "Looking for the paired cube" 30
announce "the first PIN presented is the one just written, read from the store at the point of use"
first=$(wait_for "$launched" "password withResponse: %" 60)
case "$first" in
    *"($PIN_WRONG)")
        verdict_pass
        grey "          $first"
        ;;
    "")
        unreadable=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $launched AND message LIKE 'The stored PIN could not be read%' ORDER BY debug_log_id LIMIT 1;")
        if [ -n "$unreadable" ]; then
            verdict_fail "$unreadable (on macOS a Keychain prompt went unanswered for 30s; cancel it if it is still on screen)"
        else
            verdict_fail "no PIN was presented within 60s"
        fi
        ;;
    *)
        verdict_fail "the first PIN presented was: $first"
        ;;
esac
expect_log "the cube is in range and answering" "$launched" "Connected to %, presenting a PIN next" 60
expect_log "and refuses that PIN" "$launched" "PIN refused" 30
expect_log "and there is another PIN to try" "$launched" "Refused, and there is another PIN to try" 30
expect_log "which is the vendor PIN" "$launched" "password withResponse: % (000000)" 30
expect_log "the row says the cube refused the PIN, not that it was not found" "$launched" \
    "Not the paired cube: % refused the PIN.%" 60
expect_log "so the reconnect ends having reached no cube" "$launched" \
    "The paired cube was not reconnected: The paired cube was not found%" 30
check "and no PIN was accepted" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $launched AND message = 'PIN accepted';")"

# ---------------------------------------------------------------------------- the notice

expect_log "the Settings window is shown on the Device tab for it" "$launched" "Settings opened on Device" 15
expect_log "with the same not-found notice a cube out of range gets" "$launched" \
    "Notice shown: The TimeFlip was not found, offering Rescan, Time by Hand, Quit" 10
notice=$(notice_text "was not found" 10)
check_contains "the notice says the TimeFlip was not found" "$notice" "The TimeFlip was not found"
check "and offers Rescan, Time by Hand and Quit" "Quit|Rescan|Time by Hand" "$(alert_buttons)"
check "and says nothing about a PIN, being the same notice a cube out of range gets" "0" \
    "$(printf '%s\n' "$notice" | grep -ciw 'pin' || true)"

# ---------------------------------------------------------------------------- Rescan, with the PIN still wrong

retried=$(mark)
if ! press_title "Rescan"; then
    fail "Rescan could not be pressed on the notice"
    finish
    exit 1
fi
expect_log "Rescan is the answer taken" "$retried" "Notice answered: Rescan" 5
expect_log "Rescan looks for the cube again" "$retried" "Looking for the paired cube" 15
expect_log "and presents the wrong PIN again" "$retried" "password withResponse: % ($PIN_WRONG)" 60
expect_log "and is refused again" "$retried" "Not the paired cube: % refused the PIN.%" 60
wait_for "$retried" "The paired cube was not reconnected:%" 30 >/dev/null
check_contains "so the notice comes back" "$(notice_text "was not found" 10)" "The TimeFlip was not found"

# ---------------------------------------------------------------------------- Time by Hand

chosen=$(mark)
if ! press_title "Time by Hand"; then
    fail "Time by Hand could not be pressed on the notice"
    finish
    exit 1
fi
expect_log "Time by Hand is the answer taken" "$chosen" "Notice answered: Time by Hand" 5
expect_log "Time by Hand times this launch by hand" "$chosen" \
    "Timing by hand for this launch, the cube not having been found" 15
# `alert_buttons` answers nothing when no notice is up, which is the answer being waited for.
buttons=$(alert_buttons)
for _ in $(seq 1 25); do
    [ -z "$buttons" ] && break
    sleep 0.2
    buttons=$(alert_buttons)
done
check "and the notice is gone" "" "$buttons"
check "the cube is still paired, a refused PIN saying nothing about which cube this app has" "1" \
    "$(setting paired paired)"

quiet=$(mark)
sleep 12
check "no further look is made on its own" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $quiet AND message = 'Looking for the paired cube';")"
check "and nothing is connected to" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $quiet AND message LIKE 'Connecting to %';")"

# ---------------------------------------------------------------------------- the real PIN goes back

quit_app
sleep 1
if ! restore_pin; then
    fail "the store took the real PIN back"
    finish
    exit 1
fi
check "the store holds the real PIN again ($PIN_REAL)" "$(store_expected "$PIN_REAL")" "$(store_reading)"

relaunched=$(mark)
ensure_app_running
expect_log "a fresh launch reaches the cube again" "$relaunched" "Reconnected to %" 90
presented=$(first_presented "$relaunched")
if [ "$presented" = "$PIN_REAL" ]; then
    pass "on the real PIN, presented first"
else
    fail "the first PIN presented after the restore was ${presented:-none}, not $PIN_REAL"
fi
check "and the cube is left on it, nothing being rotated" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $relaunched AND message = 'Setting the PIN on the cube to one of its own';")"
if free_the_cube; then
    pass "the cube the quit locked is unlocked and running again"
else
    fail "the cube the quit locked was not freed"
fi
finish
