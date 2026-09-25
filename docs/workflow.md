# Workflow: the usage the schema is shaped around

[← Back to README](../README.md) · [Operation Spec](operation-spec.md) · [Database Design](database-design.md)

The *why* behind the schema: what the device owner is trying to do with activities and faces, and which of the app's shapes follow from it. See the [Operation Spec](operation-spec.md) for the "how" (the technical pipeline that turns a device event into a stored record), and [Database Design](database-design.md) for what the rows look like.

**This is not the guide to using the app.** That is at <https://facet.tux.com.au>. What is here is the reasoning a change to the schema or to the tabs has to survive.

## Two kinds of activity

The device is used to track two different kinds of activity:

- **Recurring activities** — things like "Meetings" or "Break" that happen repeatedly and don't need per-occurrence detail beyond when they happened and how long they lasted.
- **Short-lived, ad-hoc activities** — things like an individual JIRA ticket number, which is effectively a one-off category that may only ever be used for a single time entry.

Both are just `category` rows — there's no schema distinction between a "recurring" category and a "short-lived" one, only a difference in how long a given category stays in use.

## Faces map to categories many-to-one

A `category` can have more than one `face` pointing at it — e.g. if `face 3` and `face 6` are both mapped to `category 5` ("Meetings"), then flipping to *either* face 3 or face 6 records a timing segment against the same category. Concretely: a `device_event` row for `device_face = 3` and one for `device_face = 6` both resolve to `category_id = 5` when their `time_entry` rows are created — the resulting `time_entry` rows are indistinguishable by category, and only the underlying `device_face` value (and, transitively, which physical face was flipped) tells them apart.

This is why the day's totals and the `daily_limit` budget are **per category, never per face**. Keying them by face is how it went wrong once: two faces sharing a category each counted alone, so 40 minutes on one and 40 on another left a 60-minute limit unreached.

The app's own faces, 13 and 14, work the same way. They are what it times on when nothing is paired, used in rotation so a finished segment and the one starting are never on the same face — a face reassigned while a finished segment still awaits its `time_entry` would otherwise change the answer under it.

## Setting what a face is for

The gesture is **turn the cube to the face, then click a category** on the Faces tab. A face has no name of its own: it holds a `category_id`, and the category's name, icon and colour are what show for that face. So there is no per-face editing to do — everything a face *displays* is edited on the Categories tab, once, for every face holding that category.

This replaced a pair of buttons the previous app had, **New** and **Rename**, which sat under a text field on the face row and behaved differently depending on whether the typed name already existed elsewhere. The distinction only ever mattered in one case (typing the name of an *existing, different* category: **New** re-pointed the face at it, **Rename** overwrote its name for every face sharing it), and getting the wrong one silently renamed a category across the whole app. Splitting the two questions apart removed it:

- **Which category is on this face** is answered by clicking one in the list. Nothing is typed, so nothing can collide.
- **What a category is called** is answered on the Categories tab, by clicking its name. Every rename is confirmed, and because everything references a category by id, nothing recorded is lost: reports covering time from *before* the rename show the new name too.
- **Making a category that does not exist yet** is the **Create** button, under the list on either tab. The name is decided against the whole `category` table rather than the list on screen, so a name a retired category already holds offers to reactivate it rather than quietly making a second one.

## Locking a face

A face can be **locked**, from the padlock beside the face name on the Faces tab. A locked face keeps what it has: clicking a category no longer assigns to it. It also protects the category from the other end — one on a locked face cannot be retired on the Categories tab, and cannot have its name, icon, colour or daily limit changed either, since half of what a locked face shows lives on the category.
