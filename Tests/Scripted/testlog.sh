#!/bin/bash
# The durable record of every scripted run: `logs/testlog.sqlite`.
#
#   Tests/Scripted/testlog.sh              a report on the last few runs and what is failing
#   Tests/Scripted/testlog.sh 12           the same, over the last 12 runs
#
# Sourced by `lib.sh`, which records into it as checks happen. Run directly, it reports.
#
# **Why this exists rather than just `logs/screen.txt`.** screen.txt is one run, overwritten by the next,
# and it holds only what was printed. Two things are missing from it, and both are what a failure actually
# needs:
#
#   1. **The app's own `debug_log` rows, which the next run destroys.** They live in `test.sqlite`, and a
#      clean run rebuilds that file from the DDL. So the evidence behind this morning's failure is gone by
#      the time anybody sits down to read about it. They are copied in here as each script ends.
#   2. **Every other run.** "Has this ever passed?", "when did it start failing?", "does it fail one run in
#      four?" are the questions that decide whether a red check is a bug or a flaky script, and none of them
#      can be asked of a single file that was overwritten.
#
# **Nothing here may fail a run.** Every write is best-effort: a broken or locked log database must not turn
# a passing run red, because this file is a record of the tests and not one of them. That is why every
# statement ends `2>/dev/null || true`.
#
# **The design is for reading failures, not for storing output.** Checks carry a `check_key` -- the
# description with every run of digits replaced by `#` -- so the same check is one row across runs even
# though its text carries a clock-stamped category name and a fresh row id every time. Without that,
# comparing runs is impossible and this is just screen.txt with extra steps.

TESTLOG="${TESTLOG:-logs/testlog.sqlite}"

# Escapes a value for a single-quoted SQL literal.
sq() { printf '%s' "${1:-}" | sed "s/'/''/g"; }

# Runs a statement against the log, swallowing everything. See the note above about never failing a run.
tlog() { sqlite3 "$TESTLOG" "$1" 2>/dev/null || true; }

