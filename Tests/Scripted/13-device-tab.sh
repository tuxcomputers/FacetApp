#!/bin/bash
# The Device tab with no cube paired: its two sections, the controls that are dead without a cube, and the folds.
#
# **Nothing here touches a cube.** It is the half of the Device tab a run can check on any machine: what the tab
# draws from a table that has never paired, and that every setting stays out of reach until a cube is connected.
#
# **Converted from the Swift suite 2026-09-27**, against the Device tab as `feature/deviceTab` builds it:
#
# - **The headings are pressed on their own ids**: `device-timeflip-section-heading` and
#   `device-settings-section-heading`, and `device-more` and `device-led` for the two nested ones. The Swift ids
#   ended `-heading-button`.
# - **The steppers are Slint SpinBoxes with no arrow buttons**, so the Swift checks that the `-up` and `-down`
#   arrows were dead have nothing to check; the stepper itself being dead is checked instead.
# - **Renaming is not built yet**, so the Name row is a plain value, and the two Swift checks on its refusal
#   are gone until it is.
# - **The Scan button is live on both platforms**, each build having a radio.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=37
start "the Device tab's two sections, and the folds that need no cube"

open_settings
select_tab Device

# ---------------------------------------------------------------------------- the sections

for pair in "device-timeflip-section TimeFlip" "device-settings-section Settings"; do
    set -- $pair
    check_contains "the $2 section is on the tab" "$(tree)" "id=$1"
done
check "and there is no third section left over" "0" "$(on_tab device-info-section)"
check "nor the old pairing one" "0" "$(on_tab device-pairing-section)"

check "the TimeFlip section starts open" "1" "$(on_tab device-connection)"
check "the Settings section starts open" "1" "$(on_tab device-auto-pause)"
check "the Name row and the scan button are in one section" "1" "$(on_tab device-name)"
check "and the scan button is there with it" "1" "$(on_tab device-scan)"

check "nothing is paired, which is what this tab is drawn from" "0" "$(setting paired paired)"
check "so nothing is connected either" "0" "$(setting connection connected)"

# ---------------------------------------------------------------------------- dead with no cube

press device-led
sleep 0.5
for control in \
    device-pause-on-lock \
    device-battery-warning \
    device-auto-pause \
    device-led-brightness \
    device-led-blink
do
    check "$control is dead with no cube connected" "1" \
        "$(tree | grep -cE "id=$control([[:space:]].*)?disabled" || true)"
done
press device-led
sleep 0.5
check "and the inner fold is back as it was built" "0" "$(on_tab device-led-brightness)"

check_contains "the Name row says there is no device" "$(element device-name)" "Not paired"

scan_line=$(element device-scan)
if [ -n "$scan_line" ] && [[ "$scan_line" != *disabled* ]]; then
    pass "the Scan button is live, this build having a radio"
else
    fail "the Scan button is not live: $scan_line"
fi

# ---------------------------------------------------------------------------- the folds

for pair in "device-timeflip-section device-connection" "device-settings-section device-auto-pause"; do
    set -- $pair
    section="$1" inside="$2"

    since=$(mark)
    press "$section-heading"
    sleep 1
    expect_log "pressing the $section heading folds it" "$since" "Device section $section folded"
    check "and its rows go with it" "0" "$(on_tab "$inside")"

    since=$(mark)
    press "$section-heading"
    sleep 1
    expect_log "pressing it again opens it" "$since" "Device section $section opened"
    check "and its rows come back" "1" "$(on_tab "$inside")"
done

# **More is folded by default and keeps its state inside the TimeFlip fold.** Folding the outer section hides the
# inner one's rows; opening it again finds More as it was left, not reset.
check "More starts folded, so its rows are not in the tree" "0" "$(on_tab device-manufacturer)"
press device-more
sleep 0.7
check "opening More brings its rows out" "1" "$(on_tab device-manufacturer)"
press device-timeflip-section-heading
sleep 0.7
check "folding TimeFlip takes the open More with it" "0" "$(on_tab device-manufacturer)"
press device-timeflip-section-heading
sleep 0.7
check "and opening TimeFlip finds More still open, not reset" "1" "$(on_tab device-manufacturer)"

press device-timeflip-section-heading
sleep 0.7
check "folding TimeFlip takes the scan button with the readings" "0" "$(on_tab device-scan)"
press device-timeflip-section-heading
sleep 0.7
check "and both come back together" "1" "$(on_tab device-scan)"

# ---------------------------------------------------------------------------- a fresh window

# **Closing and opening again puts every fold back as a fresh window has it**, and says nothing while doing so:
# the fold rows are written by a person pressing a heading, not by the window being built.
press device-settings-section-heading
sleep 0.7
check "precondition: Settings is folded" "0" "$(on_tab device-auto-pause)"

before_reset=$(mark)
close_settings
open_settings
select_tab Device
check "the next open finds Settings back open" "1" "$(on_tab device-auto-pause)"
check "and TimeFlip back open too" "1" "$(on_tab device-connection)"
check "and More back to folded, which is its own default" "0" "$(on_tab device-manufacturer)"
resets=$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $before_reset AND message LIKE 'Device section %';")
check "and it said nothing while putting them back" "0" "${resets:-0}"

finish
