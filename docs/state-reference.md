# State Reference

The name of every state in this app. **One state, one name, used everywhere.**

A second spelling of a state listed here is not an alternative, it is a rename waiting to happen.

Its companion, `docs/state-audit.md`, is the Swift tree's snapshot of what that code called these things and the
sweep list that mapped every spelling onto the name given here. It stayed in the reference tree
(`~/harry.git/TimeFlipLinux/docs/state-audit.md`, or `~/harry.git/TimeFlipApp/docs/state-audit.md` on the Linux
box) and has no counterpart for the Rust code. Section numbers match between the two files.

## The convention

- **A state that can only be true or false is `is<Name>` or `has<Name>`**, whichever is the English.
  `is<Name>` for a state the subject is *in*: `isCubeConnected`, `isManualMode`, `isCubePaired`, `isScanning`.
  `has<Name>` for something the subject *possesses* or *has already done*: `hasCategory`,
  `hasGoogleCredentials`, `hasReportedWriteFailure`. The test is which one reads as English: `hasPaused` is
  wrong because paused is a state, not a possession, and `isCategory` is wrong because a category is a thing
  a face holds, not a state it is in.
- **A state that can be more than true or false is `<name>State`.** `timingState`, `cubeLockState`. The type it
  is carried in takes the same name in Pascal case: `TimingState`, `CubeLockState`.
- **The name carries its subject.** `isLocked` obeys the convention and still means two unrelated facts, the
  cube frozen on its face and a face refusing reassignment. So `cubeLockState` and `isFaceLocked`, never
  `isLocked` twice.
- **No exceptions, including where the owning type repeats the subject.** `HistoryIngestor.isHistoryFetching`
  reads as a stutter, and that is the price of the value having one name wherever it is passed.

Two rules follow from the convention that are not obvious, and most of the renames below come from them.

**An optional Bool is three-valued, unless the third value is absence.** `Bool?` holds three answers, so it is
a `<name>State`. But `nil` does two different jobs here and only one is a state. Where `nil` means *nobody has
asked the cube yet*, that is a real third answer the app already branches on, so it becomes an explicit
`unknown` case. Where `nil` means *there is no such thing to ask about*, as in a face number that is not a
face, that is a failed lookup, the name stays `is<Name>`, and the absence is handled where the lookup is.

**A two-valued enum is a boolean.** The convention is about the fact, not the storage.

**Timing by hand is derived, never held** (2026-08-29, widened 2026-09-02). `isManualMode` is
`!isCubePaired || hasGivenUpOnCube`, both read at the moment the question is asked:
`ManualTimerRules.isManualMode(isCubePaired:hasGivenUpOnCube:)` is the one place they are put together, and every
surface goes through it. Nothing may hold the answer. It was a `LaunchMode` decided once at startup, which could not
move while the row under it did, and keeping the two in step cost a restart after every pair, forget and reset.

The second input is `hasGivenUpOnCube`, and it is the same fact as the reconnect loop stopping rather than a second
one: a paired launch that cannot find its cube offers `Rescan`, `Time by Hand` and `Quit`, and taking the middle one
both settles the loop and makes the app its own clock. It was called `hasStoppedLooking` while it did only the first
of those, on the reading that giving up hunting and having no device are different questions. They are, and the
pairing still answers the second -- but a launch that had given up hunting and *also* refused to time by hand was a
third state nobody asked for, which showed up as a Faces tab refusing every click after somebody had just said they
wanted to work without the cube.

---

## 1. Launch and process

| Name | Values | Truth |
| --- | --- | --- |
| `isManualMode` | true / false | `ManualTimerRules.isManualMode(isCubePaired:hasGivenUpOnCube:)`, both read at the point of use; in Rust `timing::is_manual_mode(is_cube_paired, has_given_up_on_cube)` in `facet-core`, called by `Faces::is_manual_mode` with `setting.paired.paired` read at that moment |
| `isTestDatabase` | true / false | `DatabaseEnvironment`, from `setting.db_type.type`; in Rust `setting::database_type` |
| `isQuitting` | true / false | `setting.connection.quit_request`, plus `QuitSequence` progress; in Rust a `Cell<bool>` local to `main` in each platform crate guarding a second quit, with `setting.connection.quit_request` written by `device::rows` at a clean quit |
| `isDebugEnabled` | true / false | `setting.debug.enabled`, read at launch and told to `DebugLog` by the Settings window; in Rust `Trace::is_recording`, built from `setting::debug_trace` at launch and switched by the App tab through `Trace::set_recording` |