testlog_open() {
    mkdir -p "$(dirname "$TESTLOG")" 2>/dev/null || true

    # ---------------------------------------------------------------- migrations, before the schema below
    #
    # **`CREATE TABLE IF NOT EXISTS` adds tables, never columns, and `CREATE VIEW IF NOT EXISTS` never replaces
    # a view.** So a log database made before a column existed keeps its old shape for ever, and every write
    # naming the new column fails silently, which is the quiet nothing this file promises not to do.
    #
    # **Before the schema, not after it.** Dropping `v_run` afterwards deletes the view the heredoc had just
    # decided not to rebuild, and leaves the database with no `v_run` at all -- which is what happened the
    # first time this was written the other way round. Dropping it here means the `CREATE VIEW` below finds
    # nothing and makes the current one.
    #
    # Each is offered unconditionally and allowed to fail: sqlite refuses a duplicate column and refuses to
    # drop a column that is not there, which is the normal case once a database has been through this once.
    sqlite3 "$TESTLOG" "ALTER TABLE script ADD COLUMN expected INTEGER NOT NULL DEFAULT 0;" >/dev/null 2>&1 || true
    sqlite3 "$TESTLOG" "ALTER TABLE script ADD COLUMN human_seconds INTEGER NOT NULL DEFAULT 0;" >/dev/null 2>&1 || true
    sqlite3 "$TESTLOG" "CREATE UNIQUE INDEX IF NOT EXISTS UN1_script ON script (run_id, name);" >/dev/null 2>&1 || true
    # A skip has been unreachable since the helpers stopped offering the verdict, and a column that can only
    # ever be 0 is one people read as meaning something.
    if sqlite3 "$TESTLOG" "SELECT skipped FROM run LIMIT 1;" >/dev/null 2>&1; then
        sqlite3 "$TESTLOG" "DROP VIEW IF EXISTS v_run;" >/dev/null 2>&1 || true
        sqlite3 "$TESTLOG" "ALTER TABLE run DROP COLUMN skipped;" >/dev/null 2>&1 || true
    fi

    # **Output discarded, not just errors.** `PRAGMA journal_mode` answers `wal` on stdout, and this runs
    # inside `testlog_run_start`, whose stdout *is* the new run's id. Letting it through put the word `wal`
    # in front of every id and broke every write after it.
    sqlite3 "$TESTLOG" >/dev/null 2>&1 <<'SQL' || true
PRAGMA journal_mode = WAL;

-- One row per invocation of run.sh, or per script run on its own.
CREATE TABLE IF NOT EXISTS run (
    run_id          INTEGER PRIMARY KEY,
    started_at      TEXT    NOT NULL,
    started_epoch   INTEGER NOT NULL,
    finished_at     TEXT,
    finished_epoch  INTEGER,
    branch          TEXT,
    commit_sha      TEXT,
    -- Whether the working tree had uncommitted changes. A failure against a dirty tree is not evidence
    -- about the commit it names.
    dirty           INTEGER,
    database_file   TEXT,
    -- 0 for --keep. A check that only passes on a kept database has not been proven from nothing.
    rebuilt         INTEGER,
    filter          TEXT,
    binary_built_at TEXT,
    signing         TEXT,
    os_version      TEXT,
    invocation      TEXT,
    outcome         TEXT,
    scripts_run     INTEGER,
    passed          INTEGER DEFAULT 0,
    failed          INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS script (
    script_id       INTEGER PRIMARY KEY,
    run_id          INTEGER NOT NULL REFERENCES run(run_id),
    name            TEXT    NOT NULL,
    subject         TEXT,
    sequence        INTEGER NOT NULL,
    started_epoch   INTEGER,
    finished_epoch  INTEGER,
    -- How many passing checks the script says it runs, declared as `EXPECTED_CHECKS` at the top of it.
    -- `passed` is what it actually did. The two parting company is a check that never ran at all, which
    -- every other column here reports as success.
    expected        INTEGER NOT NULL DEFAULT 0,
    passed          INTEGER DEFAULT 0,
    failed          INTEGER DEFAULT 0,
    -- Seconds the script spent waiting for a person: a prompt answered, a cube turned, a radio switched off
    -- and on. Held apart from `finished_epoch - started_epoch` rather than folded into it, because the two
    -- answer different questions -- how long the app took is worth watching for a regression, and how long
    -- somebody took to reach the keyboard is worth nothing except that the app must not be charged for it.
    -- A script that usually takes two minutes and took thirty because whoever ran it went to make coffee
    -- otherwise looks exactly like one that has broken.
    human_seconds   INTEGER NOT NULL DEFAULT 0,
    -- **No `skipped` here, deliberately.** Nothing in `Tests/Scripted` has incremented `SKIPPED` since the
    -- helpers stopped offering a skip verdict: every path now answers pass or fail, and a column that is 0 in
    -- every row of every run is a column somebody reads as meaning something. `run` keeps its own, because the
    -- stamp's `checks:` block and CI's gate are both written against it and a missing line there fails closed.
    -- If a skip is ever wanted again, it comes back here first.
    log_from        INTEGER
);

CREATE TABLE IF NOT EXISTS check_result (
    check_id      INTEGER PRIMARY KEY,
    script_id     INTEGER NOT NULL REFERENCES script(script_id),
    sequence      INTEGER NOT NULL,
    description   TEXT    NOT NULL,
    -- The description with digits normalised to `#`, so one check is one key across runs. This is what
    -- makes every cross-run question below answerable.
    check_key     TEXT    NOT NULL,
    verdict       TEXT    NOT NULL CHECK (verdict IN ('pass', 'fail', 'skip')),
    detail        TEXT,
    expected      TEXT,
    actual        TEXT,
    started_epoch INTEGER,
    seconds       INTEGER,
    -- The app's debug_log id when this check began. With the next check's, it brackets exactly the rows
    -- this check produced.
    log_from      INTEGER
);

-- Captured only when a check fails, because it costs a second and is worthless when nothing is wrong.
CREATE TABLE IF NOT EXISTS evidence (
    evidence_id INTEGER PRIMARY KEY,
    check_id    INTEGER NOT NULL REFERENCES check_result(check_id),
    kind        TEXT    NOT NULL,
    body        TEXT    NOT NULL
);

-- The app's own log, copied because test.sqlite is rebuilt by the next clean run and takes it away.
CREATE TABLE IF NOT EXISTS app_log (
    app_log_id   INTEGER PRIMARY KEY,
    run_id       INTEGER NOT NULL REFERENCES run(run_id),
    script_id    INTEGER REFERENCES script(script_id),
    debug_log_id INTEGER NOT NULL,
    logged_at    TEXT,
    tag          TEXT,
    message      TEXT
);

-- Lets the copy run twice over the same window without duplicating: once when a check fails (in case the
-- script dies before it ends) and once when it ends.
CREATE UNIQUE INDEX IF NOT EXISTS UN1_app_log ON app_log (run_id, debug_log_id);
-- **What makes a row claimable rather than only creatable.** `00-setup` writes a row for every script in the
-- run before any of them starts, so a script that is never reached is still in the record with its declared
-- count and nothing against it. `testlog_script_start` then updates the row it finds instead of adding a
-- second, and this is the key it finds it by.
CREATE UNIQUE INDEX IF NOT EXISTS UN1_script ON script (run_id, name);
CREATE INDEX IF NOT EXISTS IX1_check_key ON check_result (check_key);
CREATE INDEX IF NOT EXISTS IX2_check_script ON check_result (script_id);

-- ---------------------------------------------------------------------------- views, for reading it

-- Every failure with the context needed to place it: which run, which branch, what was expected.
CREATE VIEW IF NOT EXISTS v_failure AS
SELECT r.run_id, r.started_at, r.branch, s.name AS script, c.check_id,
       c.description, c.detail, c.expected, c.actual, c.check_key
  FROM check_result c
  JOIN script s ON s.script_id = c.script_id
  JOIN run r ON r.run_id = s.run_id
 WHERE c.verdict = 'fail'
 ORDER BY r.run_id DESC, s.sequence, c.sequence;

-- One row per check, across every run it has appeared in. The first question about a red check is
-- always "has it ever been green?".
CREATE VIEW IF NOT EXISTS v_check_history AS
SELECT c.check_key,
       COUNT(*)                                        AS runs,
       SUM(c.verdict = 'pass')                         AS passed,
       SUM(c.verdict = 'fail')                         AS failed,
       SUM(c.verdict = 'skip')                         AS skipped,
       MAX(s.run_id)                                   AS last_run,
       MAX(CASE WHEN c.verdict = 'pass' THEN s.run_id END) AS last_passed,
       MAX(CASE WHEN c.verdict = 'fail' THEN s.run_id END) AS last_failed
  FROM check_result c
  JOIN script s ON s.script_id = c.script_id
 GROUP BY c.check_key;

-- Checks that have both passed and failed. A check that has never passed is probably wrong; one that
-- passes sometimes is probably a race, and they want different fixes.
CREATE VIEW IF NOT EXISTS v_flaky AS
SELECT * FROM v_check_history WHERE passed > 0 AND failed > 0 ORDER BY failed DESC;

-- Never green anywhere. Usually a check written against behaviour the app does not have.
CREATE VIEW IF NOT EXISTS v_never_passed AS
SELECT * FROM v_check_history WHERE passed = 0 AND failed > 0 ORDER BY failed DESC;

-- Where the time goes. A check that suddenly takes 45 seconds is usually one that is timing out and
-- being rescued by a fallback rather than one that got slow.
CREATE VIEW IF NOT EXISTS v_slow AS
SELECT c.check_key, COUNT(*) AS runs, MAX(c.seconds) AS worst, AVG(c.seconds) AS average
  FROM check_result c
 WHERE c.seconds IS NOT NULL
 GROUP BY c.check_key
 HAVING worst >= 5
 ORDER BY worst DESC;

CREATE VIEW IF NOT EXISTS v_run AS
SELECT run_id, started_at, branch, commit_sha, dirty, rebuilt, signing, binary_built_at,
       database_file, os_version, outcome, scripts_run, passed, failed,
       finished_epoch - started_epoch AS seconds
  FROM run
 ORDER BY run_id DESC;
SQL

    # **`CREATE TABLE IF NOT EXISTS` adds tables, never columns**, so a log database made before a column
    # existed keeps its old shape for ever and every write naming the new column fails silently -- which is
    # exactly the kind of quiet nothing this file promises not to do. Offered unconditionally and allowed to
    # fail: sqlite refuses a duplicate column, which is the normal case and means the migration is done.
}

# ---------------------------------------------------------------------------- writing

# Starts a run row and prints its id. Called by run.sh, and by lib.sh when a script is run on its own.
testlog_run_start() {
    local rebuilt="${1:-1}" filter="${2:-}" invocation="${3:-}"
    testlog_open

    # ------------------------------------------------------------------ runs that never finished
    #
    # **A killed run leaves its row saying `running` for ever**, because the only thing that closes a row is the
    # finish it never reached. Three of them piled up on 2026-08-16 in the space of two minutes -- a run started,
    # stopped, and started again -- and the effect is not cosmetic: "the last run" answered by status rather than by
    # `run_id` then names a row that is not the last one and never ended, which is exactly the question anybody
    # querying this table is asking.
    #
    # Closed here rather than at the kill, because a killed process is precisely the one that does not get to run its
    # own tidy-up. Starting a run is the next moment anybody is looking.
    #
    # **Marked `abandoned`, not `failed`.** Nothing is known about why it stopped, and a row claiming a verdict it
    # never reached would be worse than one admitting it has none. The counts it did manage are kept, so a run killed
    # part way still shows how far it got, and it is finished at its **last recorded activity** rather than at now,
    # so its duration is not inflated by however long the row sat open.
    #
    # A genuinely concurrent run would be closed by this. That is not a case worth protecting: the suite drives one
    # app and one database, so two at once are already interfering with each other's results.
    local stranded
    stranded=$(tlog "SELECT COUNT(*) FROM run WHERE finished_epoch IS NULL;")
    if [ "${stranded:-0}" -gt 0 ]; then
        tlog "UPDATE run
                 SET finished_epoch = IFNULL(
                         (SELECT MAX(s.finished_epoch) FROM script s WHERE s.run_id = run.run_id), started_epoch),
                     finished_at = datetime(
                         IFNULL((SELECT MAX(s.finished_epoch) FROM script s WHERE s.run_id = run.run_id),
                                started_epoch),
                         'unixepoch', 'localtime'),
                     outcome     = 'abandoned',
                     scripts_run = (SELECT COUNT(*)               FROM script s WHERE s.run_id = run.run_id),
                     passed      = (SELECT IFNULL(SUM(s.passed), 0)  FROM script s WHERE s.run_id = run.run_id),
                     failed      = (SELECT IFNULL(SUM(s.failed), 0)  FROM script s WHERE s.run_id = run.run_id)
               WHERE finished_epoch IS NULL;"
        # **To stderr, and this is not a style choice.** This function returns the new run id by *printing* it --
        # `run.sh` does `TESTLOG_RUN_ID=$(testlog_run_start ...)` -- so anything else written to stdout is captured
        # as part of the id. Writing this line to stdout made run 20 record nothing at all: all 14 scripts ran and
        # passed on screen while every row went to a run id that was two lines of text.
        echo "  closed $stranded earlier run(s) that never finished, as abandoned" >&2
    fi

    local branch commit dirty target built signing os
    branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
    # The full hash, not the short one: the stamp is checked against the branch's history, and an
    # abbreviation can stop being unique as a repository grows.
    commit=$(git rev-parse HEAD 2>/dev/null || echo "")
    dirty=$([ -n "$(git status --porcelain 2>/dev/null)" ] && echo 1 || echo 0)
    target=$(readlink "$DB" 2>/dev/null || echo "")
    # **`stat` takes opposite flags on the two systems**, and the BSD spelling produces nothing on Linux
    # rather than failing -- which would have written an empty column and looked like an unbuilt app.
    built=$(platform_binary_built_at)
    # Both of these are a platform's answer, and the reasoning for each is in `platform.sh` beside the
    # call -- including the pipefail measurement that made runs 89 to 94 claim ad-hoc signing wrongly.
    signing=$(platform_signing)
    os=$(platform_os_version)

    tlog "INSERT INTO run (started_at, started_epoch, branch, commit_sha, dirty, database_file, rebuilt,
                           filter, binary_built_at, signing, os_version, invocation, outcome)
          VALUES (datetime('now','localtime'), strftime('%s','now'), '$(sq "$branch")', '$(sq "$commit")',
                  $dirty, '$(sq "$target")', $rebuilt, '$(sq "$filter")', '$(sq "$built")',
                  '$signing', '$(sq "$os")', '$(sq "$invocation")', 'running');"
    tlog "SELECT MAX(run_id) FROM run;"
}


