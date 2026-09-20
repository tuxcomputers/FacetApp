# Google OAuth: setting it up once, so nobody else has to

[← Back to README](../README.md) · [Behaviour inventory →](behaviour-inventory.md)

**Who this is for: whoever publishes the binary.** An earlier version of this app asked every user to
create a Google Cloud project, configure a consent screen, mint an OAuth client and paste two strings in.
That is five steps of setup before the app does anything, and every one of them is a place to give up.
This is what replaced it: **one project, owned by you, whose client ID ships inside the app.**

**Part 1 is about Google and is entirely language-independent.** It was done once, it is done now, and
none of it has to be repeated for the Rust build. **Part 2 is what the app has to do**, restated as
requirements rather than as the Swift implementation that satisfied them; the Swift version is in the
frozen TimeFlipApp repository at `docs/google-oauth-setup.md` on `feature/linuxPort`, alongside
`GoogleOAuthRules` and `GoogleOAuthClient`.

**Google's console moves.** The tabs have been reorganised at least twice and scope classifications
change. Where this names a click path, trust the intent over the wording, and trust **the tier the console
shows against a scope** over anything written here. That instruction has already earned its place once.

---

## The scope list, and why it turned out to be the whole ballgame

**Settled 2026-08-15. All four scopes are non-sensitive**, confirmed against the console.

| Scope | What it buys |
|---|---|
| `openid` | Associate the user with their Google identity |
| `userinfo.email` | The account's email address, to show who is connected |
| `userinfo.profile` | The account's name and picture, same purpose |
| `calendar.app.created` | Make secondary calendars, and manage events on the ones **this app made** |

**Two scopes were deliberately dropped, and both are sensitive**: `calendar.events` and
`calendar.readonly`. They exist for one feature, **choosing an existing calendar to sync into**. Dropping
it, and always creating and owning a "Facet" calendar, is what keeps the whole app in the non-sensitive
tier. It costs the user the ability to put Facet events on a calendar they already share.

**That single trade is worth more than everything else here**, because it is the difference between
publishing and being reviewed. **Adding either one back later is not a small change**: it moves the app
into the review process from a standing start, which is video, justifications and weeks. Treat that
feature as a decision with a price attached rather than as a later enhancement.

---

## Part 1: the Google side

**Already done. This is the record of what was set up, so it can be understood, audited or redone.**

### 1. One project, owned by you

Google Cloud console, **New Project**. Name it for the product, not for a machine. Own it from an account
that will outlive the release and that you can add a second owner to: **if the project is lost, every
installed copy of the app loses its ability to sign in.**

**The project is `facet-505603`**, created 2026-08-15, with a Desktop OAuth client in it. Two earlier
half-configured projects were retired, which matters more than it sounds: several projects that all nearly
work is how the wrong client ID reaches a release.

**Its credentials live at `~/.config/facet/google-client.json`, outside every repository.** The file is
the console's download unchanged. Its top-level key is `installed`, which is what confirms a Desktop
client rather than a web one, and the only two values the app needs are `client_id` and `client_secret`.
**Do not hardcode the `auth_uri` and `token_uri` beside them**: read them from the discovery document,
which is what survives Google moving one. `redirect_uris` says `http://localhost`, a placeholder to
ignore, since the loopback port is chosen at runtime.

### 2. Enable the Calendar API

**APIs & Services → Library → Google Calendar API → Enable.** Nothing else. Every API enabled on the
project is another thing a reviewer asks about.

### 3. Fill in the consent screen

- **User type: External.** Internal only exists for a Workspace organisation and would mean only that
  org's accounts could sign in.
- **App name: `Facet`.** This is what the user reads in "Facet wants access to your Google Account", so it
  must match the name on the homepage and must not imply Google made it. **Not** the OAuth *client* name
  from step 5, which nobody outside the console sees.
- **Support email and developer contact** are public, and are where consent-screen complaints land.
- **App logo: leave it empty.** This is the one field that costs something. **Uploading a logo is by
  itself enough to put the app into verification, whatever the scopes are.** The artwork exists
  (`facet-logo-120.png` in the site repository) for whenever that trade is worth making.