## 2. Pairing and the link

| Name | Values | Truth |
| --- | --- | --- |
| `isCubePaired` | true / false | `setting.paired.paired` |
| `isCubeConnected` | true / false | `BluetoothRadio.connectedDevice != nil`; in Rust `Device::is_cube_connected()`, a link is held and no factory reset is running |
| `isLinkSettled` | true / false | `FaceColourSync.isLinkSettled`, set by `BluetoothRadio.onCubeSettled`; no Rust counterpart |
| `isScanning` | true / false | `BluetoothRadio.isScanning`; in Rust `Device.is_scanning` (and see *Rust spellings that differ from the register*) |
| `isScanWanted` | true / false | `BluetoothRadio.wantsToScan`; no Rust counterpart |
| `isReachingForCube` | true / false | `BluetoothRadio.isReaching`; in Rust `Device.is_reaching_for_cube` |
| `isAwaitingAnswer` | true / false | `DeviceReconnector.isAwaitingAnswer`; no Rust counterpart |
| `hasGivenUpOnCube` | true / false | `DeviceReconnector.hasGivenUpOnCube`, per launch and one-way; in Rust `Faces.has_given_up_on_cube`, set by Time by Hand on the not-found notice |
| `hasReadTheCube` | true / false | `CubeFirstReading`, per launch and one-way; no Rust counterpart |
| `isConnecting` | true / false | `CubeFirstReading.isConnecting(isManualMode:)`; in Rust `Device::is_connecting()`, reaching for the cube, not connected, and not in the wait before looking again after a drop |
| `isDisconnectingDeliberately` | true / false | `BluetoothRadio.isDisconnectingDeliberately`, and the argument of `BlueZCubeRadio.dropTheLink`; no Rust counterpart |
| `isFactoryResetRunning` | true / false | in Swift two separate flags; in Rust one, `Device.is_factory_reset_running` in `facet-ui`, from the moment the reset is confirmed in the dialog until its proof ends (`reset_ended`) |
| `isEditingDeviceName` | true / false | `DeviceData.is-editing-device-name`, the Name row's field being open |
| `renameRefusal` | `notPaired`, `notConnected`, `nameUnknown`, or none | `device::name::rename_refusal`, read from the table at the point of use |

`isCubeConnected` is the connection, not the pairing: a paired cube in another room can be neither paused nor
locked. `isDeviceReachable` folds into it. The reading keeps `cubeFace` after a link drops because the face is
worth drawing, and this is the separate question of what may be sent.

`hasReadTheCube` is a fourth, and it is the only one of them that outlives a drop. It is true once this launch has
had a cube connected with its face, its pause and its lock all known at the same moment -- not when the login
succeeded, which is several round trips earlier: `DeviceLogin` reports `.loggedIn` when the PIN is accepted, and the
`0x10` read the lock and the pause come from happens after. `isConnecting` is `!isManualMode && !hasReadTheCube`, and
it is the whole of what puts `Connecting...` on the status item. The latch is what keeps that title to startup: a drop
clears the face, the lock and the pause at the radio, so the three facts alone cannot tell "never found it" from
"found it and lost it", and the second of those is meant to keep the category and turn yellow. `CubeNotFoundOffer`
holds `hasReachedCube` for the same reason and is deliberately not the same fact -- that one settles whether the
not-found dialog may still be raised, and it moves at the earlier moment.

**In Rust there is no `hasReadTheCube`.** `Device::is_connecting()` in `crates/facet-ui/src/device.rs` answers the
same question as reaching for the cube, not connected, and not in the wait before looking again after a drop; that
last clause is what keeps `Connecting...` to startup. It feeds `StatusFacts.is_connecting` in
`crates/facet-core/src/status_line.rs`.

`isLinkSettled` is a third question again, and it is not `isCubeConnected` said later. The connection turns true
several round trips before the login has finished asking the cube its own questions, and until it has, the command
channel belongs to the login: the `0x17` read it has outstanding does not set `isCommandInFlight`, so a command sent
in that window is written over it rather than refused. So `isCubeConnected` is whether there is a cube to send to and
this is whether it may be sent to yet. Measured on 2026-08-28: over 26 connects the cube answered the systemState read
about 480ms before the login settled, every time.

`isFactoryResetRunning` is one fact held in the Swift tree as two flags set and cleared independently (in Rust it
is one flag). The sweep gives it one name; whether it should also be one flag is a code question, not a naming one.

