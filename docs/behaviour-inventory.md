# Behaviour inventory: what the Swift suite pins down

[← Back to README](../README.md) · [Architecture →](architecture.md) · [Scripted suite →](scripted-suite.md) · [Operation spec →](operation-spec.md)

**A map of the rules the Rust port has to reproduce, and where to read each one in full.**

The Swift app was covered by **134 test files carrying 2,063 test declarations**, measured on
`feature/linuxPort` at `6dd0045`. Almost none of that is about Swift. It is rounding, ordering, range
arithmetic, retry sequencing, what a wrong PIN does, what happens when a face changes during a pause: the
product, written down as assertions. **Re-deriving it from the app's behaviour would be the single largest
avoidable cost of the rewrite.**

**How to use this file.** Before writing a core module, find its row. Then read the named test file in
the reference tree, because **the row gives the subject and not the assertions**, there is no
substitute for reading what was actually asserted:

```sh
REF=~/harry.git/TimeFlipLinux            # the reference worktree, pinned to feature/linuxPort
cat $REF/Tests/FacetTests/<Name>Tests.swift
cat $REF/Sources/FacetCore/<Name>.swift
rg 'daily limit' $REF/Sources/FacetCore  # it is ordinary files, so search it like any tree
```

Test *names* in that suite are written as sentences, so `grep -E '^\s*(func test|@Test)'` on a file is a
readable list of its claims before reading any body.

**The counts are test declarations, not cases.** A parameterised test is one declaration and several
cases, which is why the suite reported 1,718 running tests against 2,063 declarations here.

**What is not in this table**: eleven files that are test doubles and helpers rather than behaviour
(`InMemoryCubeRadio`, `InMemoryGatt`, `InMemorySecretStore`, `FakeBlueZLink`, `FakeBlueZTransport`,
`HandDrivenScheduler`, `ManualClock`'s driver, `TemporaryDatabase`, `OffscreenWindow`,
`RecordingDialogues`, `ColourSamples`, `ApproximateEquality`). Read those too: they are the shape the Rust
test doubles want, and they encode which seams the design actually needed.

---

## Device: reaching the cube and keeping it

The largest and least negotiable group. Every rule here was paid for against real hardware, and
[`timeflip2-firmware-observations.md`](timeflip2-firmware-observations.md) is the evidence behind several
of them.

| Tests | Subject |
|---:|---|
| 43 | `DeviceCommandRules`: what may be asked of the cube, and the read-back-before-believing rule, including the clock's drift tolerance (finding 13 depends on this) |
| 44 | `DeviceHistoryRules`: parsing history frames: event numbering, the empty-frame sentinel, big-endian duration (findings 13 and 14) |
| 25 | `DeviceLoginRules`: the PIN exchange and its verdict, on the inverted `0x02` of finding 4 |
| 22 | `DevicePINRules` / 14 `DevicePINSource`: where a PIN comes from, and rotation |
| 19 | `DeviceScanRules`: eligibility: match on service **or** name, because the cube advertises no service UUID (finding 12) |
| 20 | `DevicePairingRules` / 36 `DevicePairingRecorder`: what pairing means and what it writes |
| 17 | `DeviceReconnectRules`, 16 `DeviceReconnectorOffer`, 14 `DeviceReconnectorAttempt`: the reconnect ladder |
| 18 | `DeviceNameRules`: renaming, and the stale GAP name of finding 1 |
| 16 | `DeviceInfoRules`: the Device Information strings, exact length rather than padded (finding 5) |
| 14 | `DeviceEventRules` / 35 `DeviceEventRecorder`: a device event's shape and its row |
| 12 | `DeviceSystemStateRules`, 10 `DeviceFaceRules`, 17 `BatteryRules` (finding 7: the level dithers, every waver a notification) |
| 24 | `DeviceSettingsSync`, 15 `DeviceSettingRows`, 12 `DeviceSettingWrite`: the five device settings and their round trip |
| 10 | `DoubleTapRules` (finding 11: no command disables it, sensitivity is the only lever) |
| 17 | `CubeCommandChannel`, 13 `CubeReachSequence`, 21 `CubeLock`, 8 `CubeResetProof`, 8 `CubeReports`, 5 `CubeFirstReading`, 4 `CubePauseState`, 12 `CubeNotFoundOffer` |
| 7 | `BLETrace`: the trace wordings. **These are interface**: the scripted suite matches them with `LIKE` and `GLOB` |
| 9 | `TimeFlipUUID`: the UUID table |
| 33 | `ClickLandsOnTheCubesFace`, 12 `RenamingTheCubeReachesItFirst`, 3 `PairingIsWhatTheAppFollows`, 10 `DeviceLogin`, named-scenario tests, each one a sequence rather than a rule |

