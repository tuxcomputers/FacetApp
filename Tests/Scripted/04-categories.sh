#!/bin/bash
# The Categories tab: creating one, renaming it, retiring it, and bringing it back.
#
# Everything below this script depends on a category existing, since a face holds one and a time entry is
# filed under one. It runs before them for that reason.
#
# **The categories it makes are numbered and left behind.** `next_name` reads the highest `Scripted N`
# already there and goes one past. An active name has to be unique, and nothing here deletes what it made,
# because the rows are the evidence.
#
# **Converted from the Swift suite 2026-09-25**, against the Categories tab as `feature/catergoryTab` builds it.
# What changed under it, and what that did to the checks:
#
# - **Every question is an in-window notice now**, not a native alert: `notice-choice-<n>` buttons in the tree,
#   read by `alert_buttons` and pressed by `press_title` (platform.sh). The wording is the Rust app's.
# - **The stepper hold is not here.** Slint's SpinBox has no `-up`/`-down` arrows to hold and repeats at its own
#   rate, so the Swift acceleration (1 per 0.1s to the second multiple of 5, then 5 per 0.3s) was not built and
#   there is nothing to time. The typed-value checks stay; the eight hold checks went with the feature.
# - **Return on an open question does nothing**, where on macOS it was bound to Cancel. The claim that matters
#   is the same -- Return must not agree -- so that is what is checked, and the question is then answered.
# - **A refused write now raises a notice** ("The change was not saved"). Nothing a script can do provokes one
#   short of breaking the table, so it is not checked here.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=98
start "creating, renaming, retiring and reinstating a category"

open_settings
select_tab Categories

NAME=$(next_name Scripted)
RENAMED="$NAME renamed"

# Turns a category's name into an editable field, and waits until it really is one. A refused rename re-reads
# the list, so the row is briefly on its way out and a fixed sleep loses the race.
open_name_field() {
    press "$1"
    wait_for_element "$1-field" 5
}

begin_rename() { open_name_field "category-name-$1"; }

# The same gesture on the Inactive list, which names its rows differently so a step cannot press one
# believing it addressed the other.
begin_retired_rename() { open_name_field "retired-category-name-$1"; }

# **One section open at a time while working in it**, because a row scrolled out of the pane is not in the tree
# at all: Slint leaves out whatever the scroll view clips, so a row below the fold cannot be found, let alone
# pressed. Measured 2026-09-25 on the Linux box, whose window opens 400 points tall: after the namesakes the
# Inactive rows sat below the pane and `retired-category-active-6` was simply absent. Folding the other section
# is what a person with a short window does too. The whole heading is the control.
section_is_open() { tree_has "id=$1"; }

show_inactive() {
    if section_is_open "category-detail-row-"; then
        press categories-active-section-heading
        sleep 1
    fi
    if ! section_is_open "retired-category-row-"; then
        press categories-inactive-section-heading
        sleep 1
    fi
}

show_active() {
    if section_is_open "retired-category-row-"; then
        press categories-inactive-section-heading
        sleep 1
    fi
    if ! section_is_open "category-detail-row-"; then
        press categories-active-section-heading
        sleep 1
    fi
}

# ---------------------------------------------------------------------------- the two sections
#
# **First, before anything expands the Inactive one**, whose folded state is only observable until this script
# opens it.

check_contains "the Active section is on the tab" "$(tree)" "id=categories-active-section"
check_contains "and the Inactive section is too" "$(tree)" "id=categories-inactive-section"
check "the Inactive section starts folded" "0" "$(tree | grep -c "id=retired-category-name-" || true)"

# ---------------------------------------------------------------------------- what Save will not create
#
# **Nothing typed is not an error and not a notice, it is nothing done.** Whitespace normalises to empty first,
# so a field of spaces cannot make a category whose name nobody can see or type again.

before_count=$(sql "SELECT COUNT(*) FROM category;")

press create-category
sleep 0.5
check_contains "pressing Create opens a name field" "$(tree)" "id=category-name-field"

press save-category
sleep 0.8
check "saving an empty field creates nothing" "$before_count" "$(sql "SELECT COUNT(*) FROM category;")"