`isReadingTheValue` is **not** `isReadingBack` said again, and the two being one fact is what a read-back cost on
Linux (2026-09-13). `isReadingBack` is whether this exchange is going to want an answer, and it is true from the
moment the question is written; `isReadingTheValue` is whether the read of the command result has actually gone
out. Everything arriving in between belongs to whoever asked before this did -- which matters because a `0x10`
reply carries no echoed command byte and the characteristic frequently holds the previous command's. Routing on
the first of the two had every login on that platform take the duplicate of its own `0x17` answer as the cube's
state, and had a quit report a command refused that the cube had taken.

The paragraphs above about `isLinkSettled`, `hasReadTheCube`, `isReadingBack` and `isReadingTheValue` describe the
Swift login, which interleaved reads on one command channel. The Rust login (`facet_core::device::session`) runs
each exchange in turn on a worker thread and holds none of these as flags. The measurements stand as the reason the
exchanges are sequenced.

## 3. In-flight work

| Name | Truth |
| --- | --- |
| `isCommandInFlight` | `DeviceLogin.isBusy`; no Rust counterpart |
| `isHistoryFetching` | `HistoryIngestor.isRefreshing`; in Rust `Device.is_history_fetching`, with `is_another_fetch_wanted` holding the one re-run |
| `isForcedPauseSending` | `ForcedPauseWatch.isSending`; in Rust `Device.is_forced_pause_sending`, held until the fetch that files the decision lands |
| `isCalendarSweeping` | `CalendarSync.isSweeping`; in Rust `Google.is_calendar_sweeping` |
| `isAnotherSweepWanted` | `CalendarSync.wantsAnotherPass`; in Rust `Google.is_another_sweep_wanted` |
| `isReadingBack` | `DeviceLogin.isReadingBack`; no Rust counterpart |
| `isReadingTheValue` | `CubeCommandChannel.isReadingTheValue`; no Rust counterpart |
| `isReadingDeviceInfo` | `DeviceLogin.isReadingInfo`; no Rust counterpart |
| `isReadingDoubleTap` | `DeviceLogin.isAskingAboutTaps`; no Rust counterpart |
| `isFollowingBattery` | `DeviceLogin.isFollowingBattery`; in Rust not held, the subscription made at login being the whole of it (`Following the battery`) |
| `isSigningIn` | `AppSettingsPane.isSigningIn`, both platforms; in Rust `Google.is_signing_in` |
| `isCalendarChanging` | `AppSettingsPane.isCalendarChanging` -- a create, rename or delete is out |
| `isWriteInFlight` | `DeviceSettingsSync.isWriteInFlight(_:)`, per setting |
| `hasSaidTaskParameters` | `Device.has_said_task_parameters`, said once per link and cleared at the next login |
| `hasReportedWaiting` | `Google.has_reported_waiting`, true once entries left waiting with nowhere to go have been said since the last pass that sent any |

## 4. The cube's own condition

| Name | Values | Truth |
| --- | --- | --- |
| `cubeLockState` | `unknown` / `locked` / `unlocked` | `BluetoothRadio.cubeStatus?.isLocked`; in Rust `Device.cube_status`, the last `0x10` answer, `None` for unknown |
| `cubePauseState` | `unknown` / `paused` / `running` | `device_event.paused` on the open row, or `cubeStatus?.isPaused` live |
| `pauseClaim` | `none` / `noCategory` (with the face) / `dailyLimit` | `Device.pause_claim`, held because no table records which face the app stopped the cube on. Decided by `forced_pause::decide`: dropped once the cube is seen running on a face with a category, or stopped on a different face than the claim names |
| `cubeFace` | 1 to 12, or none | `device_event.device_face` on the open row; `BluetoothRadio.currentFace` live, in Rust `Device.cube_face` |
| `batteryPercent` | 0 to 100, or none | `BluetoothRadio.batteryPercent`; in Rust `Device.battery`, through `device::info::charge_to_show` |
| `batteryWarningPercent` | 1 to 20 | `setting.low_battery_level.percent` |
| `isBatteryLow` | true / false | `LowBatteryWatch.isLow`, latched; in Rust `Device.is_battery_low`, decided by `device::info::is_battery_low` |
| `isBlinkOn` | true / false | `LowBatteryWatch` display phase; in Rust `Device.is_blink_on` |
| `cubeSyncState` | `ok`, `factoryReset`, `timeRequired`, `faceColoursRequired`, `ledBrightnessRequired`, `blinkIntervalRequired`, `taskParametersRequired`, `autoPauseRequired`, `unknown` | `DeviceSystemStateRules.Sync`; in Rust `device::system_state::CubeSyncState` (same cases) |
| `cubeHardwareState` | `ok`, `accelerometer`, `flash`, `accelerometerAndFlash`, `unknown` | `DeviceSystemStateRules.Hardware`; in Rust `device::system_state::CubeHardwareState` (same cases) |
| `isDoubleTapEnabled` | true / false | `setting.double_tap_settings.enabled`; no Rust counterpart |