testlog_run_finish() {
    local run="${1:-}" outcome="${2:-}" ran="${3:-0}"
    [ -z "$run" ] && return 0
    tlog "UPDATE run
             SET finished_at = datetime('now','localtime'), finished_epoch = strftime('%s','now'),
                 outcome = '$(sq "$outcome")', scripts_run = $ran,
                 passed  = (SELECT IFNULL(SUM(passed), 0)  FROM script WHERE run_id = $run),
                 failed  = (SELECT IFNULL(SUM(failed), 0)  FROM script WHERE run_id = $run)
           WHERE run_id = $run;"
}

# Opens a script row. `lib.sh` calls this from `start`.
testlog_script_start() {
    local name="$1" subject="${2:-}" expected="${3:-0}" run="${TESTLOG_RUN_ID:-}"

    # Run on its own rather than through run.sh: it still gets a run row, marked as such, so a check
    # recorded outside a full run is never mistaken for one inside it.
    if [ -z "$run" ]; then
        run=$(testlog_run_start 0 "" "single script")
        export TESTLOG_RUN_ID="$run"
        TESTLOG_OWN_RUN=1
    fi
    [ -z "$run" ] && return 0

    # **Claims the row `00-setup` made, or makes one.** A full run has every row already, in file order with its
    # declared count; a single script run on its own has none and this is its insert. The upsert keeps `sequence`
    # as it was placed, so the table reads in file order rather than in the order things happened to start.
    local sequence
    sequence=$(tlog "SELECT IFNULL(MAX(sequence), 0) + 1 FROM script WHERE run_id = $run;")
    tlog "INSERT INTO script (run_id, name, subject, sequence, expected, started_epoch, log_from)
          VALUES ($run, '$(sq "$name")', '$(sq "$subject")', ${sequence:-1}, ${expected:-0},
                  strftime('%s','now'), $(app_log_mark))
          ON CONFLICT (run_id, name) DO UPDATE SET
              subject       = excluded.subject,
              expected      = excluded.expected,
              started_epoch = excluded.started_epoch,
              log_from      = excluded.log_from;"
    TESTLOG_SCRIPT_ID=$(tlog "SELECT script_id FROM script WHERE run_id = $run AND name = '$(sq "$name")';")
    TESTLOG_CHECK_SEQ=0
}

