#!/usr/bin/env bash
# Checks that the scripted suite was run, and passed, on both the Mac and the Linux box, for the code in this
# branch. CI runs it on every pull request; it cannot run the suite itself.
#
#   scripts/check-scripted-stamps.sh --branch feature/faceTab
#
# Every numbered script in Tests/Scripted must parse, be executable, call `finish`, guard the database with
# `require_test_database` and declare `EXPECTED_CHECKS`. Then each of `Tests/Scripted/last-run-mac.md` and
# `Tests/Scripted/last-run-linux.md` must record a run that:
#
#   - passed, with no failing check;
#   - ran every numbered script present, and in each one exactly the checks it declares;
#   - ran on a clean tree;
#   - with --branch: ran on that branch, at a commit in this branch's history, after which nothing under
#     crates/, Tests/Scripted/ (stamps and other Markdown aside), Cargo.toml or Cargo.lock has changed.
#
# Without --branch (a push to main) the branch, history and staleness rules are skipped. Staleness needs the
# full history, so the workflow checks out with fetch-depth: 0. Exits 1 when any rule fails, 2 on bad usage.

set -euo pipefail

BRANCH=""
while [ $# -gt 0 ]; do
    case "$1" in
        --branch) BRANCH="${2:-}"; shift 2 || shift ;;
        --branch=*) BRANCH="${1#--branch=}"; shift ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

WATCHED=(crates Tests/Scripted Cargo.toml Cargo.lock ":!Tests/Scripted/*.md")

stamp_field() {
    sed -n "s/^ *$1: *//p" "$STAMP" | head -1
}

# check_stamp <platform> <stamp file> <number of scripts here>
check_stamp() {
    local platform="$1" on_disk="$3"
    local STAMP="$2"
    echo ""
    echo "The $platform, from $STAMP:"
    if [ ! -f "$STAMP" ]; then
        echo "  no stamp. Run Tests/Scripted/run.sh on the $platform and commit the stamp it writes."
        return 1
    fi

    local ran_branch ran_commit tree outcome failed_checks scripts_ran short_scripts count_trouble problems=""
    ran_branch=$(stamp_field branch)
    ran_commit=$(stamp_field commit)
    tree=$(stamp_field tree)
    outcome=$(stamp_field outcome)
    failed_checks=$(awk '/^ *checks:/ { inblock = 1 }
                         inblock && /^ *[0-9]+ failed *$/ { print $1; exit }' "$STAMP")
    scripts_ran=$(sed -n 's/^ *scripts: *\([0-9]*\) of .*/\1/p' "$STAMP" | head -1)
    short_scripts=$(sed -n 's/^ *short: *\([0-9]*\) .*/\1/p' "$STAMP" | head -1)
    echo "  ran on $ran_branch at ${ran_commit:0:12}: $outcome, ${failed_checks:-?} failed, ${scripts_ran:-?} of $on_disk scripts"

    if [ -z "$scripts_ran" ]; then
        problems="$problems
  - the stamp does not say how many scripts ran"
    elif [ "$scripts_ran" -lt "$on_disk" ]; then
        problems="$problems
  - it ran $scripts_ran of the $on_disk numbered scripts here"
    fi
    [ "$outcome" = "passed" ] || problems="$problems
  - the run did not pass (outcome: ${outcome:-unknown})"
    [ "${failed_checks:-1}" = "0" ] || problems="$problems
  - the run had ${failed_checks:-an unknown number of} failing check(s)"
    count_trouble=$(awk -F' *[|] *' '
        /^\| / && $2 !~ /\*\*/ && $2 != "script" && $2 !~ /^-+$/ && $3 + 0 != $4 + 0 {
            printf "\n      %s: declares %s check(s), passed %s", $2, $3, $4
        }' "$STAMP")
    [ -z "$count_trouble" ] || problems="$problems
  - a script did not pass every check it declares:$count_trouble"
    [ "${short_scripts:-1}" = "0" ] || problems="$problems
  - ${short_scripts:-an unknown number of} script(s) ran fewer checks than they declare"
    [ "$tree" = "clean" ] || problems="$problems
  - the working tree was ${tree:-unrecorded} when it ran, so the run is not evidence about that commit"

    if [ -z "$BRANCH" ]; then
        echo "  no branch given, so the branch and staleness rules are skipped"
    else
        [ "$ran_branch" = "$BRANCH" ] || problems="$problems
  - it ran on $ran_branch, not on $BRANCH"
        if [ -z "$ran_commit" ] || ! git cat-file -e "$ran_commit^{commit}" 2>/dev/null; then
            problems="$problems
  - commit ${ran_commit:-(none)} is not in this checkout (the workflow needs fetch-depth: 0)"
        else
            git merge-base --is-ancestor "$ran_commit" HEAD || problems="$problems
  - ${ran_commit:0:12} is not in this branch's history, so the run was of different code"
            if ! git diff --quiet "$ran_commit" HEAD -- "${WATCHED[@]}"; then
                problems="$problems
  - the app or the checks have changed since that run:
$(git diff --name-only "$ran_commit" HEAD -- "${WATCHED[@]}" | sed 's/^/      /' | head -20)"
            fi
        fi
    fi

    if [ -n "$problems" ]; then
        echo "  not a run of this branch as it stands:$problems"
        echo "  Run Tests/Scripted/run.sh on the $platform and commit the stamp it writes."
        return 1
    fi
    echo "  passed, and is a run of this branch as it stands"
}

shopt -s nullglob
scripts=(Tests/Scripted/[0-9][0-9]-*.sh)
if [ ${#scripts[@]} -eq 0 ]; then
    echo "No numbered scripts in Tests/Scripted; nothing to check."
    exit 0
fi

echo "The ${#scripts[@]} numbered script(s), and the files they source:"
runnable=0
for f in Tests/Scripted/platform.sh Tests/Scripted/lib.sh Tests/Scripted/run.sh "${scripts[@]}"; do
    problems=""
    bash -n "$f" 2>/dev/null || problems="$problems does-not-parse"
    [ -x "$f" ] || problems="$problems not-executable"
    case "$f" in
        Tests/Scripted/[0-9][0-9]-*.sh)
            grep -q '^finish$' "$f" || problems="$problems no-finish"
            grep -q 'require_test_database' "$f" || problems="$problems no-database-guard"
            grep -qE '^EXPECTED_CHECKS=[1-9][0-9]*$' "$f" || problems="$problems no-expected-checks"
            ;;
    esac
    if [ -n "$problems" ]; then
        echo "  $f:$problems"
        runnable=1
    else
        echo "  $f ok"
    fi
done

gate=$runnable
check_stamp "Mac" "Tests/Scripted/last-run-mac.md" "${#scripts[@]}" || gate=1
check_stamp "Linux box" "Tests/Scripted/last-run-linux.md" "${#scripts[@]}" || gate=1

echo ""
if [ "$gate" -ne 0 ]; then
    echo "The scripted suite has not been run and passed on both machines for this branch as it stands."
    exit 1
fi
echo "The scripted suite was run and passed on both machines for this branch as it stands."