`cubeLockState` and `cubePauseState` are the two largest changes here. In the Swift tree both were `Bool?` with
`nil` meaning nobody has asked, and both were read as `== true`, a comparison that looks like a mistake and is in
fact the whole "unknown counts as unlocked" decision. The Rust app has not made them three cases:
`Device::is_cube_locked()` returns `Option<bool>` and its callers compare with `== Some(true)`.

`cubePauseState` carries a trap no name can fix: a locked cube reports itself paused whatever its pause byte
says, so a pause confirmed after a lock proves nothing and pause is confirmed first.

## 5. Timing

| Name | Values | Truth |
| --- | --- | --- |
| `timingState` | `idle` / `running` / `paused` | `ManualTimerRules.state(categoryID:isRunning:)`; in Rust `timing::TimingState`, carried by `timing::Reading.timing_state` |
| `isCounting` | true / false | `TimingReadout.Reading.isCounting`, answered by `DayTotal`; in Rust `timing::Reading.is_counting`, from `timing::is_counting` |
| `isRepaintTicking` | true / false | `tick != nil` on both view controllers; no Rust counterpart |
| `isSegmentOpen` | true / false | `events.openSegment() != nil`, over `device_event.finalised = 0`; in Rust `segment::open_segment` returning `Some` |
| `isAppFace` | true / false | `face > 12`; in Rust `face::is_app_face` |
| `isHistoryTimerArmed` | true / false | `HistoryTimer.holder.timer != nil`; in Rust `Device.history_timer.running()`, armed while a link is held |
| `hasSomethingToFollow` | true / false | an open segment or a connected cube; no Rust counterpart |

`timingState` already has the right name and is the model for the rest. It is also the one most often asked
wrong: it is `idle` for the whole time a cube is followed, because the app runs no clock of its own then. A
branch that wants "something is being timed" wants `isCounting`.

`isAppFace` rather than `isManualFace`, so it cannot be misread as being about `isManualMode`. The faces are
the app's own whichever mode the launch is in.

## 6. Face, category and the daily limit

| Name | Values | Truth |
| --- | --- | --- |
| `hasCategory` | true / false | `face.category_id` |
| `isFaceLocked` | true / false | `face.locked` |
| `isCategoryActive` | true / false | `category.active` |
| `dailyLimitMinutes` | 0 or more, where 0 is no limit | `category.daily_limit` |
| `isLimitReached` | true / false | `DailyLimitEnforcement.isReached(totalSeconds:limitMinutes:)`; in Rust `timing::is_limit_reached(total_seconds, daily_limit_minutes)` |
| `isLimitHoldingPause` | true / false | `DailyLimitEnforcement.isPausedByLimit`; in Rust the cube's is `pauseClaim` of `dailyLimit`, and there is no separate flag |

`isFaceLocked` stays a boolean even though `FaceStore.isLocked(face:)` returns `Bool?`: there `nil` means the
number is not a face, which is a failed lookup rather than a third answer.

`isLimitReached` was one name for what were four expressions in four files, and this said that naming it did not
merge them, only made the fact that they had to agree visible. **Merged on 2026-09-10**, candidate 4 of
`docs/architecture-review-2026-09.md` in the reference tree:
`DailyLimitEnforcement.isResumeRefused(isLimitReached:isResuming:)` is the one expression (in Rust
`timing::is_resume_refused`, with `timing::is_limit_reached`, in `crates/facet-core/src/timing.rs`), and the five
sites that had their own each say only whether they are resuming. There were five rather than four, `CubeLock`
asking twice.

`isResumeRefused` is a **decision**, not a state, so it takes no `is<Name>`/`<name>State` entry of its own: see
*What the convention does not govern* below, alongside `ManualTimerRules.isClickable` and
`DeviceReconnectRules.shouldAttempt`.

## 7. History