# The trace's debug_log high-water mark, or 0 when there is no database to ask. `DEBUG_DB` rather than `DB`
# since the log moved into its own file (2026-08-22); attaching the app's would find no such table and
# copy nothing, silently, which is the failure this whole record exists to prevent.
app_log_mark() {
    local id
    # The same busy timeout `lib.sh`'s `sql` uses, and for the same reason: a locked read answers empty,
    # which the fallback below turns into a mark of 0 -- and a window starting at 0 copies the whole log
    # into the record instead of this script's own rows.
    id=$(sqlite3 -cmd ".timeout 10000" "$DEBUG_DB" "SELECT IFNULL(MAX(debug_log_id), 0) FROM debug_log;" 2>/dev/null || echo 0)
    printf '%s' "${id:-0}"
}

# Writes a row for every script this run will attempt, before any of them runs.
#
# **So that a script which is never reached is still in the record.** A failing script stops the run, and until
# this every script below it simply had no row: the stamp listed what happened to run and said nothing about the
# fifteen that did not, so a run that died at 07 and a run that ended at 07 looked the same in the table. Now
# each one is there with the count it declares and nothing against it, which is what `passed < expected` means.
#
# Called by `00-setup`, which is the only script that runs before the others. A filtered run does not reach it,
# and gets the old behaviour of a row per script as it starts -- which is right, because the scripts a filter
# excluded were never going to run and should not be listed as though they were.
testlog_prepare_scripts() {
    local run="${TESTLOG_RUN_ID:-}" sequence=0 script name declared
    [ -z "$run" ] && return 0
    for script in Tests/Scripted/[0-9][0-9]-*.sh; do
        [ -e "$script" ] || continue
        sequence=$((sequence + 1))
        name=$(basename "$script" .sh)
        declared=$(sed -n 's/^EXPECTED_CHECKS=\([0-9][0-9]*\)$/\1/p' "$script" | head -1)
        tlog "INSERT OR IGNORE INTO script (run_id, name, sequence, expected, passed, failed)
              VALUES ($run, '$(sq "$name")', $sequence, ${declared:-0}, 0, 0);"
        # Its own row already exists, made by `start`; the insert above ignored it, so the placement is set here.
        tlog "UPDATE script SET sequence = $sequence, expected = CASE WHEN expected = 0 THEN ${declared:-0} ELSE expected END
               WHERE run_id = $run AND name = '$(sq "$name")';"
    done
}