press create-category
sleep 0.3
set_field category-name-field "   "
press save-category
sleep 0.8
check "and neither does a field of spaces" "$before_count" "$(sql "SELECT COUNT(*) FROM category;")"
check "with no notice raised for either" "no" "$(alert_is_open && echo yes || echo no)"

# ---------------------------------------------------------------------------- pasting into a field
#
# Here because this is the first open field the suite has. The shortcut is the field's own, and a keystroke is
# the only way to prove it reaches the field.
press create-category
sleep 0.5
PASTED="Pasted $(date +%s)"
platform_copy_to_clipboard "$PASTED"
post_key v --command
sleep 0.5
check_contains "a field takes a paste" "$(element category-name-field)" "$PASTED"

# Emptied and saved rather than left open: saving nothing is already known to create nothing.
set_field category-name-field ""
press save-category
sleep 0.8
check "and the paste is not left behind as a category" "$before_count" "$(sql "SELECT COUNT(*) FROM category;")"

# ---------------------------------------------------------------------------- create
#
# **Creating here starts nothing**, where the Faces tab's create starts the clock: a name on this tab is a list
# being maintained rather than a day being recorded.

since=$(mark)
press create-category
sleep 0.5
set_field category-name-field "$NAME"
press save-category
sleep 1

expect_log "a typed name is saved as a new category" "$since" "%Save new category $NAME -> Some(%"

ID=$(created_category_id "$since")
if [ -n "$ID" ]; then
    pass "the new category has an id ($ID)"
else
    fail "the app did not report a category id, so the rest of this script cannot address it"
    finish
    exit 1
fi

check "the table holds it, active" "1" "$(sql "SELECT active FROM category WHERE category_id = $ID;")"
check "under the name that was typed" "$NAME" "$(sql "SELECT category_name FROM category WHERE category_id = $ID;")"
check_contains "its row is on the tab" "$(tree)" "id=category-name-$ID "
check "and nothing started timing it" "0" "$(sql "SELECT COUNT(*) FROM device_event WHERE finalised = 0;")"

# ---------------------------------------------------------------------------- the length limit
#
# **35 characters, and the table is what enforces it**: `category_name` carries a `CHECK`. The field holds to the
# same number as it is typed into, so on Linux the typing is cut there, and what is checked is what the row holds.
#
# The 35th character is not a space, deliberately: the cut trims, so a phrase breaking on one would land at 34.
OVER="A category name far beyond the limit it is given"
since=$(mark)
press create-category
sleep 0.5
set_field_cut category-name-field "$OVER"
press save-category
sleep 1

CUT_ID=$(created_category_id "$since")
if [ -n "$CUT_ID" ]; then
    pass "a name over the limit still creates a category (id $CUT_ID)"
else
    fail "nothing was created from a name over the limit, so the app refused what it should have cut"
fi

check "the table holds 35 characters of it" "35" "$(sql "SELECT LENGTH(category_name) FROM category WHERE category_id = ${CUT_ID:-0};")"
check "and they are the first 35 that were typed" "${OVER:0:35}" "$(sql "SELECT category_name FROM category WHERE category_id = ${CUT_ID:-0};")"

# ---------------------------------------------------------------------------- rename
#
# The name is a button that becomes a field, and Return commits it.

since=$(mark)
begin_rename "$ID"
set_field_focused "category-name-$ID-field" "$RENAMED"
press_return
sleep 1

expect_log "committing a new name asks first" "$since" "%rename -> $RENAMED, asking%"
check "nothing is renamed until the question is answered" "$NAME" "$(sql "SELECT category_name FROM category WHERE category_id = $ID;")"

since=$(mark)
press_title Rename
sleep 1
expect_log "answering Rename does it" "$since" "%Rename $NAME -> $RENAMED"
check "the table holds the new name" "$RENAMED" "$(sql "SELECT category_name FROM category WHERE category_id = $ID;")"

# The id is what history hangs off, and a rename that made a new row would strand every entry recorded
# against the old one.
check "and it is the same row, not a new one" "1" "$(sql "SELECT COUNT(*) FROM category WHERE category_id = $ID;")"

# ---------------------------------------------------------------------------- retire

since=$(mark)
press "category-active-$ID"
sleep 1
expect_log "unticking Active retires it" "$since" "%$RENAMED retired%"
check "the table says inactive" "0" "$(sql "SELECT active FROM category WHERE category_id = $ID;")"