### 4. Add the scopes

**Data access → Add or remove scopes.** Exactly the four above. **Note the tier the console puts against
each one**; that label, not this file, decides whether step 6 applies.

### 5. Create the OAuth client

**Clients → Create OAuth client → Desktop app.** Name it for the console list rather than for the product,
since it is internal: `Facet macOS (desktop)` beats `Facet`.

**Why Desktop and not iOS**, whose description also covers macOS. An iOS client issues no client secret at
all, which sounds safer for a binary anybody can open. Three things outweigh it:

- **A Desktop client is not keyed to the bundle identifier**, so renaming the app does not mean recreating
  the client.
- **The loopback flow is what this app implements.** The iOS path means a registered URL scheme and an
  open-URL handler instead.
- **Custom URI schemes are weak on macOS specifically.** Any installed app can claim the same scheme and
  the OS picks a winner. A port your own process is listening on is more predictable.

**The secret is not a secret, and neither type gives real client authentication.** Google's installed-app
model explicitly does not treat a desktop client secret as confidential, and a bundle identifier is
equally forgeable for a binary distributed outside an app store. **PKCE is what actually protects the
exchange.**

So: **the client ID and the secret are both extractable from the binary**, that is accepted for installed
apps, and it cannot be prevented. In practice somebody could stand up a different application showing your
app's name on its consent screen. PKCE stops them intercepting *your* users' codes; nothing stops the
impersonation itself.

### 6. Publish

**Move the publishing status from Testing to In production.** Testing is not a soft launch: it is capped
at 100 named test users, and **refresh tokens issued under it expire after seven days**, so every user
would be silently signed out weekly. That expiry alone is the reason to move even while the app is only
being tried out.

**Three things put an app into verification, and they are all yours to avoid**: more than 10 domains, a
logo, or sensitive or restricted scopes. Facet has one domain and four non-sensitive scopes, **so the logo
is the only one in play**, and leaving it unset keeps the app out of review altogether.

**Verified is not worth wanting on its own.** What verification removes is the "Google hasn't verified this
app" interstitial and the roughly 100 user ceiling, and both are consequences of asking for sensitive
scopes while unverified. Trip none of the three triggers and neither exists. The choice is not "verified
or not", it is "logo or no review". **What no logo costs is a consent screen without a custom icon. That
is all.**

The homepage and privacy policy remain required consent-screen fields and are live at `facet.tux.com.au`.
**If the logo is ever added**, the review it triggers wants domain ownership verified in Search Console
**under the same Google account that owns the Cloud project**. That is the step that goes wrong quietly:
verifying under one account and creating the project under another leaves both looking complete and the
submission rejected.

**What was never needed**: a third-party security assessment. CASA applies to *restricted* scopes, which
are Gmail and full Drive.

### 7. What you have taken on

- **Quota is yours.** Every user's calls count against this project. Volume is not the concern; **a single
  abusive user getting the project rate-limited or suspended is**, because it takes everybody's sync down.
- **The support email is yours**, in front of every user.
- **The 10-domain ceiling** is worth remembering rather than checking. One domain is a long way from it,
  but it is a trigger that could be tripped absent-mindedly.

---

## Part 2: what the app must do

**Requirements, with the reasoning that was paid for.** The Swift implementation satisfied all of them and
had run against a real account.

### Ship the client ID and secret with the build

**Three sources, in order: an environment variable, then `~/.config/facet/google-client.json`, then what
the build put in.** The first two are files on one machine; **only the third travels with the binary**,
which is why it exists. The override earns its place twice over: it allows testing against a second
project without a release build, and it is a way out if the bundled project is ever suspended.

`scripts/generate-credentials.sh` is what fills the bundled copy, and it carries over. **It copies the
console's download verbatim rather than rewriting it**, so the bundled copy and the two overrides are one
format read by one parser rather than two that can drift.

**No credentials is not an error.** A fork with no Google project must build and run everything else, with
the App tab saying *"This copy of Facet was built without Google credentials, so it cannot sign in."* That
is also CI's case, so a plain build needs no secret.