# How many scripts ran fewer checks than they declare. **This is what decides the outcome**: a script that failed
# stops the run and leaves every script below it at nothing against a real expectation, so one question answers
# both "did anything fail" and "did everything run".
testlog_short_scripts() {
    local run="${1:-}"
    [ -z "$run" ] && { printf 0; return 0; }
    printf '%s' "$(tlog "SELECT COUNT(*) FROM script WHERE run_id = $run AND passed < expected;" || echo 0)"
}

# Copies the app's own log rows for this script's window. Safe to call twice: the unique index drops
# what is already there.
testlog_copy_app_log() {
    local run="${TESTLOG_RUN_ID:-}" script="${TESTLOG_SCRIPT_ID:-}"
    [ -z "$run" ] || [ -z "$script" ] && return 0
    local from
    from=$(tlog "SELECT IFNULL(log_from, 0) FROM script WHERE script_id = $script;")
    sqlite3 "$TESTLOG" "
        ATTACH DATABASE '$(sq "$DEBUG_DB")' AS app;
        INSERT OR IGNORE INTO app_log (run_id, script_id, debug_log_id, logged_at, tag, message)
        SELECT $run, $script, debug_log_id, logged_at, tag, message
          FROM app.debug_log WHERE debug_log_id > ${from:-0};
        DETACH DATABASE app;" 2>/dev/null || true
}

# Everything the app logged that no script's window covered, attributed to the run and to no script.
#
# **The end of a run is not the end of the last script.** Each script copies its own window as it finishes, so the
# rows the app writes after that belong to nobody: the cube reconnecting once the checks stop driving it, and the
# quit `run.sh` does on its way out. Measured on run 72 (2026-08-23), that tail was 94 of 2,379 rows -- and it is
# the part covering the shutdown, which is exactly what somebody reads this table for when a run dies at the end.
#
# **From the run's own first mark, not from the last row copied.** `--keep` leaves `debug.sqlite` standing, so its
# ids carry on climbing from the previous run and copying everything would file that run's rows under this one. The
# smallest `log_from` this run recorded is where its own rows start. Everything above it is offered and the unique
# index drops the rest, so this also picks up anything a window happened to miss between two scripts.
testlog_copy_run_tail() {
    local run="${1:-${TESTLOG_RUN_ID:-}}"
    [ -z "$run" ] && return 0
    local from
    # NULL when the run has no scripts at all, which is a run that copied nothing and has no tail either.
    from=$(tlog "SELECT MIN(IFNULL(log_from, 0)) FROM script WHERE run_id = $run;")
    [ -z "$from" ] && return 0
    sqlite3 "$TESTLOG" "
        ATTACH DATABASE '$(sq "$DEBUG_DB")' AS app;
        INSERT OR IGNORE INTO app_log (run_id, script_id, debug_log_id, logged_at, tag, message)
        SELECT $run, NULL, debug_log_id, logged_at, tag, message
          FROM app.debug_log WHERE debug_log_id > $from;
        DETACH DATABASE app;" 2>/dev/null || true
}

testlog_script_finish() {
    local passed="${1:-0}" failed="${2:-0}"
    [ -z "${TESTLOG_SCRIPT_ID:-}" ] && return 0
    testlog_copy_app_log
    tlog "UPDATE script
             SET finished_epoch = strftime('%s','now'),
                 passed = $passed, failed = $failed,
                 human_seconds = ${HUMAN_SECONDS:-0}
           WHERE script_id = $TESTLOG_SCRIPT_ID;"

    # A script run on its own owns its run row, so it closes it too.
    if [ "${TESTLOG_OWN_RUN:-0}" = "1" ]; then
        testlog_run_finish "$TESTLOG_RUN_ID" "$([ "$failed" -eq 0 ] && echo passed || echo failed)" 1
    fi
}

