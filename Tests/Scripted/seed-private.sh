#!/bin/bash
# The seeds that cannot be written down here: real accounts, real ids, anything belonging to one person.
#
# **Sourced by `00-setup.sh`, never run on its own.** It is separated from the ordinary seeds because the
# two have opposite rules. An ordinary seed is written into `00-setup.sh` in full, where anybody can read
# what a run starts from. A private one holds a real email address, a real calendar id, and in time
# whatever else identifies a person's account -- so what lives here is the *mechanism*, and the values
# live outside the repository entirely.
#
# This repository takes outside contributions. A seed committed into it would put one developer's account
# into everybody's checkout, and the first anybody would know is their own calendar filling with somebody
# else's time.
#
# Where the values go: `~/.config/facet/scripted-seed.json`, 0600, in the directory that already holds
# the OAuth client credentials for the same reason.
#
# To add another private seed: capture it in `capture_private_seeds` and write it back in
# `apply_private_seeds`, keyed by name in the same file. Nothing else needs to change.

SEED_DIR="$HOME/.config/facet"
SEED="$SEED_DIR/scripted-seed.json"

# Reads the private values out of the database as it stands, before a rebuild takes them away.
#
# **Silent and never fatal.** This runs before every clean run, and a machine with nothing to capture --
# anybody who has not connected an account -- is the ordinary case rather than a problem.
capture_private_seeds() {
    local value email
    value=$(sql "SELECT setting_value FROM setting WHERE setting_name = 'google_account';" 2>/dev/null || true)
    email=$(printf '%s' "$value" | python3 -c "
import json, sys
try:
    print(json.load(sys.stdin).get('email') or '')
except Exception:
    print('')" 2>/dev/null || true)

    [ -z "$email" ] && return 0

    mkdir -p "$SEED_DIR"
    chmod 700 "$SEED_DIR" 2>/dev/null || true

    # **The calendar id is the only record of what to delete.** `calendarList.list` returns nothing usable
    # under `calendar.app.created` (measured 2026-08-15), so a calendar this file forgets can never be
    # found again and has to be deleted by hand in Google Calendar. It is put back into the rebuilt
    # database, and 10 deletes it there before making a fresh one.
    #
    # A delete that fails leaves the id in the row, so the next capture picks it up again and the next run
    # tries once more. Nothing else is needed to make that work, which is why the separate retry list this
    # used to carry has gone.
    # **A capture never trades a known calendar for a blank one.** The database can legitimately hold no
    # calendar -- a run that deleted one and stopped before making another leaves exactly that -- and
    # writing that over the seed would throw away the only record of a calendar still sitting in somebody's
    # account, which nothing can then find again. So an empty id yields to the one already on file.
    printf '%s' "$value" | SEED_FILE="$SEED" python3 -c "
import json, os, pathlib, sys

path = pathlib.Path(os.environ['SEED_FILE'])
account = json.loads(sys.stdin.read() or '{}')

if not (account.get('calendar_id') or '').strip() and path.exists():
    try:
        known = json.loads(json.loads(path.read_text()).get('google_account', '{}'))
    except ValueError:
        known = {}
    if (known.get('calendar_id') or '').strip():
        account['calendar_id'] = known['calendar_id']
        account['calendar_name'] = known.get('calendar_name', '')

path.write_text(json.dumps({'google_account': json.dumps(account)}, indent=2))
path.chmod(0o600)
"
    chmod 600 "$SEED" 2>/dev/null || true
    echo "Captured the connected Google account for the next run ($email)."
}

# Writes them into the fresh database.
apply_private_seeds() {
    if [ ! -f "$SEED" ]; then
        step "no private seeds at $SEED"
        step "connect a Google account once -- it is captured before the next rebuild and put back after"
        return 0
    fi

    # **The token survives a rebuild and the account does not.** The refresh token is in the login
    # Keychain, which no rebuild touches, so the machine can still reach Google perfectly well while a
    # fresh database says nobody is connected. Putting the row back is what lets the calendar checks run
    # on a clean database instead of skipping every time.
    local account email stored calendar
    account=$(python3 -c "
import json
with open('$SEED') as f:
    print(json.load(f).get('google_account', ''))" 2>/dev/null || true)

    if [ -z "$account" ]; then
        step "the private seed file holds no Google account"
        return 0
    fi

    email=$(printf '%s' "$account" | python3 -c "import json,sys; print(json.load(sys.stdin).get('email') or '')")

    # Written and then read back, as the app does for its own writes: a statement that reported success
    # and did not happen would leave every later script testing a connection that is not there, and
    # reporting it as a Google failure.
    sql "UPDATE setting SET setting_value = '$(printf '%s' "$account" | sed "s/'/''/g")' WHERE setting_name = 'google_account';"

    stored=$(sql "SELECT json_extract(setting_value, '\$.email') FROM setting WHERE setting_name = 'google_account';")
    # **Narrated, not checked.** This is arrangement rather than a claim about the app, and it used to be two
    # `check` calls -- one of them conditional, so 00-setup ran two verdicts on some runs and three on others.
    # That is the moving count the `expected` column exists to forbid, and on run 73 it was the first thing it
    # caught. The seeding now reports through `trouble`, and 00-setup answers for all of it once.
    if [ "$stored" != "$email" ]; then
        trouble "the Google account would not seed (wanted $email, the table holds ${stored:-nothing})"
        return 0
    fi
    step "the Google account is seeded ($email)"

    # Reported because it is what the next step needs in order to delete the right calendar, not because
    # the run wants to keep it. Either way 10 makes a fresh one.
    calendar=$(sql "SELECT json_extract(setting_value, '\$.calendar_name') FROM setting WHERE setting_name = 'google_account';")
    if [ -n "$calendar" ]; then
        step "naming last run's calendar, which 10 deletes before making its own ($calendar)"
    else
        step "the seed names no calendar, so 10 makes the first one"
    fi
}