**Two traps from the Swift build, both measured, both worth knowing before choosing a mechanism here:**

- **A build-time existence check failed silently.** The first version was a generated source file with the
  manifest checking whether it existed and defining a flag when it did. **The manifest was cached**, so the
  check did not re-run after the generator created the file: generator ran, build succeeded, flag absent,
  credentials quietly not in the binary, nothing anywhere saying so. In Rust the equivalent hazard is a
  `build.rs` whose rerun conditions do not name the generated file. **Declare the dependency explicitly.**
- **Removing credentials did not prune the build.** Deleting the source file left the copy in the build
  directory, so a build went on carrying a client somebody had just taken away. **The generator clears
  them itself** rather than trusting the build system to notice.

### No paste-in fields

There are no Client ID or Client Secret fields anywhere in the app. That was the whole point.

### The loopback redirect, and own it

The client is a Desktop one, so the redirect is `http://127.0.0.1:<port>`. No custom URI scheme anywhere.

**Write the listener rather than depending on an OAuth framework.** The Swift app did, and the reasoning
holds in Rust: the flow for an installed app is small enough that owning it is cheaper than depending on
something built around a different platform's idioms, and it makes every decision in it ordinary code with
tests on it. The listener is a **port**, because it is the one part that touches the platform's
networking.

**The port is whatever the system gives, never fixed.** A hardcoded port is a sign-in that fails whenever
something else already holds it, and Google accepts any port on the loopback address precisely so it does
not have to be registered.

**PKCE is on**: 32 random bytes base64url-encoded to a 43-character verifier, the shortest RFC 7636
allows, with its S256 challenge. It is what actually protects the exchange, given the client secret ships
in the binary. **A `state` value is echoed and checked too**, so a redirect that did not come from this
process's own request can be told apart from one that did.

`sha2` replaces the hand-written SHA-256 the Swift version needed because CryptoKit is not portable.

### One item in the secret store

**The refresh token, and nothing else.** The client secret is configuration, not a stored credential, so
there is no second store to keep. This goes through the secrets port: Keychain, Secret Service, Credential
Manager.

**On macOS, codesigning is what makes it survive a rebuild**, and this cost a real debugging session. See
the last section.

### Fail honestly when the project is the problem

A suspended project, a revoked client and a quota ceiling all come back as an auth or API error rather
than as anything a user can act on. **Name them in words**, including the one that is easiest to misread:
**the secret store refusing to hand over the token is not the same as not being signed in.**

### Changing the scope list under an existing user

A token issued before the list changed does not gain new scopes on its own, and the answer is sign out and
sign in again. It matters less than it did, the list having been stable since 2026-08-15.

---

## Why macOS asks for Keychain access after every rebuild

**Not specific to any language, and it will happen again the first time a Rust build writes a token.**

The Keychain grants access to *an application*, identified by its code signature. **For an ad-hoc signed
build that identity is the cdhash of the binary**, so every rebuild is a different application as far as
the Keychain is concerned. Clicking **Always Allow** works exactly as advertised; it records permission for
a binary that no longer exists after the next build.

A real certificate changes what the permission is recorded against. The designated requirement becomes
an identifier and an anchor with **no hash in it**, so it is the same for every build and the answer
holds.

**The setup, once:**

1. Xcode → Settings → Accounts → add an Apple ID (a free account is enough) → **Manage Certificates…** →
   **+** → **Apple Development**.
2. Check it is usable: `security find-identity -v -p codesigning` must list it.
3. Sign the run with that identity. Answer **Always Allow** to the one prompt that follows, because the
   identity has changed one last time.

**If step 2 says `0 valid identities found` while Xcode clearly shows the certificate**, the chain cannot
be built and the certificate is therefore not a usable identity. Measured 2026-08-15: the only WWDR
intermediate installed was the original one, which **expired on 2023-02-07**, while the certificate is
issued by WWDR **G3**. Installing the current intermediate fixes it.

**The failure mode with no certificate is not an error**: sync simply never runs. Measured 2026-08-16,
when an unsigned build replaced a signed one and the scripted checks found the sweep silent.