| Name | Values | Truth |
| --- | --- | --- |
| `lastEventNumber` | a number | `MAX(device_event.event_number)`, checked against what the cube can reach; no Rust counterpart |
| `historyFrameState` | `event` / `noSuchEvent` / `endOfStream` | `DeviceHistoryRules`; in Rust `device::history::HistoryFrameState` (`Event`, `NoSuchEvent`, `EndOfStream`) |
| `isSingleFrameRequest` | true / false | the request's own shape; no Rust counterpart |

`historyFrameState` was three answers spread across two booleans in the Swift tree, which is the shape the
convention says is an enum: there `isNoSuchEvent` had to check `!isEndOfStream` first to avoid answering about the
wrong frame. In Rust it is the enum `HistoryFrameState`.

## 8. Google and calendar sync

| Name | Values | Truth |
| --- | --- | --- |
| `googleAccountState` | `notConnected`, `checking`, `signedOut`, `unverified`, `connected`, `expired`, `unreachable`, `unreadable` | `GoogleAccountRules.State`; in Rust `google::GoogleAccountState`, where `checking` is an account on record whose saved sign-in the secret store has not answered for yet |
| `credentialState` | `present`, `missing`, `unavailable` | the secret store, read on a background thread with a timeout because a locked store blocks; `google::CredentialState` |
| `googleSignInState` | `notAsked`, `working`, `unreachable`, `refused` | Swift `GoogleCalendar.SignInState`, which also had `notSignedIn` and `storeUnavailable`; in Rust `google::GoogleSignInState`, held by `Google.sign_in` and never stored, those two being `credentialState` `missing` and `unavailable`. What asking Google came back with, which `googleAccountState` is then worked out from |
| `hasGoogleCredentials` | true / false | client credentials present |
| `hasGoogleIdentity` | true / false | `GoogleAccountRules.Account`; in Rust `google::has_google_identity` |
| `calendarSettlementState` | `check(id)` / `leaveToTheUser` | `GoogleCalendarRules.Settlement`; no Rust counterpart |
| `isCalendarGone` | true / false | `CalendarGone` |
| `hasReportedMissingCalendar` | true / false | `CalendarSync` |
| `hasReportedWriteFailure` | true / false | `DebugLog` |
| `hasReachedCube` | true / false | `CubeNotFoundOffer`; no Rust counterpart |

## 9. Settings window and list UI

View state. Listed because it appears in branches, not because anything outside the window should read it.

| Name | Values | Truth |
| --- | --- | --- |
| `settingsTabState` | `faces`, `categories`, `report`, `app`, `device`, `about` | Swift: AppKit's `selectedTabViewItem` and GTK's `GtkNotebook` page, the cases being `SettingsTab`; Rust: `SettingsWindow.active-tab`, 0 to 5, in `crates/facet-ui/ui/settings.slint`, one window on both platforms |
| `isExpanded` | true / false | `PanelSection` / `DisclosureRow`, both platforms |
| `isEditing` | true / false | `CategoryCreateControl`, `EditableNameCell` |
| `isSelected` | true / false | per list |
| `isHidden` | true / false | AppKit |
| `isEnabled` | true / false | AppKit |

`isEnabled` and `isHidden` belong to `NSControl` and `NSView`, and to `gtk_widget_set_sensitive` and
`gtk_widget_hide` on the other side, and are not ours to rename. Our own answers to
"may this be pressed" (`isClickable`, `isButtonEnabled`, `isSelectable`) are **not** states and are not renamed
to `isEnabled`: they are decisions computed from state, and the last section says why that matters.

## 10. Report tab

| Name | Values | Truth |
| --- | --- | --- |
| `isInMonth` | true / false | `ReportCalendarGrid`; in Rust a `ReportDay` field built in `crates/facet-ui/src/report.rs` |
| `isRangeStart` | true / false | `ReportCalendarGrid`; in Rust a `ReportDay` field built in `crates/facet-ui/src/report.rs` |
| `isRangeEnd` | true / false | `ReportCalendarGrid`; in Rust a `ReportDay` field built in `crates/facet-ui/src/report.rs` |
| `isSameDay` | true / false | `ReportCalendarGrid`; no Rust counterpart |
| `isSameMonth` | true / false | `ReportCalendarGrid`; no Rust counterpart |
| `isEmphasised` | true / false | `ReportCalendar`; no Rust counterpart |
| `isSortAscending` | true / false | `ReportSortRules.Direction`; in Rust `report::SortOrder.is_sort_ascending` |
| `sortColumnState` | `category` / `time` | `ReportSortRules.Column`; in Rust `report::SortColumnState` |