# **Matched on `id=category-name-N `**, id and trailing gap both: the retired row is `retired-category-name-N`,
# which contains the shorter string, and `N` is a prefix of `N0`.
check "it is gone from the active table" "0" "$(tree | grep -c "id=category-name-$ID " || true)"

show_inactive
check_contains "it is listed under Inactive" "$(tree)" "id=retired-category-name-$ID "

# ---------------------------------------------------------------------------- reinstate

since=$(mark)
press "retired-category-active-$ID"
sleep 1
expect_log "ticking it there brings it back" "$since" "%$RENAMED reinstated"
check "the table says active" "1" "$(sql "SELECT active FROM category WHERE category_id = $ID;")"
show_active
check_contains "and its row is back on the active table" "$(tree)" "id=category-name-$ID "

# **A retired category keeps everything.** Nothing above deleted a row, which is the point of retiring.
check "its name survived the round trip" "$RENAMED" "$(sql "SELECT category_name FROM category WHERE category_id = $ID;")"

# ============================================================================ retired namesakes
#
# Typing a name a *retired* category already holds. Only active names are unique, so there are two legitimate
# answers and the app asks rather than choosing. With several retired namesakes Reactivate is withdrawn, there
# being no "the old one" to bring back.

# Creates a category and retires it, leaving exactly one retired row under `name`. Prints its id.
make_retired() {
    local name="$1" id
    show_active
    press create-category
    sleep 0.5
    set_field category-name-field "$name"
    press save-category
    wait_for_value "SELECT COUNT(*) FROM category WHERE category_name = '$name' AND active = 1;" "1" 10 || return 1
    id=$(sql "SELECT category_id FROM category WHERE category_name = '$name' AND active = 1;")
    press "category-active-$id"
    wait_for_value "SELECT active FROM category WHERE category_id = $id;" "0" 10 || return 1
    printf '%s' "$id"
}

# Types `name` into the create control and saves it, which is what raises the notice.
ask_about() {
    show_active
    press create-category
    sleep 0.5
    set_field category-name-field "$1"
    press save-category
    sleep 1.2
}

# How many rows hold the name, and how many of those are active, as one reading.
tally() {
    sql "SELECT COUNT(*) || ' rows, ' || SUM(active) || ' active' FROM category WHERE category_name = '$1';"
}

REACTIVATE="$NAME reactivate"
CREATE_NEW="$NAME create new"
CANCELLED="$NAME cancelled"

# ---- the notice itself

reactivate_id=$(make_retired "$REACTIVATE")
if [ -z "$reactivate_id" ]; then
    fail "could not make a retired category, so the namesake checks below have no fixture"
    finish
    exit 1
fi
check "one deactivated category holds the name ($REACTIVATE)" "1 rows, 0 active" "$(tally "$REACTIVATE")"

since=$(mark)
ask_about "$REACTIVATE"
expect_log "typing it again asks instead of inserting" "$since" "%Save new category $REACTIVATE -> asking, 1 retired%"
check "and nothing is created while the question is open" "1 rows, 0 active" "$(tally "$REACTIVATE")"
check "the notice offers three buttons" "Cancel|Create new one|Reactivate" "$(alert_buttons)"

# ---- Reactivate

since=$(mark)
press_title Reactivate
sleep 1
expect_log "Reactivate brings that one back" "$since" "%Reactivate $REACTIVATE -> category_id $reactivate_id"
check "the same row is active again" "1" "$(sql "SELECT active FROM category WHERE category_id = $reactivate_id;")"
check "and it is still one row, not a second one" "1 rows, 1 active" "$(tally "$REACTIVATE")"
check_contains "its row is back on the active table" "$(tree)" "id=category-name-$reactivate_id "

# ---- Create new one

create_new_id=$(make_retired "$CREATE_NEW")
check "a second name with one deactivated category ($CREATE_NEW)" "1 rows, 0 active" "$(tally "$CREATE_NEW")"