# Records one check. `verdict` is pass, fail or skip.
testlog_check() {
    local verdict="$1" detail="${2:-}"
    [ -z "${TESTLOG_SCRIPT_ID:-}" ] && return 0
    TESTLOG_CHECK_SEQ=$((${TESTLOG_CHECK_SEQ:-0} + 1))

    # Digits to `#`, so a description carrying a clock-stamped name and a fresh row id is still the same
    # check as last run's. Everything cross-run in this file depends on it.
    local key seconds
    key=$(printf '%s' "$LAST" | sed 's/[0-9][0-9]*/#/g')
    seconds=$(( $(date +%s) - ${LAST_STARTED:-$(date +%s)} ))

    tlog "INSERT INTO check_result (script_id, sequence, description, check_key, verdict, detail,
                                    expected, actual, started_epoch, seconds, log_from)
          VALUES ($TESTLOG_SCRIPT_ID, $TESTLOG_CHECK_SEQ, '$(sq "$LAST")', '$(sq "$key")',
                  '$verdict', '$(sq "$detail")', '$(sq "${LAST_EXPECTED:-}")', '$(sq "${LAST_ACTUAL:-}")',
                  ${LAST_STARTED:-0}, $seconds, ${LAST_LOG_MARK:-0});"

    [ "$verdict" = "fail" ] && testlog_failure_evidence
    return 0
}

# What a failure needs and cannot be got back afterwards. Only on failure: the tree costs about a second,
# and the app is usually gone by the time anybody reads this.
testlog_failure_evidence() {
    local check
    check=$(tlog "SELECT MAX(check_id) FROM check_result WHERE script_id = $TESTLOG_SCRIPT_ID;")
    [ -z "$check" ] && return 0

    # First, so a script that dies immediately after still leaves the app's account of what happened.
    testlog_copy_app_log

    local dump
    dump=$(python3 scripts/ax-dump.py 2>&1 | head -400)
    [ -n "$dump" ] && tlog "INSERT INTO evidence (check_id, kind, body)
                            VALUES ($check, 'ax_tree', '$(sq "$dump")');"

    local sheet
    sheet=$(python3 scripts/ax-alert.py 2>&1 | head -20)
    # An alert nobody expected is the usual reason a press went nowhere, and it is invisible in screen.txt.
    [ -n "$sheet" ] && tlog "INSERT INTO evidence (check_id, kind, body)
                             VALUES ($check, 'sheet', '$(sq "$sheet")');"

    # **Which app is in front, and what has the keyboard.** A posted key goes to whoever is frontmost, so
    # a commit that produced no log row and no error is usually a Return that landed somewhere else -- and
    # nothing in the tree or the log says so. This is the only record of it.
    local focus
    focus=$(python3 - 2>&1 <<'PYTHON' | head -5
from AppKit import NSWorkspace
from ApplicationServices import AXUIElementCopyAttributeValue, AXUIElementCreateApplication

def attribute(element, name):
    error, value = AXUIElementCopyAttributeValue(element, name, None)
    return value if error == 0 else None

front = NSWorkspace.sharedWorkspace().frontmostApplication()
print(f"frontmost: {front.localizedName() if front else 'unknown'}")
pid = next(
    (a.processIdentifier() for a in NSWorkspace.sharedWorkspace().runningApplications()
     if a.localizedName() == "Facet"),
    None,
)
if pid is None:
    print("Facet is not running")
else:
    focused = attribute(AXUIElementCreateApplication(pid), "AXFocusedUIElement")
    if focused is None:
        print("nothing in Facet has keyboard focus")
    else:
        print(f"focused: {attribute(focused, 'AXRole')} {attribute(focused, 'AXIdentifier')}")
        print(f"value: {str(attribute(focused, 'AXValue'))[:80]}")
PYTHON
)
    [ -n "$focus" ] && tlog "INSERT INTO evidence (check_id, kind, body)
                             VALUES ($check, 'focus', '$(sq "$focus")');"
    return 0
}

# ---------------------------------------------------------------------------- the committed record