---
## What the convention does not govern

Naming everything `is` or `State` would take in things that are not state.

- **Numbers and identifiers.** `cubeFace`, `batteryPercent`, `dailyLimitMinutes`, `lastEventNumber`. A face is
  1 to 12, not a set of named cases, and `cubeFaceState` would imply an enum that should not exist. The
  convention has no rule for these and this file does not invent one: they are named for what they hold.
- **Decisions.** `ManualTimerRules.isClickable`, `DeviceReconnectRules.shouldAttempt`,
  `StatusItemClickRouter.action`, `PauseMenuRules.target`, `FacesTabRules.doesAnything`. These answer "what
  should happen", computed from the states above. A decision is named for what it decides. Where one returns
  named outcomes it is already an enum (`StatusItemClick`, `DailyLimitAction`, `ForcedPauseAction`,
  `CubeNotFoundAnswer`, `DeviceLoginOutcome`) and does not take the `State` suffix, because an action is not a
  state.
- **Reports of what just happened.** `DeviceEventRecorder.Outcome.wasInserted` and `.isOpen` describe a write
  that has already run, not what the table now holds. `Outcome.isOpen` is therefore not renamed to
  `isSegmentOpen`.
- **A row's own columns.** `DeviceEventSegment.isPaused`, `TimeEntryRecorder.Row.isPaused` and `.isFinalised`,
  and `DeviceCommandRules.Status.isLocked` and `.isPaused` are fields of one decoded record, mirroring `paused`
  and `finalised`. **A finalised row saying it was a pause is history, not what the cube is doing now**, and the
  three-case states above exist precisely because the live question has an `unknown` that a decoded row never
  has. So these keep their boolean names, and `cubePauseState` is built from them rather than replacing them.
- **AppKit's own names.** `isEnabled`, `isHidden`, `isActive` on `NSLayoutConstraint`, `wantsLayer`,
  `needsLayout`. Not ours, and `isActive` is why `category.active` becomes `isCategoryActive` rather than
  `isActive`.
- **Database columns.** `paused`, `locked`, `finalised`, `processed`, `active`, `daily_limit`. SQL names set
  by the DDL. The Swift-side reader takes the name from this file.

---

## Rust spellings that differ from the register

Each of these is one fact with a second name in the Rust code. Rename the code to the register's name, or where the
Rust name is the better one change it here; do not leave both.

| Register name | Rust spelling | Where |
| --- | --- | --- |
| `isCalendarChanging` | `is_calendar_busy` | `Google`, `crates/facet-ui/src/google.rs` |
| `isManualMode`, inverted | `is_following_cube` | `Faces::is_following_cube`; `StatusFacts.is_following_cube` in `crates/facet-core/src/status_line.rs` |
| `isWriteInFlight` | `is_sending` | `EditedSetting`, `crates/facet-ui/src/device.rs` |
| `isScanning` | `is_radio_scanning`, held beside it | `Device`, `crates/facet-ui/src/device.rs` |
| `cubeLockState` | `is_cube_locked()` returning `Option<bool>` | `Device`; `StatusFacts.is_cube_locked` is a plain bool |
| `hasReportedWriteFailure` | `reported_failure` | `DebugLog`, `crates/facet-core/src/debug_log.rs` |
| `isCalendarGone` | `is_gone()` | `crates/facet-ui/src/google.rs` |

## Where the rename list lives

The Swift tree's rename list, what each of these was called in that code and the mapping from that to the name
above, is `docs/state-audit.md` in the reference tree. There is none for the Rust code; the Rust spellings that
differ from this register are listed in *Rust spellings that differ from the register* above.

## The second platform has added no state of its own

**Worth saying because it was not obvious it would be true.** The Swift Linux port (`FacetLinux`) grew a menu bar, five Settings tabs, a
radio and the Google half, and every fact any of it branches on is already in this register -- the same names, the
same values, mostly the same owning types, because the decisions are in `FacetCore` and only the drawing is not.

**The three it did add are all "what is this app doing right now"**: `isCalendarChanging`, `isWriteInFlight` and
`googleSignInState`, and none is a second answer to anything already here. Two of them were caught by this file
rather than by review -- they were written as `isWorking` and `signInCheck`, which carry no subject and hide five
values behind a name that reads as two.

## Adding a state

New code uses these names. If a branch needs a fact that is not here, add it here in the same change, and pick
the name by the convention at the top: two answers means `is<Name>`, more than two means `<name>State`, and
the subject goes in the name either way.