since=$(mark)
ask_about "$CREATE_NEW"
press_title "Create new one"
sleep 1
expect_log "Create new one inserts alongside it" "$since" "%Create new one $CREATE_NEW -> category_id %, leaving category_id $create_new_id retired"
check "there are now two rows, one active" "2 rows, 1 active" "$(tally "$CREATE_NEW")"
check "the old one was left retired" "0" "$(sql "SELECT active FROM category WHERE category_id = $create_new_id;")"

# ---- Cancel

cancelled_id=$(make_retired "$CANCELLED")
check "a third name with one deactivated category ($CANCELLED)" "1 rows, 0 active" "$(tally "$CANCELLED")"

since=$(mark)
ask_about "$CANCELLED"
press_title Cancel
sleep 1
expect_log "Cancel creates nothing" "$since" "%Cancel, $CANCELLED not created"
check "the name is still held by one retired row" "1 rows, 0 active" "$(tally "$CANCELLED")"
check "and the notice is gone" "no" "$(alert_is_open && echo yes || echo no)"

# ---------------------------------------------------------------------------- several namesakes, two ways

active_create_new=$(sql "SELECT category_id FROM category WHERE category_name = '$CREATE_NEW' AND active = 1;")
show_active
press "category-active-$active_create_new"
sleep 1
check "retiring the newer one leaves two deactivated under the name" "2 rows, 0 active" "$(tally "$CREATE_NEW")"

since=$(mark)
ask_about "$CREATE_NEW"
expect_log "the notice says how many there are" "$since" "%Save new category $CREATE_NEW -> asking, 2 retired%"

# **The absent button is the assertion**: offering Reactivate would mean the app picking one of two identically
# named rows on the user's behalf.
check "the notice offers two buttons, and no Reactivate" "Cancel|Create new one" "$(alert_buttons)"

since=$(mark)
press_title Cancel
sleep 1
expect_log "Cancel still creates nothing" "$since" "%Cancel, $CREATE_NEW not created"
check "both retired rows are untouched" "2 rows, 0 active" "$(tally "$CREATE_NEW")"

since=$(mark)
ask_about "$CREATE_NEW"
press_title "Create new one"
sleep 1
expect_log "Create new one still inserts" "$since" "%Create new one $CREATE_NEW -> category_id %"
check "a third row under the name, and only it is active" "3 rows, 1 active" "$(tally "$CREATE_NEW")"
check "the two retired ones are still retired" "2" "$(sql "SELECT COUNT(*) FROM category WHERE category_name = '$CREATE_NEW' AND active = 0;")"
check "and the notice is gone" "no" "$(alert_is_open && echo yes || echo no)"

# ---------------------------------------------------------------------------- the dead end
#
# An **active** category holds the name, so there is no second answer to offer. One button, and it only dismisses.

since=$(mark)
ask_about "$RENAMED"
expect_log "a name an active category holds is refused outright" "$since" "%Save new category $RENAMED -> already active as category_id $ID"
check "the notice offers one button, which only dismisses" "OK" "$(alert_buttons)"
press_title OK
sleep 0.8
check "nothing was created" "1" "$(sql "SELECT COUNT(*) FROM category WHERE category_name = '$RENAMED';")"

# ---------------------------------------------------------------------------- reinstating is refused
#
# Ticking a retired row's Active box when an active category already holds the name. `$CREATE_NEW` is exactly
# that shape by now. The question is asked before the write so the notice can say *which* category is in the way.

blocked=$(sql "SELECT category_id FROM category WHERE category_name = '$CREATE_NEW' AND active = 0 ORDER BY category_id LIMIT 1;")
holder=$(sql "SELECT category_id FROM category WHERE category_name = '$CREATE_NEW' AND active = 1;")

show_inactive
since=$(mark)
press "retired-category-active-$blocked"
sleep 1
expect_log "reinstating onto an active name is refused, naming what is in the way" "$since" \
    "%$CREATE_NEW reinstate REFUSED: category_id $holder is active under that name"
check "the row is still retired" "0" "$(sql "SELECT active FROM category WHERE category_id = $blocked;")"
check_contains "and the notice says the name is taken" "$(platform_alert_message)" "already in use"
press_title OK
sleep 0.5

# **The box goes back to unticked**, the row re-read from the table: a control still showing the click would be
# the two-answers problem in miniature.
show_inactive
check_contains "the row is still in the Inactive list" "$(tree)" "id=retired-category-name-$blocked "

