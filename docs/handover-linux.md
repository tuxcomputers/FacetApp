# Handover: for the Linux box

[← Back to README](../README.md) · [The other direction →](handover-mac.md) · [The Linux box →](system-linux.md) · [The Mac →](system-mac.md)

**This file is for the Linux box to act on. Everything in it is written by the Mac.**

Its mirror is [handover-mac.md](handover-mac.md), which the Linux box writes and the Mac acts on. Neither
machine edits the file addressed to itself except to delete from it, and neither deletes from the file it
wrote.

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
   [system-linux.md](system-linux.md); something the hardware does goes in
   [timeflip2-firmware-observations.md](timeflip2-firmware-observations.md); something the port measured
   goes in [port-findings.md](port-findings.md), the D-Bus mechanics in
   [linux-bluez-port-notes.md](linux-bluez-port-notes.md), and something about the shape of the code in
   [architecture.md](architecture.md). This file is the asking, and it is meant to empty.
4. **An item you cannot do stays put, with a line saying why.** That is an answer too, and a more useful
   one than silence, but say it in the item rather than deleting it, so whoever asked can decide what
   to do instead.
5. **Numbers are labels and are never reused.** A gap means an item was finished. A new item takes the
   next number never used before, so a commit message saying "handover 3" still means the same thing
   years later.
6. **When you have done everything you can, write what you want back.** Add items to
   [handover-mac.md](handover-mac.md) for the other machine. A blank file on both sides is the finished
   state.

## What this is not

**Not the system files.** [system-linux.md](system-linux.md) and [system-mac.md](system-mac.md) hold
*facts about a machine*: what compiler, which filesystem, where the data directory resolves. A question
whose answer is a fact belongs there, written into that machine's own file. This file asks for *work*:
build something, run something, decide something.

**Not the open questions in [rust-port.md](rust-port.md).** Those are what the port has not settled yet,
for whichever machine gets to them. This is what the *other* machine is being asked for, which is a much
shorter list and one that empties.

---

## 17. The App tab: build it, try its dialogs, Secret Service and Google on MATE, and convert `08`

**On `feature/appTab`.** The App tab now does everything the Swift one did, Google included, and a lot of it
runs through things only Linux can say work there. The Mac could not compile `facet-linux`, whose `main.rs`
changed throughout.

**What changed under you:**

- **The logger is a `Trace`** (`7dbde1e`): every controller takes `Rc<Trace>`, helpers take any `Record`, and
  the Debug logging box switches recording while the app runs. `CLAUDE.md`'s logger rule says so now.
- **Ports in `facet_core::port`**: `Opener`, `FileChooser`, `SecretStore`, `Http`, `LoopbackListener`.
- **A new crate, `facet-adapters`** (a default member): `rfd` dialogs (XDG portal, no wayland feature),
  `keyring` 4.2, `ureq` 3 with rustls, and a std loopback listener. `LinuxOpener` is in `facet-linux`
  and uses `xdg-open`, opening the folder when revealing a file.
- **Google**: the refresh token goes to the secret store under `au.com.tux.facet.google-refresh`, account
  `refresh-token`. Credentials come from `FACET_GOOGLE_CLIENT_JSON`, then `~/.config/facet/google-client.json`,
  then a copy `scripts/generate-credentials.sh` bundles in.

**What is still wanted.** The build, the Debug buttons, the `08` conversion and a stamped run are done
(`68f69dc`, `08e21df`, and the portal results in [port-findings.md](port-findings.md)), and so is Google:
on 2026-09-27 sign-in, create, rename, delete and Disconnect all ran against a real account here, and
Disconnect left no token in the Secret Service. One part remains:

1. **A stored App setting redraws the menu bar.** `08` proves every row writes through and reads back; nothing
   has yet watched the tray change after a write, such as Show seconds.
