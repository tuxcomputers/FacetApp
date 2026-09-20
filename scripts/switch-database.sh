#!/usr/bin/env bash
# Switches which database the next app launch uses, by repointing the appdata.sqlite symlink:
#
#   scripts/switch-database.sh               swap to whichever database is not in use right now
#   scripts/switch-database.sh test          switch to test.sqlite, kept exactly as it is
#   scripts/switch-database.sh test -clean   switch to a brand new (rebuilt) test.sqlite
#   scripts/switch-database.sh prod          switch back to production.sqlite (no-op if already there)
#   scripts/switch-database.sh -clean        swap, as above, rebuilding what it lands on
#   scripts/switch-database.sh -adopt        make the split: appdata.sqlite -> production.sqlite + a link
#
# test.sqlite is what the scripted suite (Tests/Scripted/) runs against, so it never touches real data;
# production.sqlite is the real one. This script is what creates and moves the symlink -- the app just
# opens appdata.sqlite and lets sqlite resolve it -- and setting.db_type is how a launch says which of
# the two it landed on.
#
# With no target the current symlink decides: on production it switches to test, on test it
# switches back to production.
#
# **Which directory, asked of `Tests/Scripted/platform.sh` rather than spelled out here.** That file is
# where the suite decides what every path is, and this script used to hardcode
# `~/Library/Application Support/Facet` -- so on Linux it did not merely refuse, it resolved every path
# under a directory that does not exist and reported the absence of a database nobody had asked about
# (found 2026-09-20, the first time the suite was pointed at the Linux box).
#
# **Keeping the database is the default, whatever the target and however that target was chosen.**
# Switching is repointing a symlink and nothing else, so running this twice, or running it having
# forgotten which database was in use, moves no recorded data. Destroying a database is the thing that
# has to be asked for by name, which is `-clean`, and it is asked for that way round because it is the
# one of the two that cannot be undone. `prod` is never rebuilt at all -- real data -- so `-clean`
# against it is refused rather than quietly ignored.
set -euo pipefail

usage() {
  local me
  me="$(basename "$0")"
  echo "usage: $me [test|prod] [-clean]" >&2
  echo "  (no target)  swap to whichever database is not currently in use" >&2
  echo "  test         switch to test.sqlite, kept as it is" >&2
  echo "  prod         switch to production.sqlite (does nothing if already there)" >&2
  echo "  -clean       rebuild the database being switched to, from the DDL (test only)" >&2
  echo "  -adopt       rename a plain appdata.sqlite to production.sqlite and link to it" >&2
}

TARGET=""
CLEAN=0   # keeping what is already there is the default, for every target: only -clean rebuilds
ADOPT=0   # making the split out of a plain appdata.sqlite, which is asked for by name -- see below
while [ "$#" -gt 0 ]; do
  case "$1" in
    test|prod)
      if [ -n "$TARGET" ] && [ "$TARGET" != "$1" ]; then
        echo "error: two different targets given ('$TARGET' then '$1') -- pick one." >&2
        usage
        exit 2
      fi
      TARGET="$1"
      ;;
    -clean|--clean) CLEAN=1 ;;
    -adopt|--adopt) ADOPT=1 ;;
    *) echo "error: unknown argument '$1' -- expected 'test', 'prod', '-clean' or '-adopt'." >&2; usage; exit 2 ;;
  esac
  shift
done

# `SUPPORT` and `PROCESS_NAME` come from the suite's own platform file, so that the directory this
# script writes in and the directory the checks read cannot drift apart. Sourced from the repository
# root, which is where every caller runs it from.
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=../Tests/Scripted/platform.sh
source "$HERE/Tests/Scripted/platform.sh" || {
  echo "error: cannot work out which platform this is; see Tests/Scripted/platform.sh." >&2
  exit 2
}

DB_DIR="$SUPPORT"
APPDATA="$DB_DIR/appdata.sqlite"
PRODUCTION="$DB_DIR/production.sqlite"
TEST_DB="$DB_DIR/test.sqlite"