# ---------------------------------------------------------------------------- a retired row is a record
#
# **No icon, no colour, no daily limit.** Two things stay live: the Active box that brings it back, and the name.

check "a retired row draws no icon button" "0" "$(tree | grep -c "id=category-icon-$blocked " || true)"
check "no colour button" "0" "$(tree | grep -c "id=category-colour-$blocked " || true)"
check "and no daily limit field" "0" "$(tree | grep -c "id=category-limit-$blocked " || true)"
check_contains "but its Active box is there" "$(tree)" "id=retired-category-active-$blocked "

# ---------------------------------------------------------------------------- renaming a retired row
#
# The one edit the Inactive list needs: retired namesakes pile up under a single name, and telling them apart is
# worth nothing if the answer cannot then be written down.

DISTINCT="$CREATE_NEW distinct"

show_inactive
since=$(mark)
begin_retired_rename "$blocked"
set_field_focused "retired-category-name-$blocked-field" "$DISTINCT"
press_return
sleep 1
expect_log "a retired name commits and asks first, as any other rename does" "$since" \
    "%rename -> $DISTINCT, asking%"
press_title Rename
sleep 1
check "the retired row takes the new name" "$DISTINCT" \
    "$(sql "SELECT category_name FROM category WHERE category_id = $blocked;")"
check "and it is still retired" "0" "$(sql "SELECT active FROM category WHERE category_id = $blocked;")"
check "the active namesake is untouched" "$CREATE_NEW" \
    "$(sql "SELECT category_name FROM category WHERE category_id = $holder;")"

# ---- taking a name an active category holds
#
# **Allowed, because the database allows it**: `UN1_category` is `WHERE active = 1`. It is confirmed rather than
# silent because it costs something: while the two share a name the retired one cannot be brought back.

show_inactive
since=$(mark)
begin_retired_rename "$blocked"
set_field_focused "retired-category-name-$blocked-field" "$CREATE_NEW"
press_return
sleep 1

check "taking an active name offers a way through and a way out" "Cancel|Rename anyway" "$(alert_buttons)"
check_contains "and the notice says what it costs" "$(platform_alert_message)" "brought back"

# **Return must not agree with it.** On macOS it was bound to Cancel; here nothing in the notice takes Return at
# all. Either way the claim is the same, and the table is what answers it.
press_return
sleep 1
check "Return does not answer the question with yes" "$DISTINCT" \
    "$(sql "SELECT category_name FROM category WHERE category_id = $blocked;")"
press_title Cancel
sleep 1
check "and Cancel then answers it with no" "no|$DISTINCT" \
    "$(alert_is_open && echo yes || echo no)|$(sql "SELECT category_name FROM category WHERE category_id = $blocked;")"

show_inactive
begin_retired_rename "$blocked"
set_field_focused "retired-category-name-$blocked-field" "$CREATE_NEW"
press_return
sleep 1
press_title "Rename anyway"
sleep 1
check "answering Rename anyway takes the name" "$CREATE_NEW" \
    "$(sql "SELECT category_name FROM category WHERE category_id = $blocked;")"
check "the active row still holds it too" "$CREATE_NEW" \
    "$(sql "SELECT category_name FROM category WHERE category_id = $holder;")"
check "which is one active row and two retired under one name" "3 rows, 1 active" "$(tally "$CREATE_NEW")"

# **And now it cannot come back**, which is exactly what the notice said would happen.
show_inactive
since=$(mark)
press "retired-category-active-$blocked"
sleep 1
expect_log "reinstating it is now refused, as the notice warned" "$since" \
    "%$CREATE_NEW reinstate REFUSED: category_id $holder is active under that name"
press_title OK
sleep 0.5
check "so it is still retired" "0" "$(sql "SELECT active FROM category WHERE category_id = $blocked;")"

# ---------------------------------------------------------------------------- colour and icon
#
# Both open over the pane and both write to the table. `0` is the seeded *None* row rather than a null, which is
# how either is cleared while the foreign key still holds. Picking the one already chosen is what clears it.

