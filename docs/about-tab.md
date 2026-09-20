# The About tab

[← Back to README](../README.md) · [NOTICE →](../NOTICE) · [The Rust port →](rust-port.md)

**Not built. This is what it is for and what is not decided yet**, written down now because one half of
it is a licence obligation and the other half has a design question with a wrong answer that is easy to
reach by accident.

---

## Why it exists at all

**Because Slint's Royalty-free licence requires attribution**, and this is how Facet provides it. Clause
2 offers two ways, and Facet takes the first:

> (a) Display the `AboutSlint` widget in an "About" screen or dialog that is accessible from the top
> level menu of the Application.

So the tab is a compliance artefact before it is a feature. **It has to exist, be reachable from the
menu, and carry that widget.** The alternative, clause 2(b), is a Made-with-Slint badge on the download
page at `facet.tux.com.au`, which would cost nothing and is the fallback if the tab is ever dropped.
Doing both is allowed and is cheap insurance.

The rest of the tab is ordinary: version, licence, a link to the source, acknowledgements. The
acknowledgements are not decoration either. The icon permission is per-project and does not transfer,
and [`NOTICE`](../NOTICE) records that.

**A scripted check should assert the widget is present.** A licence condition nothing verifies is a
licence condition that quietly lapses across a refactor.

---

## The update check, and what is not decided

**Wanted: the app tells the user when a newer version exists. How is open.**

The design question that matters is not where to fetch from, it is this: **an update check is the first
thing Facet does over the network that the user did not ask for.** The Google connection is opt-in and
obviously so. A version check is not, and it necessarily discloses to whoever serves it that this
machine is running Facet, at this version, at this time, from this IP.

**So it needs a setting and a line of plain English next to it**, and the honest default is a question
rather than an assumption. Everything else below is downstream of that.

### Where to check

| | |
|---|---|
| **GitHub Releases** | `GET /repos/tuxcomputers/FacetApp/releases/latest`. Nothing to host, and the tag is already the source of truth. Unauthenticated calls are rate limited per IP, which is ample for one desktop app and can be hit by many users behind one NAT |
| **A static file on `facet.tux.com.au`** | Owned outright, no rate limit, and it can carry more than a version string: a minimum supported version, or a note for a release that needs manual steps. One more thing to remember to update |

Neither is measured. **The tag is the source of truth either way**, so the static file, if chosen, is
published by the release process rather than edited by hand.

### Rules it has to follow, which are not new rules

- **A failed check is logged, never raised.** An alert saying the update check failed is worse than the
  problem it reports. This is *Nothing fails silently* with its other half applied: the failure is
  recorded, and it is recorded where a check can read it, which is `debug_log`.
- **Whether to check, and when it last checked, are `setting` rows**, read at the point of use like
  everything else. Nothing caches the answer in memory across the question.
- **The comparison is on the version, not on the tag string.** A string compare puts 0.10.0 before
  0.9.0.
- **Check and tell. Do not download, and do not self-update.** Auto-update is a far larger commitment
  than it looks: code signing and notarisation on macOS, a signature scheme the app verifies, a rollback
  story, and a per-platform delivery mechanism. It is a separate decision, not a natural extension of
  this one.

### Open, and worth settling before any of it is written

1. **Default on or off?** On is more useful and is a surprise. Off is honest and most users never find
   it. A first-run prompt is the third answer and costs a first-run prompt.
2. **How often**, and what happens on a machine that is offline for a month.
3. **Whether the check runs at launch or on opening this tab.** On opening is the least surprising and
   the least useful.
4. **Whether a pre-release tag counts.** GitHub's `releases/latest` excludes them; a static file would
   have to decide.
