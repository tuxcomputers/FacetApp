#!/bin/bash
# Disconnecting a Google account and connecting it again, with the calendar surviving in between.
#
# **The one check in this suite that needs your hands.** Signing in goes through a browser and a Google
# consent screen, which a script must not do on somebody's behalf. So the script presses Sign in, the
# browser opens on this machine's screen, and that browser is the prompt: the run waits for the account to
# come back and carries on by itself. Nobody signing in within four minutes is a fail, not a skip.
#
# What it is really testing is a deliberate asymmetry in what a disconnect throws away:
#
#   - **The identity goes.** Name and email are cleared, and the refresh token is removed from the secret
#     store, because a store still able to act on an account the app says it is not connected to is the
#     thing a disconnect is for.
#   - **The calendar stays.** `calendar_id` and `calendar_name` are kept on purpose. Signing out and back in
#     on the same account is the ordinary case, and forgetting the id would make a second calendar beside
#     the first. The id is *checked* on the way back in rather than trusted.
#
# The proof that access came back is not a row: it is `Google calendar confirmed`, which the app writes only
# after really fetching the calendar from Google.
#
# **Converted from the Swift suite 2026-09-27.** It asks y/n at the terminal before opening the browser, and pauses
# once the sign-in has come back, as the Swift script did. The secret store is checked directly, before and after,
# where the Swift script took the disconnect's word.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=20
start "disconnecting Google and connecting again, with the calendar surviving"

open_settings
select_tab App

account() { sql "SELECT json_extract(setting_value, '\$.$1') FROM setting WHERE setting_name = 'google_account';"; }

email=$(account email)
before_id=$(account calendar_id)
before_name=$(account calendar_name)

if [ -z "$email" ]; then
    fail "no Google account is connected, so there is nothing to disconnect"
    finish
    exit $?
fi
if [ -z "$before_id" ]; then
    fail "connected, but no calendar was made, so nothing can survive the disconnect"
    finish
    exit $?
fi

pass "connected as $email, with a calendar to keep ($before_name)"
check "and the secret store holds its sign-in" "yes" "$(token_stored)"

# ---------------------------------------------------------------------------- disconnecting

# One button for both, titled from the state: "Disconnect" while connected, "Sign in with Google" while not.
check_contains "the button offers to disconnect" "$(element app-google-button)" "Disconnect"

since=$(mark)
press app-google-button
sleep 1.5
expect_log "disconnecting is recorded" "$since" "Google account disconnected"

# **REFUSED is in the same log line**, so a disconnect the table would not accept is not read as success.
refused=$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE '%disconnected REFUSED%';")
check "and the table accepted it" "0" "$refused"

check "the email is cleared" "" "$(account email)"
check "and the sign-in is gone from the secret store" "no" "$(token_stored)"
check_contains "the Status row says so" "$(element_eventually app-google-status "Not connected")" "Not connected"
check_contains "and the button offers to sign in again" "$(element_eventually app-google-button "Sign in with Google")" "Sign in with Google"

# ---- the point of the whole script

check "the calendar id is NOT wiped out" "$before_id" "$(account calendar_id)"
check "and neither is its name" "$before_name" "$(account calendar_name)"

# ---------------------------------------------------------------------------- connecting again

if ! action_required \
    "Sign in to Google, so the run can check the calendar survived." \
    "1. Press y. Facet opens your browser at Google's sign-in page." \
    "2. Sign in as $email -- the same account, or the calendar will not resolve." \
    "3. Approve the access it asks for, then come back here." \
    "" \
    "The run waits for you and carries on by itself once you are done." \
    "Answering anything else leaves Facet SIGNED OUT, and every later run's" \
    "Google checks fail until you sign in on the App tab by hand."; then
    fail "the sign-in was declined, so reconnecting is untested"
    echo ""
    red   "  #########################################################################"
    red   "  ##  Facet is left signed out of Google."
    red   "  ##  The calendar id is still stored, so signing in on Settings -> App"
    red   "  ##  will pick the same calendar back up."
    red   "  #########################################################################"
    echo ""
    finish
    exit 0
fi

since=$(mark)
asked=$SECONDS
press app-google-button

announce "waiting for the sign-in to come back (up to 4 minutes)"
if wait_for_value "SELECT json_extract(setting_value, '\$.email') != '' FROM setting WHERE setting_name = 'google_account';" "1" 240; then
    note_human_wait "$asked"
    verdict_pass
else
    note_human_wait "$asked"
    verdict_fail "no account was recorded within 4 minutes"
    finish
    exit 1
fi

# Stops once the sign-in is detected: the browser is in front, and what the app does next (checking the stored
# calendar against Google) is worth watching. The checks below read from the mark taken before the sign-in, so the
# pause cannot lose anything.
wait_for_dev "the sign-in came back" \
    "Facet is connected again as $email." \
    "" \
    "Bring Facet's Settings window to the front if you want to watch what follows:" \
    "it re-checks the stored calendar against Google, and the next checks read" \
    "whether it confirmed the same one or gave up and made another."

check "it is the same account" "$email" "$(account email)"
check "and its sign-in is back in the secret store" "yes" "$(token_stored)"
check_contains "and the Status row agrees" "$(element_eventually app-google-status "Connected")" "Connected"

# ---- still has access to the calendar
#
# **This is a real request to Google, not a row being read.** The app fetches the calendar by its stored id on
# the way back in, which proves both that the id still addresses something and that the new token can reach
# it. A stored id that had stopped resolving would take the other branch and be forgotten.

expect_log "the calendar is confirmed against Google" "$since" "Google calendar confirmed,%" 60

check "it is the same calendar, not a new one" "$before_id" "$(account calendar_id)"
check "under the name it had before" "$before_name" "$(account calendar_name)"
check_contains "and the Calendar row shows it" "$(element_eventually app-google-calendar "$before_name")" "$before_name"

# Nothing was made. A sign-in that could not resolve the stored id offers a fresh calendar instead, which is
# correct behaviour and the wrong outcome here.
made=$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Google calendar created%';")
check "no second calendar was made" "0" "$made"

finish