**The BlueZ adapter's own tests** (27 `BlueZCubeRadio`, 22 `BlueZCubeGatt`, 9 `BlueZObjectTree`, 7
`SystemBus`) do not port: `btleplug` replaces all of it. Read them anyway for the *sequence* they drive,
which is the same sequence `facet-core` will drive through the radio port, and read
[`linux-bluez-port-notes.md`](linux-bluez-port-notes.md) for the traps they were written around.

---

## Time entries and the clock

The pipeline from a device event to a stored row. [`operation-spec.md`](operation-spec.md) is the prose
version and these are its assertions.

| Tests | Subject |
|---:|---|
| 18 | `TimeEntryRecorder`: writing an entry |
| 9 | `TimeEntryRules`: when an entry starts, ends and merges |
| 25 | `HistoryIngestor`: bringing the cube's own backlog in, and reconciling it with what is already stored |
| 15 | `HistoryTimer` |
| 18 | `ManualTimerRules`: timing with no cube |
| 21 | `ForcedPause`, 11 `ForcedPauseWatch` |
| 21 | `DailyLimitEnforcement`, 23 `DailyLimitWatch`, 6 `DailyLimitResume` |
| 10 | `LowBatteryWatch` |
| 20 | `DayTotal`, 13 `DayWindow`: what a day is, which is where timezone normalisation bites |
| 7 | `ManualClock`, 3 `ClockResumeFanOut`, 2 `LinkEndedFanOut` |
| 5 | `CreateStartsTiming`, 4 `AutoPauseSettlesBeforeItIsSent` (finding 10, and the three-round-trip correction loop) |

---

## Categories, faces and colours

| Tests | Subject |
|---:|---|
| 44 | `CategoryStore`: the table and its queries |
| 30 | `CategoryEdits`, 21 `CategoryRenameRules`, 20 `CategoryEditRules`, 16 `CategoryCreateRules` |
| 34 | `CategoryTable`, 21 `RetiredCategoryTable`, 10 `CategoryListView`, 8 `CategoryCreateControl`, surface behaviour, including retirement |
| 20 | `EditableNameCell`: click to open, Return commits, Escape abandons, a click elsewhere abandons, and **a model update must not take the text out from under a typist**. Proven to work in Slint on 2026-09-20 |
| 19 | `FaceStore`, 14 `FaceEdits`, 14 `FacesTabRules`, 8 `FacesPane` |
| 22 | `FaceColourSync`, 6 `FaceColourRules`, 13 `ColourList`, 6 `ColourStore` |
| 10 | `IconGrid`, 5 `ActivityIcon` |

---

## Report

| Tests | Subject |
|---:|---|
| 20 | `ReportTotals`: the sums, which is the whole point of the tab |
| 14 | `ReportCategoryGroup`, 12 `ReportSortRules`, 10 `ReportRangeRules`, 6 `ReportReadout` |
| 12 | `ReportCalendar`, 9 `ReportCalendarGrid`, 4 `ReportCalendarMetrics` |
| 10 | `ReportPane`, 3 `ReportTabAddsUpThePickedRange` |