show_active
press "category-colour-$ID"
sleep 0.8
check_contains "the colour list opens" "$(tree)" "id=colour-option-Red"
press colour-option-Red
sleep 1
check "picking Red writes Red, by id" "$(sql "SELECT colour_id FROM colour WHERE colour_name = 'Red';")" \
    "$(sql "SELECT colour_id FROM category WHERE category_id = $ID;")"

press "category-colour-$ID"
sleep 0.8
press colour-option-Red
sleep 1
check "picking Red again clears it to None" "0" "$(sql "SELECT colour_id FROM category WHERE category_id = $ID;")"

first_icon=$(sql "SELECT icon_name FROM icon WHERE icon_id = 1;")
press "category-icon-$ID"
sleep 0.8
check_contains "the icon grid opens" "$(tree)" "id=icon-cell-$first_icon"
press "icon-cell-$first_icon"
sleep 1
check "picking an icon writes it" "1" "$(sql "SELECT icon_id FROM category WHERE category_id = $ID;")"

press "category-icon-$ID"
sleep 0.8
press "icon-cell-$first_icon"
sleep 1
check "picking it again clears it to None" "0" "$(sql "SELECT icon_id FROM category WHERE category_id = $ID;")"

# ---------------------------------------------------------------------------- the daily limit
#
# In minutes, with 0 meaning no limit. **Written as a value, which is what the SpinBox commits**: its set-value
# action goes through `update-value`, which emits `edited`, so there is no Return to send.

others_before=$(sql "SELECT IFNULL(SUM(daily_limit), 0) FROM category WHERE category_id != $ID;")

since=$(mark)
set_field "category-limit-$ID" "45"
sleep 1
expect_log "a daily limit is written as it is set" "$since" "%$RENAMED daily limit -> 45min"
check "and the table holds it" "45" "$(sql "SELECT daily_limit FROM category WHERE category_id = $ID;")"

# **Nobody else moved.** A field that wrote to the wrong row would still pass the check above.
check "and no other category's limit moved" "$others_before" \
    "$(sql "SELECT IFNULL(SUM(daily_limit), 0) FROM category WHERE category_id != $ID;")"

set_field "category-limit-$ID" "0"
sleep 1
check "setting it back to zero lifts it" "0" "$(sql "SELECT daily_limit FROM category WHERE category_id = $ID;")"

# ---------------------------------------------------------------------------- the rename dead ends

# ---- Cancel leaves the name alone

since=$(mark)
begin_rename "$ID"
set_field_focused "category-name-$ID-field" "$NAME abandoned"
press_return
sleep 1
press_title Cancel
sleep 1
expect_log "cancelling a rename is recorded" "$since" "%not renamed"
check "and the name is untouched" "$RENAMED" "$(sql "SELECT category_name FROM category WHERE category_id = $ID;")"

# ---- renaming onto an active namesake
#
# **The edit closes, and the name has to be typed again**, as the Swift app did. Handing the field back holding the
# rejected name is still owed.

since=$(mark)
begin_rename "$reactivate_id"
set_field_focused "category-name-$reactivate_id-field" "$RENAMED"
press_return
sleep 1
check "renaming onto an active name offers only Cancel" "Cancel" "$(alert_buttons)"
press_title Cancel
sleep 1
expect_log "and cancelling it says the name was taken" "$since" "%Cancel, $REACTIVATE rename refused, name taken"
check "the table is untouched" "$REACTIVATE" "$(sql "SELECT category_name FROM category WHERE category_id = $reactivate_id;")"

# ---- capitalisation only is not a collision
#
# **A row is not in its own way.** Names are matched case-insensitively, as the unique index is, so a category
# matches itself when only its capitalisation changes.

CAPITALISED=$(printf '%s' "$REACTIVATE" | tr '[:lower:]' '[:upper:]')
since=$(mark)
begin_rename "$reactivate_id"
set_field_focused "category-name-$reactivate_id-field" "$CAPITALISED"
press_return
sleep 1
expect_log "changing only the capitalisation is offered, not refused" "$since" "%rename -> $CAPITALISED, asking%"
press_title Rename
sleep 1
check "and it lands" "$CAPITALISED" "$(sql "SELECT category_name FROM category WHERE category_id = $reactivate_id;")"

# Finished, so the row goes back to being a name rather than a field.
check_contains "the field closes once a rename is accepted" "$(tree)" "id=category-name-$reactivate_id "

finish
