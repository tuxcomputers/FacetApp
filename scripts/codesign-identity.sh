#!/bin/sh
# Prints the codesigning identity to build with, or nothing when there is none.
#
# **One place, because more than one thing builds the app** -- `scripts/run.sh` and
# `Tests/Scripted/platform.sh` -- and a build that misses the identity is not obviously broken. Cargo
# leaves a linker-applied ad-hoc signature, whose designated requirement is the cdhash of the binary, so
# every rebuild is a different application as far as the Keychain is concerned and the permission granted
# to the last one matches nothing. A certificate makes the requirement an identifier and an anchor with
# no hash in it, which is the same for every build, so one Always Allow holds.
#
# In the Swift app that cost a real debugging session: an unsigned build could not read the refresh token
# behind Google sync, and nothing said so. It was not a build error or a test failure, the sweep simply
# never ran (measured 2026-08-16).
#
# `FACET_CODESIGN_IDENTITY` overrides the search, for anyone holding more than one.
# See docs/google-oauth-setup.md for what to do when there is no identity at all.
if [ -n "${FACET_CODESIGN_IDENTITY:-}" ]; then
    printf '%s' "$FACET_CODESIGN_IDENTITY"
    exit 0
fi

# **Developer ID first, wherever there is one.** It is the only kind Apple will notarize, so a machine
# holding both has to reach for it when scripts/package.sh builds something to send somebody; picking
# whichever the listing happened to print first would produce an un-notarizable image and say nothing.
# Apple Development is the fallback, which is what a developer machine ordinarily has and is enough for
# everything except distribution.
identities="$(security find-identity -v -p codesigning 2>/dev/null)"

found="$(printf '%s\n' "$identities" | grep -E '"Developer ID Application:' | head -1)"
if [ -z "$found" ]; then
    found="$(printf '%s\n' "$identities" | grep -E '"Apple Development:' | head -1)"
fi

printf '%s' "$found" | sed -E 's/.*"(.*)".*/\1/' | tr -d '\n'