# Writes this machine's stamp -- `last-run-mac.md` or `last-run-linux.md`, whichever `platform.sh` named --
# the one part of this that goes into the repository.
#
# **Named for the platform that produced it.** The scripted suite drives a real app on a real screen, so a
# run says only what works on the machine it ran on. Linux will write `last-run-linux.md` beside this, and
# CI will want both.
#
# **Because CI cannot run the suite and must still be able to tell whether anybody did.** There is no
# screen, no Keychain and no Google account on a build machine, so the only thing that can gate a merge is
# a record of a run that happened somewhere else. `logs/testlog.sqlite` holds far more, but `/logs/` is
# ignored by git and CI never sees it, so the few fields a check needs are written out here.
#
# **Written whether the run passed or failed.** A stamp that only appeared on success would let a failing
# branch keep an older passing one, which is the exact staleness this is for.
#
# It is generated from the recorded run rather than from shell variables, so what it claims is what the
# database saw. See `scripts/check_interactive_checklists.sh` for what is enforced.
testlog_stamp() {
    local run="${1:-}" path="${2:-$STAMP}"
    [ -z "$run" ] && return 0

    local row branch commit dirty rebuilt started finished outcome passed failed filter
    row=$(tlog "SELECT branch, commit_sha, dirty, rebuilt, started_at, IFNULL(finished_at, ''),
                       IFNULL(outcome, ''), IFNULL(passed, 0), IFNULL(failed, 0),
                       IFNULL(filter, '')
                  FROM run WHERE run_id = $run;")
    [ -z "$row" ] && return 0
    IFS='|' read -r branch commit dirty rebuilt started finished outcome passed failed filter <<EOF
$row
EOF

    # **Ran, not listed.** Every script has a row from the moment `00-setup` made them, so counting rows would
    # answer "how many scripts are there" on every run alike. What somebody wants from this line is how far the
    # run got, which is the ones that reached their own finish.
    local scripts_run scripts_listed scripts_failed scripts_short
    scripts_run=$(tlog "SELECT COUNT(*) FROM script WHERE run_id = $run AND finished_epoch IS NOT NULL;")
    scripts_listed=$(tlog "SELECT COUNT(*) FROM script WHERE run_id = $run;")
    scripts_failed=$(tlog "SELECT COUNT(*) FROM script WHERE run_id = $run AND failed > 0;")
    scripts_short=$(tlog "SELECT COUNT(*) FROM script WHERE run_id = $run AND passed < expected;")

    {
        echo "# Scripted suite: last run"
        echo ""
        echo "Written by \`Tests/Scripted/run.sh\` at the end of every run, and committed."
        echo "**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually"
        echo "run, and a stamp that does not describe a real run is worse than no stamp at all."
        echo ""
        echo "    branch:   ${branch}"
        echo "    commit:   ${commit}"
        echo "    tree:     $([ "${dirty:-0}" = "1" ] && echo dirty || echo clean)"
        echo "    database: $([ "${rebuilt:-1}" = "1" ] && echo 'rebuilt from the DDL' || echo 'kept (--keep)')"
        echo "    started:  ${started}"
        echo "    finished: ${finished}"
        echo "    outcome:  ${outcome}"
        echo "    scripts:  ${scripts_run:-0} of ${scripts_listed:-0} run, ${scripts_failed:-0} with failures"
        # The figure the outcome is decided on, written down rather than left to be counted off the table.
        echo "    short:    ${scripts_short:-0} ran fewer checks than they declare"
        # **A filtered run says so, in the stamp rather than only in the database.**
        #
        # `run.sh --filter` runs a subset, and until this line the result was a stamp indistinguishable from a full
        # run: passed, nothing failed, clean tree, right commit. The only trace was the script count being lower than
        # the number of files, and nothing compared those, so a run of one script could stand as evidence for the
        # suite. `check_interactive_checklists.sh` now does compare them; this is the other half, so the file says
        # what happened instead of leaving it to be worked out by arithmetic.
        #
        # Printed only when there was one: a `filter: ` line reading empty on every ordinary run is noise that
        # teaches people to skip the block.
        [ -n "${filter}" ] && echo "    filter:   ${filter}"
        # The total first, because it is the figure somebody scans for: "how much did this run actually check?"
        # A stamp that only gave the breakdown made that an addition somebody had to do in their head.
        #
        # **No skipped line.** Nothing in `Tests/Scripted` has produced a skip since the helpers stopped offering
        # one -- every path answers pass or fail -- so the line read `0 skipped` on every run of every branch, and
        # a number that cannot be anything but zero is one people learn to scroll past. `check_the_suite_was_run`
        # stopped reading it in the same change; if a skip ever comes back, both come back together.
        echo "    checks:   $(( passed + failed )) in total"
        echo "              ${passed} passed"
        echo "              ${failed} failed"
        echo ""
        # **`expected` sits first, before what happened**, because it is the only column in the table that was
        # decided before the run: it is what the script says it does, written down in the script. Everything to
        # the right of it is the answer. A row whose `expected` and `passed` disagree is the case no other column
        # can show -- checks that never ran at all, each of them reporting nothing rather than failing.
        #
        # **`time` is last because it is the only column that judges nothing.** The three before it decide whether
        # the run counts; this one says what it cost, which is the question asked while waiting for the next one --
        # a device script that usually takes two minutes and took nine is worth knowing about long before it starts
        # failing. Minutes and seconds with their units on, rather than `1:47`, which reads as a clock and leaves
        # somebody deciding whether the left-hand number is minutes or hours.
        #
        # It comes off the script row itself, so it measures the script rather than the run: the gaps between them
        # belong to `run.sh`, not to any script, which is why the total below is short of the wall clock in the
        # header.
        #
        # **Time spent waiting for a person is taken out of it and shown in brackets.** Several scripts stop and ask
        # -- a cube to be turned, a radio switched off, a dialog answered -- and until this the answer sat inside the
        # script's own figure, so a script that took two minutes and waited half an hour for somebody to come back to
        # the keyboard was indistinguishable from one that had broken. The figure on the left is now the app's alone,
        # and the bracket is what the run waited on a human for.
        #
        # Empty means the script never reached its own finish, which is what a run stopped part way leaves behind.
        # A dash rather than `0m 00s`, since no time at all and no answer are different things.
        echo "| script | expected | passed | failed | time |"
        echo "|---|---|---|---|---|"
        sqlite3 -noheader -separator '|' "$TESTLOG" \
            "SELECT name, expected, passed, failed,
                    CASE
                        WHEN started_epoch IS NULL OR finished_epoch IS NULL THEN ''
                        ELSE printf('%dm %02ds',
                                    (finished_epoch - started_epoch - human_seconds) / 60,
                                    (finished_epoch - started_epoch - human_seconds) % 60)
                             || CASE WHEN human_seconds > 0
                                     THEN printf(' (%dm %02ds)', human_seconds / 60, human_seconds % 60)
                                     ELSE '' END
                    END
               FROM script WHERE run_id = $run ORDER BY sequence;" \
            2>/dev/null |
            while IFS='|' read -r name e p f t; do
                echo "| $name | $e | $p | $f | ${t:--} |"
            done
        # The totals on the bottom row, so the table adds up to the summary above it rather than asking to be
        # trusted that it does.
        #
        # **The time total adds up the column, and so is less than `finished` minus `started` above.** What is
        # missing is `run.sh`'s own work between scripts -- rebuilding the database, capturing the seeds -- which
        # no script did and none of them should be charged for.
        local script_seconds human_seconds total_time
        script_seconds=$(tlog "SELECT IFNULL(SUM(finished_epoch - started_epoch - human_seconds), 0) FROM script
                                WHERE run_id = $run AND started_epoch IS NOT NULL AND finished_epoch IS NOT NULL;")
        human_seconds=$(tlog "SELECT IFNULL(SUM(human_seconds), 0) FROM script
                               WHERE run_id = $run AND started_epoch IS NOT NULL AND finished_epoch IS NOT NULL;")
        total_time=$(printf '%dm %02ds' "$(( ${script_seconds:-0} / 60 ))" "$(( ${script_seconds:-0} % 60 ))")
        [ "${human_seconds:-0}" -gt 0 ] && total_time="$total_time ($(printf '%dm %02ds' "$(( human_seconds / 60 ))" "$(( human_seconds % 60 ))"))"
        echo "| **total** | **$(tlog "SELECT IFNULL(SUM(expected), 0) FROM script WHERE run_id = $run;")** | **${passed}** | **${failed}** | **${total_time}** |"
        echo ""
        # Said once, under the table, rather than in every row that has one: a bracket with no legend reads as
        # easily as "and another 30 minutes on top" as it does the truth.
        [ "${human_seconds:-0}" -gt 0 ] && echo "A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it." && echo ""
        if [ "${dirty:-0}" = "1" ]; then
            echo "> The working tree had uncommitted changes when this ran, so it is not evidence about the"
            echo "> commit it names. CI refuses a stamp in this state."
            echo ""
        fi
        echo "The full record, including the app's own log rows and the accessibility tree at each failure,"
        echo "is in \`logs/testlog.sqlite\` on the machine that ran it. That file is not in the repository."
    } > "$path" 2>/dev/null || true
}

# ---------------------------------------------------------------------------- reading

testlog_report() {
    local limit="${1:-8}"
    if [ ! -f "$TESTLOG" ]; then
        echo "No log at $TESTLOG yet. It is written by Tests/Scripted/run.sh."
        return 0
    fi
    echo "== the last $limit run(s) =="
    sqlite3 -header -column "$TESTLOG" \
        "SELECT run_id, started_at, branch, dirty, rebuilt, outcome, passed, failed, seconds
           FROM v_run LIMIT $limit;"

    echo ""
    echo "== failures in those runs =="
    sqlite3 -header -column "$TESTLOG" \
        "SELECT run_id, script, description, detail FROM v_failure
          WHERE run_id > (SELECT IFNULL(MAX(run_id), 0) - $limit FROM run);" \
        | head -60

    echo ""
    echo "== never passed, in any run =="
    sqlite3 -header -column "$TESTLOG" "SELECT check_key, failed FROM v_never_passed LIMIT 20;"

    echo ""
    echo "== flaky: passed sometimes, failed others =="
    sqlite3 -header -column "$TESTLOG" "SELECT check_key, passed, failed, last_failed FROM v_flaky LIMIT 20;"

    echo ""
    echo "To read a failure's evidence:"
    echo "  sqlite3 $TESTLOG \"SELECT body FROM evidence WHERE check_id = <id> AND kind = 'ax_tree';\""
    echo "  sqlite3 $TESTLOG \"SELECT logged_at, tag, message FROM app_log WHERE script_id = <id> ORDER BY debug_log_id;\""
}

# Sourced by lib.sh, or run on its own to report.
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    cd "$(dirname "${BASH_SOURCE[0]}")/../.."
    testlog_report "${1:-8}"
fi
