# Can `keyring` reach this machine's Secret Service?

**The evidence behind item 5 of [`docs/handover-linux.md`](../../docs/handover-linux.md), which was a flag
rather than a task: `keyring` 4.2.0 was pinned in the workspace and nothing had ever been built against it,
on either platform.** Answered 2026-09-22 on the Linux box: it works, and it needs less than was assumed.

```sh
cargo run
```

**It writes only under its own service name, `facet-keyring-probe`, and deletes what it wrote.** It does
not touch the `gh` token in the same keyring, which was checked independently afterwards.

## What it proved

| Step | Result |
|---|---|
| `Entry::store_status` | available |
| Entry absent before writing | `NoEntry`, so the read-backs below prove this run wrote them |
| `set_password` / `get_password` | round-tripped `000000`, compared rather than merely fetched |
| `set_secret` / `get_secret` | 5 bytes including a non-UTF-8 pair, byte-clean |
| `delete_credential` | removed it |
| Read after delete | **`NoEntry`**, distinguishable from a failure |

**That last row is the one Facet actually depends on.** The app has to tell *no PIN has ever been stored*
from *the secret store is broken*: the first is an ordinary first run and the second must not be treated as
one. The store makes them different errors, so the app can.

## `libsecret-1-dev` is not load-bearing, and that was assumed the other way

The probe binary links **`libc` and `libgcc_s` and nothing else**. No `libsecret`, no `libdbus`.

```
$ ldd target/debug/keyring-probe
libc.so.6   libgcc_s.so.1
```

`keyring`'s default `v1` feature selects `zbus-secret-service-keyring-store` on Linux, which reaches the
Secret Service over **zbus in pure Rust** through `secret-service` 5.2.0. There is a
`dbus-secret-service-keyring-store` that would use the C library, and it is **not** on.

**So [`system-linux.md`](../../docs/system-linux.md) recorded the wrong reason for keeping that package.**
It is the mirror of the `btleplug` result, which genuinely does link `libdbus-1.so.3`: one crate needed the
system library and the other never did, and neither was knowable without building them. Slint's AccessKit
bridge reaches AT-SPI the same pure-Rust way.

## A locked collection blocks. It does not fail

**Measured 2026-09-22 with the owner present**, and it is the third of the three answers that were on the
table rather than either of the two `system-linux.md` had anticipated.

```sh
cargo run -- store                                  # while unlocked
dbus-send --session --dest=org.freedesktop.secrets --print-reply --type=method_call \
  /org/freedesktop/secrets org.freedesktop.Secret.Service.Lock \
  array:objpath:/org/freedesktop/secrets/collection/login
cargo run -- read                                   # the measurement
# unlock by typing the password into the dialog, then:
cargo run -- delete
```

| | |
|---|---|
| What `read` did against a locked collection | **Nothing, for as long as it was given.** Killed at a 25s cap, exit `124` |
| What appeared instead | A GNOME **Unlock Login Keyring** dialog |
| After the caller was killed | **The dialog stayed on screen**, orphaned from the process that raised it |
| After unlocking | `read` returned `000000` -- the secret survived the cycle intact |

**The expectation was a prompt *or* an error, and the real answer is neither.** It waits. Every operation
in `zbus-secret-service-keyring-store` calls `ensure_unlocked` first, and `secret-service`'s
`ensure_unlocked` runs `exec_prompt` and awaits it. The `ServiceError::Locked => no_access` mapping in that
crate is real but only reached once a prompt has been **refused**; while one is pending there is no error
to map.

**So a background Facet on a locked keyring hangs rather than degrades**, and on a headless machine with no
prompter it would hang with nothing on screen to explain why. **Any read of a stored PIN needs its own
timeout and its own fallback**, and must not sit on the launch path waiting. That is a constraint on the
secret store port rather than a bug in anything here, and it is in
[`port-findings.md`](../../docs/port-findings.md).

**It stays a manual exercise rather than a mode of this probe.** Locking `login` takes the `gh` token out
with it and puts a password dialog in front of whoever is at the screen. A probe should not do that to
somebody who ran it to check a round trip.

## What it did not do, and why

**It measures nothing about macOS.** The same `v1` default selects `apple-native-keyring-store/keychain`
there, target-gated, so the Mac should get the Keychain from the same dependency line -- but *should* is
what this probe exists to replace. It is item 3 in [`handover-mac.md`](../../docs/handover-mac.md).

## What it says about the dependency itself

**`keyring` may be the wrong crate for this app, and its own documentation says so.** From `keyring` 4.2.0's
`lib.rs`:

> Note that *neither* of these modes are either useful for or meant for use by applications which want to
> control which credential stores they use on which platforms [...] Such applications should not be linking
> to this library at all; they should instead be linking to the `keyring-core` library and any specific
> credential stores they want to use.

**That is a description of Facet.** [`CLAUDE.md`](../../CLAUDE.md) requires every platform capability to be
a port: the core states what it needs as a trait and the composition root injects the thing that does it, so
each platform *does* choose its own store. `keyring`'s `v1` feature is the opposite arrangement -- one
platform-independent `Entry` that decides for itself -- which is convenient and is not the shape this app is
built in.

**It is a question for the secret store port, not a reason to change the pin today.** Nothing depends on
`keyring` yet. What this probe establishes is that the underlying store works and how thin the path to it
is: `keyring-core` plus `zbus-secret-service-keyring-store` is two crates, and both are already compiled
here as dependencies of the thing that was pinned.

## What it is not

**Not an adapter and not the start of one.** It is a question asked once and answered, kept because the
answer is worth more with the thing that produced it beside it. **It writes no `debug_log` rows**, not being
the app, so its evidence is its own stdout.
