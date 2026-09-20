# The About tab

[← Back to README](../README.md) · [NOTICE →](../NOTICE) · [The Rust port →](rust-port.md)

**The tab is built. The update check on it is not**, and this file is both halves: why the tab has to
exist at all, which is a licence obligation rather than a product decision, and the design question the
update check turns on, which has a wrong answer that is easy to reach by accident.

---

## Why it exists at all

**Because Slint's Royalty-free licence requires attribution**, and this is how Facet provides it. Clause
2 offers two ways, and Facet takes the first:

> (a) Display the `AboutSlint` widget in an "About" screen or dialog that is accessible from the top
> level menu of the Application.

So the tab is a compliance artefact before it is a feature.

**`About Facet` is the first item on the status item's menu**, which is this app's top level menu, since
an accessory application has no menu bar of its own. Choosing it opens the Settings window on the About
tab. Built the other way round first, reachable only by opening Settings and finding the tab, and that
was changed on 2026-09-21: it rested on reading "accessible from" loosely, and a licence condition
should not rest on an argument.

**Clause 2(b) is a fully independent alternative and is worth doing as well.** A Made-with-Slint badge on
the download page at `facet.tux.com.au` satisfies the attribution on its own, whatever the app does, and
costs nothing on a site that is already owned. Doing both means compliance never depends on how anyone
reads clause 2(a). **Not done yet.**

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
