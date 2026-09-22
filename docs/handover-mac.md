# Handover: for the Mac

[← Back to README](../README.md) · [The other direction →](handover-linux.md) · [The Mac →](system-mac.md) · [The Linux box →](system-linux.md)

**This file is for the Mac to act on. Everything in it was written by the Linux box.**

Its mirror is [handover-linux.md](handover-linux.md), which the Mac writes and the Linux box acts on.
Neither machine edits the file addressed to itself except to delete from it, and neither deletes from the
file it wrote.

**Two developers handing work to each other across a desk, and this is the note left on the keyboard.**
Both machines commit to the same branch under the same identity, so the only way one can ask the other
for something is to write it down where the other will look.

## How to work through this

1. **Take the items in whatever order suits.** They are numbered so a commit message can name one, not
   to say which comes first. Where one genuinely blocks another the item says so.
2. **Delete an item the moment it is done, one at a time, and commit that deletion on its own.** The
   commit message is where the answer goes: what you ran, what came back, what you changed because of
   it. Not struck through, not ticked, not moved to a "done" list: removed. One item, one commit, so the
   history reads as a conversation rather than as a bulk edit.
3. **Put facts where facts live, not here.** Something true about this machine goes in
   [system-mac.md](system-mac.md); something the hardware does goes in
   [timeflip2-firmware-observations.md](timeflip2-firmware-observations.md); something the port measured
   goes in [port-findings.md](port-findings.md), and something about the shape of the code goes in
   [architecture.md](architecture.md). This file is the asking, and it is meant to empty.
4. **An item you cannot do stays put, with a line saying why.** That is an answer too, and a more useful
   one than silence, but say it in the item rather than deleting it, so whoever asked can decide what
   to do instead.
5. **Numbers are labels and are never reused.** A gap means an item was finished. A new item takes the
   next number never used before, so a commit message saying "handover 3" still means the same thing
   years later.
6. **When you have done everything you can, write what you want back.** Add items to
   [handover-linux.md](handover-linux.md) for the other machine. A blank file on both sides is the
   finished state.

---

## 1. Confirm `facet-mac` still builds after the status icon moved

**The only thing being asked, and it is a build rather than a decision.** The menu bar icon is now
`facet_ui::status_icon`, shared, because Linux needed the same pixels and a second copy is how two trays
start disagreeing about what paused looks like. **This box cannot compile `facet-mac`**, so the change was
made without ever being built.

**What moved, and it moved unchanged**: `Showing`, `Rendered`, `render`, the three glyph functions and all
six tests, from `crates/facet-mac/src/status_icon.rs` into `crates/facet-ui/src/status_icon.rs`. The six
tests pass here. Nothing about the drawing was touched, which is deliberate: the move should be reviewable
as a move.

**What stayed**: `crates/facet-mac/src/status_icon.rs` still exists and still has `draw`, because handing
the buffer to tray-icon as an `Icon` is the one genuinely macOS-facing line. It re-exports the three names
above, so **`main.rs` needed no edit at all** and `status_icon::draw`, `status_icon::Showing` and
`status_icon::render` all still resolve.

**One real edit, in `examples/draw-status-icons.rs`.** It used to pull the file in with
`#[path = "../src/status_icon.rs"] mod status_icon;`, which existed only because `facet-mac` is a binary
crate and an example cannot import from one. The drawing is in a library now, so it takes it the ordinary
way: `use facet_ui::status_icon::{self, Showing};`. **That line is the most likely thing to be wrong**,
and `cargo run -p facet-mac --example draw-status-icons` is the check.

So: `cargo build -p facet-mac`, the example, and a look at the menu bar to confirm the icon is what it was.
`with_icon_as_template(false)` still has to stay and is still in `main.rs`; the shared tests assert the
colours are in the buffer, and only that flag makes them survive.

**If it does not build, say so in this item rather than reverting the share.** Linux depends on that
module now, and the fix is almost certainly a line in the example.

## 2. Not a task: `cargo fmt` is not a gate, and this box cannot make it one alone

**Flagged rather than asked, because it is a decision and not work.** `cargo fmt --check` fails on this
tree. Measured on a clean checkout on 2026-09-22, so **it predates the Linux work and is not something
that arrived with it**.

**It is not a formatting lapse.** There is no `rustfmt.toml`, and the house style is wider than rustfmt's
defaults: compact struct literals such as `Showing { paused: false, locked: false }` are on one line
throughout, and rustfmt's `struct_lit_width` of 18 would explode every one. Running `cargo fmt` would
rewrite most of the codebase into a style nobody chose, which is why nobody has run it here.

**The Mac cannot check this at all**, rustfmt not being installed there: `docs/system-mac.md` records it,
and it is why no commit from that side has ever been formatted-checked. **So the honest position is that
the project has no formatting standard it enforces**, and making one means agreeing a `rustfmt.toml` first
and reformatting once, deliberately.

`cargo clippy` is the gate that is actually ready: installed here, exit 0 on `facet-core`, `facet-ui` and
`facet-linux`. It is **not** installed on the Mac either, and CI gates on neither.

## 3. Run the keyring probe, and say what the Mac gets

**A build and a run, and it is the other half of a measurement this box has already taken.**
`probe/keyring-secret-service` round-trips a secret through the Secret Service here and passes every
check. **The same probe should work unchanged on the Mac**, and nobody knows that it does.

```sh
(cd probe/keyring-secret-service && cargo run)
```

**It writes only under its own service name, `facet-keyring-probe`, and deletes what it wrote.** It does
not touch anything else in the keychain. Seven lines of output and an exit code is the whole answer.

**Why it should just work, and why that is not evidence.** `keyring` 4.2.0's default `v1` feature enables
all three platform stores, each target-gated, so a Mac takes `apple-native-keyring-store/keychain` from
the same dependency line that gives this box `zbus-secret-service-keyring-store`. **That is read off a
manifest, which is exactly the kind of reasoning the probe exists to replace**: the equivalent inference
about `libsecret-1-dev` on this box turned out to be wrong in the direction nobody expected.

**The line worth reporting whichever way it goes** is what the binary links. Here it is `libc` and
`libgcc_s` and nothing else, no `libsecret`, because the path is pure Rust over zbus. A Mac linking
Security.framework would be the expected answer and is worth writing down, in
[system-mac.md](system-mac.md) beside the other toolchain facts.

**One more thing worth knowing, and it is the part most likely to differ.** A **locked** collection here
does not return an error: the read **blocks indefinitely** on a GUI prompt, and the prompt outlives the
process that raised it, so a background Facet would hang rather than fall back. Measured 2026-09-22 and
written up in [port-findings.md](port-findings.md). **Whether a locked Keychain does the same to a caller
on macOS is a separate question and should not be assumed to match** -- it decides whether the timeout
that constraint implies is a Linux workaround or a rule for the port. `security lock-keychain` and a
`cargo run -- read` is the shape; the probe has `store`, `read` and `delete` modes for exactly this.

**Also worth your view, and it is a design question rather than a build one.** `keyring`'s own `lib.rs`
says an application that wants to choose its store per platform *"should not be linking to this library
at all"* and should take `keyring-core` plus a specific store. **That describes this app**: CLAUDE.md
makes every platform capability a port, so each platform does choose its own store, and `v1`'s single
platform-independent `Entry` is the opposite arrangement. Nothing depends on `keyring` yet, so this is a
question for the secret store port rather than a change to make now. The probe README sets out both
sides.