# **A plain file is the shape before the split has ever been made**, which is what a machine that has
# only ever run the app normally has. The Mac's split predates this script and was made by hand; the
# Linux box had never had one (owner, 2026-09-20).
#
# **It used to refuse with advice naming a migration that does not exist.** "Launch the app once with
# Developer Mode on so it can migrate this into production.sqlite + a symlink" describes nothing in
# `FacetCore`, `FacetMac` or `FacetLinux` -- checked 2026-09-20, `production.sqlite` appears in the
# sources only in two comments. So the refusal was unfollowable, and the one machine that hit it could
# do nothing but read the source to find that out.
#
# It still refuses, because renaming somebody's real database is not a thing to do because a test run
# wanted it. But it now says what will actually work, and `-adopt` is that thing asked for by name --
# the same reason `-clean` is opt-in.
if [ -e "$APPDATA" ] && [ ! -L "$APPDATA" ]; then
  if [ "$ADOPT" = "1" ]; then
    if [ -e "$PRODUCTION" ]; then
      echo "error: $PRODUCTION already exists, so adopting $APPDATA would overwrite a database" \
        "that is already there. Move one of them aside by hand and decide which is real." >&2
      exit 1
    fi
    mv "$APPDATA" "$PRODUCTION"
    ln -s "$(basename "$PRODUCTION")" "$APPDATA"
    echo "Adopted: appdata.sqlite was a plain file and is now a link to production.sqlite."
  else
    echo "error: $APPDATA exists but is not a symlink -- refusing to touch it." \
      "It holds real data and this script will not rename it unasked." >&2
    echo "  To make the split, which renames it to production.sqlite and leaves a link in its" >&2
    echo "  place: scripts/switch-database.sh -adopt (add 'test' to switch straight afterwards)." >&2
    exit 1
  fi
fi

# Which database is in use right now, read off the symlink itself -- that is what the next launch
# opens, so it is the thing being switched. Empty when there is no symlink yet, or when it points
# at a file this script doesn't manage; both are only fatal when the target has to be inferred.
CURRENT=""
if [ -L "$APPDATA" ]; then
  linked="$(basename "$(readlink "$APPDATA")")"
  if [ "$linked" = "$(basename "$PRODUCTION")" ]; then
    CURRENT="prod"
  elif [ "$linked" = "$(basename "$TEST_DB")" ]; then
    CURRENT="test"
  fi
fi

if [ -z "$TARGET" ]; then
  if [ -z "$CURRENT" ]; then
    echo "error: can't tell which database is in use (no appdata.sqlite symlink, or it points" \
      "somewhere unexpected), so there is no 'other' one to swap to. Name the target explicitly:" \
      "'test' or 'prod'." >&2
    usage
    exit 1
  fi
  if [ "$CURRENT" = "prod" ]; then TARGET="test"; else TARGET="prod"; fi
  echo "Currently on $CURRENT; switching to $TARGET."
fi

# production.sqlite holds real data: this script only ever relinks to it, and never rebuilds it. So
# `-clean` against prod is a refusal, not something to drop on the floor -- whoever passed it asked for
# an empty database and would otherwise be handed the real one and told the switch worked. Checked
# after the swap default above, so it catches `-clean` on its own while sitting on test just as well as
# `prod -clean`.
if [ "$TARGET" = "prod" ] && [ "$CLEAN" = "1" ]; then
  echo "error: -clean cannot be used with 'prod' -- production.sqlite is real data and this script" \
    "never rebuilds it. Drop -clean to switch to it as it is." >&2
  exit 2
fi

if [ "$TARGET" = "prod" ] && [ "$CURRENT" = "prod" ]; then
  echo "Already on production.sqlite -- nothing to do."
  exit 0
fi

if [ "$TARGET" = "prod" ] && [ ! -e "$PRODUCTION" ]; then
  echo "error: $PRODUCTION does not exist -- this would point appdata.sqlite at a database" \
    "that's never been created. Launch the app once while appdata.sqlite points at" \
    "production.sqlite (the default) so it can be created, then re-run this script." >&2
  exit 1
fi

if [ "$TARGET" = "test" ] && [ ! -e "$APPDATA" ] && [ ! -L "$APPDATA" ]; then
  echo "error: $APPDATA does not exist yet -- launch the app at least once first," \
    "so it can create the symlink and production.sqlite." >&2
  exit 1
fi