---

## Menu bar

The part [`rust-port.md`](rust-port.md) says cannot be made identical on three platforms, so these rules
are also the specification for what each platform gives up.

| Tests | Subject |
|---:|---|
| 52 | `StatusItemTitle`: what the text beside the icon says. Unavailable on Windows in any language |
| 9 | `StatusItemReadout`: the readout the Linux tray label is asserted against |
| 19 | `StatusItemMenu`: the item list, including an optional Settings line (`None` means the line is not offered) |
| 15 | `StatusItemClickRouter`, 9 `StatusItemGesture`: left click, and what it is an accelerator *for* |
| 21 | `MenuBarController`, 8 `MainMenu`, 7 `MenuBarLine` |
| 12 | `PauseMenuRules` |

---

## Settings, preferences and windows

| Tests | Subject |
|---:|---|
| 94 | `DevicePane`: the largest file in the suite, and the Device tab is the densest surface in the app |
| 49 | `AppSettingsPane`, 18 `AppSettingsRules`, 6 `AppSettingWrite` |
| 48 | `TimingReadout`, 45 `TimingView` |
| 19 | `SettingStore`: the `setting` table |
| 14 | `SteppedNumberField`, 7 `StepperHoldRules`: press-and-hold acceleration |
| 7 | `SettingsMetrics`: every dimension in the window. **Portable and already proven to transfer**: the Slint prototype read its metrics from here rather than choosing them |
| 7 | `SettingsWindowController`, 4 `SettingsTab`, 6 `CollapsibleSection`, 9 `Dialogue` |
| 7 | `WriteDebounce`: debouncing a live-edited setting before it reaches the cube |

---

## Google Calendar

| Tests | Subject |
|---:|---|
| 21 | `GoogleOAuthRules`: the flow, including PKCE |
| 33 | `GoogleSection`: the Settings surface |
| 15 | `GoogleEventRules`, 11 `GoogleCalendarRules`, 11 `GoogleCalendar`, 8 `GoogleConnection` |
| 5 | `GoogleLoopbackListener`: the redirect listener, which is a port |
| 5 | `PortableSHA256`: written because CryptoKit is not portable; `sha2` replaces it |

[`google-oauth-setup.md`](google-oauth-setup.md) is the surrounding context, including the parts that are
about the Google Cloud project rather than about code.

---

## Storage, logging and process

| Tests | Subject |
|---:|---|
| 6 | `DatabaseBootstrap`: applying the DDL. **Must fail loudly on an empty set**; see [port-findings.md](port-findings.md) |
| 6 | `DatabaseEnvironment` |
| 15 | `DebugLog`, 3 `DebugLogTag`: the log the scripted suite reads its assertions out of |
| 13 | `DebugTraceRules`, 13 `DebugTraceFile` |
| 11 | `DeveloperConfigFile` |
| 12 | `SecretStore`: the port, exercised against an in-memory double |
| 4 | `InstanceLock`: refusing a second copy. A port for Windows |
| 13 | `QuitSequence` |
| 9 | `GLibScheduler`: the Linux clock slot |

---

## Tests about the rules rather than about the app

Four files whose subject is the architecture. Each one exists because the type system would not answer the
question and something had to read the sources. **All four want a Rust equivalent**, and
[architecture.md](architecture.md) says why the first one is partly free here and partly not.

| Tests | Subject |
|---:|---|
| 3 | `PlatformBlindCore`: scans the core for platform conditionals against a shrinking allowlist |
| 1 | `EveryLinuxExclusionEarnsItsPlace`: fails when a test excluded from a platform no longer needs to be |
| 1 | `AnIsolatedXCTestCaseAbortsTheLinuxRun`: a harness trap, specific to Swift; the class of problem is not |
| | `Package.swift`'s two exclusion lists, same tactic |
