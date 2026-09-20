#!/usr/bin/env bash
# Reports how a database differs from what database/*.sql would create.
#
# Migrations are not applied automatically (see database/CLAUDE.md: an ALTER TABLE is written
# commented out and run by hand until the 099-script + database_version feature exists), so an
# existing database drifts behind the DDL every time a column is added. This says by how much, and
# prints the statements that would close the gap.
#
#   scripts/compare-database-to-ddl.sh                    # production.sqlite
#   scripts/compare-database-to-ddl.sh path/to/other.db
#   scripts/compare-database-to-ddl.sh ~/Library/Application\ Support/Facet/debug.sqlite
#
# **Which half of the DDL is the reference depends on the target.** One directory holds two schemas --
# below 500 the app's, 500 and above the trace's (DatabaseBootstrap.firstDebugDDLNumber) -- so a
# reference built from all of it would report `debug_log` missing from production.sqlite and every app
# table missing from debug.sqlite. A target named `debug*.sqlite` is compared against the 500 files, and
# anything else against the rest.
#
# The target is only ever read. Exits 0 when it matches the DDL, 1 when it does not.
set -euo pipefail

TARGET="${1:-$HOME/Library/Application Support/Facet/production.sqlite}"
if [ ! -f "$TARGET" ]; then
  echo "error: no such database: $TARGET" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
REFERENCE="$WORK/reference.sqlite"

# The reference is what a brand-new database looks like: every DDL file this target owns, in the order
# the app runs them (DatabaseBootstrap.ensureDatabase, filename-sorted).
case "$(basename "$TARGET")" in
  debug*) WANT_DEBUG=1 ;;
  *)      WANT_DEBUG=0 ;;
esac

for sql_file in "$REPO_ROOT"/database/*.sql; do
  number="$(basename "$sql_file")"
  number="${number:0:3}"
  if [ "$WANT_DEBUG" = "1" ]; then
    [ "$number" -ge 500 ] 2>/dev/null || continue
  else
    [ "$number" -lt 500 ] 2>/dev/null || continue
  fi
  { echo "PRAGMA foreign_keys = ON;"; cat "$sql_file"; } | sqlite3 "$REFERENCE"
done

# Read the target read-only, so inspecting a live database can never alter it.
target_sql() { sqlite3 -readonly "$TARGET" "$1"; }
reference_sql() { sqlite3 "$REFERENCE" "$1"; }

TABLES_Q="SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;"
# One line per column: name, declared type, NOT NULL, default. Position is deliberately excluded:
# a hand-run ALTER appends the column at the end, so an otherwise-identical table would otherwise
# read as different purely because of column order.
columns_q() {
  printf "SELECT name || '  ' || type || \
    CASE WHEN \"notnull\" = 1 THEN ' NOT NULL' ELSE '' END || \
    CASE WHEN dflt_value IS NOT NULL THEN ' DEFAULT ' || dflt_value ELSE '' END \
    FROM pragma_table_info('%s') ORDER BY name;" "$1"
}
INDEXES_Q="SELECT name || '  on ' || tbl_name FROM sqlite_master WHERE type='index' \
  AND name NOT LIKE 'sqlite_%' ORDER BY name;"

differences=0
note() { differences=$((differences + 1)); printf '%s\n' "$1"; }

echo "Comparing:"
echo "  database: $TARGET"
echo "  against:  $REPO_ROOT/database/*.sql"
echo

# --- tables ------------------------------------------------------------------------------------
reference_sql "$TABLES_Q" > "$WORK/ref_tables"
target_sql "$TABLES_Q" > "$WORK/tgt_tables"

while read -r table; do
  [ -n "$table" ] && note "MISSING TABLE   $table"
done < <(comm -23 "$WORK/ref_tables" "$WORK/tgt_tables")

while read -r table; do
  [ -n "$table" ] && note "EXTRA TABLE     $table (in the database, not in the DDL)"
done < <(comm -13 "$WORK/ref_tables" "$WORK/tgt_tables")

# --- columns, for tables both sides have ---------------------------------------------------------
while read -r table; do
  [ -z "$table" ] && continue
  reference_sql "$(columns_q "$table")" > "$WORK/ref_cols"
  target_sql "$(columns_q "$table")" > "$WORK/tgt_cols"

  while read -r col; do
    [ -z "$col" ] && continue
    name="${col%% *}"
    # A column whose definition merely changed shows up on both sides of the comparison; leave it
    # to the "differs" pass below rather than also calling it missing.
    grep -q "^$name  " "$WORK/tgt_cols" && continue
    # Prefer the migration the DDL file already records for this column, so the statement printed
    # is the one its author wrote, CHECK constraints and all.
    recorded="$(grep -h -m1 -i -- "^-- ALTER TABLE $table ADD COLUMN $name\b" "$REPO_ROOT"/database/*.sql 2>/dev/null | sed 's/^-- //' || true)"
    if [ -n "$recorded" ]; then
      note "MISSING COLUMN  $table.$name -- run:  $recorded"
    else
      note "MISSING COLUMN  $table.$name ($col) -- no recorded migration in database/*.sql"
    fi
  done < <(comm -23 "$WORK/ref_cols" "$WORK/tgt_cols")

  while read -r col; do
    [ -z "$col" ] && continue
    name="${col%% *}"
    if grep -q "^$name  " "$WORK/ref_cols"; then
      note "COLUMN DIFFERS  $table.$name"
      note "                  DDL says:      $(grep -m1 "^$name  " "$WORK/ref_cols")"
      note "                  database has:  $col"
    else
      note "EXTRA COLUMN    $table.$name (in the database, not in the DDL)"
    fi
  done < <(comm -13 "$WORK/ref_cols" "$WORK/tgt_cols")
done < <(comm -12 "$WORK/ref_tables" "$WORK/tgt_tables")

# --- indexes -------------------------------------------------------------------------------------
reference_sql "$INDEXES_Q" > "$WORK/ref_idx"
target_sql "$INDEXES_Q" > "$WORK/tgt_idx"

while read -r idx; do
  [ -n "$idx" ] && note "MISSING INDEX   $idx"
done < <(comm -23 "$WORK/ref_idx" "$WORK/tgt_idx")

while read -r idx; do
  [ -n "$idx" ] && note "EXTRA INDEX     $idx (in the database, not in the DDL)"
done < <(comm -13 "$WORK/ref_idx" "$WORK/tgt_idx")

echo
if [ "$differences" -eq 0 ]; then
  echo "No differences: the database matches the DDL."
else
  echo "$differences difference(s). Nothing was changed; run the statements above yourself."
  exit 1
fi