if pgrep -x "$PROCESS_NAME" > /dev/null 2>&1; then
  echo "warning: $PROCESS_NAME is currently running -- it already has the old database file open" \
    "and won't see this change until you quit and relaunch it." >&2
fi

if [ "$TARGET" = "prod" ]; then
  TARGET_DB="$PRODUCTION"
else
  TARGET_DB="$TEST_DB"
  # An existing test.sqlite is left exactly as it is by default, so switching to test never costs
  # anything that was recorded there: only the symlink below is (re)pointed at it.
  #
  # `-clean` is what asks for a session that starts from an empty database: the existing test.sqlite
  # (and its WAL/SHM sidecars) is deleted and recreated from the DDL, so nothing carries over. This
  # only ever touches test.sqlite -- production.sqlite is never affected. A test.sqlite that does not
  # exist yet is created either way, since there is nothing to keep.
  if [ "$CLEAN" != "1" ] && [ -e "$TEST_DB" ]; then
    echo "Keeping existing $TEST_DB (recorded test state preserved -- pass -clean to rebuild it)."
  else
    if [ -e "$TEST_DB" ]; then
      echo "Deleting existing $TEST_DB (-clean was passed, so a fresh one is built)..."
      rm -f "$TEST_DB" "$TEST_DB-wal" "$TEST_DB-shm"
    fi
    echo "Creating $TEST_DB..."
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    # Same DDL files, same filename-sorted order AppDataStore.runDatabaseDDL() runs at every launch,
    # with foreign keys enforced during seeding to match the app's own connection. The files hold no
    # live ALTER TABLE (see database/CLAUDE.md: migrations are commented out and run by hand), so
    # every statement here applies cleanly to an empty database. A live one would fail this script
    # under `set -e`, which is the right outcome: it breaks that rule.
    for sql_file in "$SCRIPT_DIR"/database/*.sql; do
      { echo "PRAGMA foreign_keys = ON;"; cat "$sql_file"; } | sqlite3 "$TEST_DB"
    done
    sqlite3 "$TEST_DB" "UPDATE setting SET setting_value = '{\"type\":\"test\"}' WHERE setting_name = 'db_type';"

    # A fresh test.sqlite reads as never-paired, and that is now deliberate: it used to be handed
    # production's `paired`, `device_uuid` and `device_name` so the app could simply connect to the
    # device it was already paired to, rather than spending a scan and a click pairing again against
    # a cube whose PIN an earlier pairing had rotated off the factory default.
    #
    # **Stopped on 2026-08-12, and it was sound until then.** The copy asserts a pairing, and that
    # assertion used to imply which PIN the cube was on: Forget Device reset the device's password to
    # the factory default over `0x30`, so unpaired meant a cube on the default and paired meant a cube
    # on a rotated one. `paired` was a usable proxy for the device's own state.
    #
    # Two changes on this branch removed that. Forget no longer touches the cube's PIN at all -- it is
    # local bookkeeping now, which is the whole point of it, so being unpaired says nothing about what
    # the cube holds. And `02b` Scenario B made the cube's PIN load-bearing for the first time, by
    # testing that a wrong stored PIN cannot reach it. So the proxy can now be wrong in a way nothing
    # in the database can detect: reset the cube by hand and production still says paired, so the fresh
    # database says paired, so `00-test-setup` Step 11 skips the pairing that would have rotated the
    # PIN, and the run proceeds against a cube on the factory default. It even connects, because the
    # reset writes `000000` into `config.json` too and both sides agree -- so the state survives a
    # connection check and surfaces two checklists later as a pairing that succeeded where a refusal
    # was the entire point of the scenario.
    #
    # Pairing from scratch costs a scan and a click, and it establishes by observation what the copy
    # only asserted. It also cannot be wrong about which PIN the cube is on: pairing presents the
    # factory default and then the stored PIN (`PairingPasswordRules`), so a reset cube and a rotated
    # one are both reached, and a reset one is rotated on the way -- leaving the state every checklist
    # after it assumes.
  fi
fi

rm -f "$APPDATA"
ln -s "$(basename "$TARGET_DB")" "$APPDATA"

echo "appdata.sqlite now points at $(basename "$TARGET_DB"). Quit and relaunch the app to pick this up."
