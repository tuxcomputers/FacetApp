# State Reference

The name of every state in this app. **One state, one name, used everywhere.**

A second spelling of a state listed here is not an alternative, it is a rename waiting to happen.

Its companion is `docs/state-audit.md`: the snapshot of what the code calls these things today, and the sweep
list that maps every current spelling onto the name given here. Section numbers match between the two files.

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
| `isManualMode` | true / false | `ManualTimerRules.isManualMode(isCubePaired:hasGivenUpOnCube:)`, both read at the point of use |
| `isTestDatabase` | true / false | `DatabaseEnvironment`, from `setting.db_type.type` |
| `isQuitting` | true / false | `setting.connection.quit_request`, plus `QuitSequence` progress |
| `isDebugEnabled` | true / false | `setting.debug.enabled`, read at launch and told to `DebugLog` by the Settings window |

## 2. Pairing and the link

| Name | Values | Truth |
| --- | --- | --- |
| `isCubePaired` | true / false | `setting.paired.paired` |
| `isCubeConnected` | true / false | `BluetoothRadio.connectedDevice != nil` |
| `isLinkSettled` | true / false | `FaceColourSync.isLinkSettled`, set by `BluetoothRadio.onCubeSettled` |
| `isScanning` | true / false | `BluetoothRadio.isScanning` |
| `isScanWanted` | true / false | `BluetoothRadio.wantsToScan` |
| `isReachingForCube` | true / false | `BluetoothRadio.isReaching` |
| `isAwaitingAnswer` | true / false | `DeviceReconnector.isAwaitingAnswer` |
| `hasGivenUpOnCube` | true / false | `DeviceReconnector.hasGivenUpOnCube`, per launch and one-way |
| `hasReadTheCube` | true / false | `CubeFirstReading`, per launch and one-way |
| `isConnecting` | true / false | `CubeFirstReading.isConnecting(isManualMode:)` |
| `isDisconnectingDeliberately` | true / false | `BluetoothRadio.isDisconnectingDeliberately`, and the argument of `BlueZCubeRadio.dropTheLink` |
| `isFactoryResetRunning` | true / false | today two separate flags |

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

`isLinkSettled` is a third question again, and it is not `isCubeConnected` said later. The connection turns true
several round trips before the login has finished asking the cube its own questions, and until it has, the command
channel belongs to the login: the `0x17` read it has outstanding does not set `isCommandInFlight`, so a command sent
in that window is written over it rather than refused. So `isCubeConnected` is whether there is a cube to send to and
this is whether it may be sent to yet. Measured on 2026-08-28: over 26 connects the cube answered the systemState read
about 480ms before the login settled, every time.

`isFactoryResetRunning` is one fact currently held as two flags set and cleared independently. The sweep gives
it one name; whether it should also be one flag is a code question, not a naming one.

`isReadingTheValue` is **not** `isReadingBack` said again, and the two being one fact is what a read-back cost on
Linux (2026-09-13). `isReadingBack` is whether this exchange is going to want an answer, and it is true from the
moment the question is written; `isReadingTheValue` is whether the read of the command result has actually gone
out. Everything arriving in between belongs to whoever asked before this did -- which matters because a `0x10`
reply carries no echoed command byte and the characteristic frequently holds the previous command's. Routing on
the first of the two had every login on that platform take the duplicate of its own `0x17` answer as the cube's
state, and had a quit report a command refused that the cube had taken.

## 3. In-flight work

| Name | Truth |
| --- | --- |
| `isCommandInFlight` | `DeviceLogin.isBusy` |
| `isHistoryFetching` | `HistoryIngestor.isRefreshing` |
| `isForcedPauseSending` | `ForcedPauseWatch.isSending` |
| `isCalendarSweeping` | `CalendarSync.isSweeping` |
| `isAnotherSweepWanted` | `CalendarSync.wantsAnotherPass` |
| `isReadingBack` | `DeviceLogin.isReadingBack` |
| `isReadingTheValue` | `CubeCommandChannel.isReadingTheValue` |
| `isReadingDeviceInfo` | `DeviceLogin.isReadingInfo` |
| `isReadingDoubleTap` | `DeviceLogin.isAskingAboutTaps` |
| `isFollowingBattery` | `DeviceLogin.isFollowingBattery` |
| `isSigningIn` | `AppSettingsPane.isSigningIn`, both platforms |
| `isCalendarChanging` | `AppSettingsPane.isCalendarChanging` -- a create, rename or delete is out |
| `isWriteInFlight` | `DeviceSettingsSync.isWriteInFlight(_:)`, per setting |

## 4. The cube's own condition

| Name | Values | Truth |
| --- | --- | --- |
| `cubeLockState` | `unknown` / `locked` / `unlocked` | `BluetoothRadio.cubeStatus?.isLocked` |
| `cubePauseState` | `unknown` / `paused` / `running` | `device_event.paused` on the open row, or `cubeStatus?.isPaused` live |
| `cubeFace` | 1 to 12, or none | `device_event.device_face` on the open row; `BluetoothRadio.currentFace` live |
| `batteryPercent` | 0 to 100, or none | `BluetoothRadio.batteryPercent` |
| `batteryWarningPercent` | 1 to 20 | `setting.low_battery_level.percent` |
| `isBatteryLow` | true / false | `LowBatteryWatch.isLow`, latched |
| `isBlinkOn` | true / false | `LowBatteryWatch` display phase |
| `cubeSyncState` | `ok`, `factoryReset`, `timeRequired`, `faceColoursRequired`, `ledBrightnessRequired`, `blinkIntervalRequired`, `taskParametersRequired`, `autoPauseRequired`, `unknown` | `DeviceSystemStateRules.Sync` |
| `cubeHardwareState` | `ok`, `accelerometer`, `flash`, `accelerometerAndFlash`, `unknown` | `DeviceSystemStateRules.Hardware` |
| `isDoubleTapEnabled` | true / false | `setting.double_tap_settings.enabled` |

`cubeLockState` and `cubePauseState` are the two largest changes here. Both are `Bool?` today with `nil`
meaning nobody has asked, and both are read as `== true`, a comparison that looks like a mistake and is in
fact the whole "unknown counts as unlocked" decision. As three cases the branch says what it means.

`cubePauseState` carries a trap no name can fix: a locked cube reports itself paused whatever its pause byte
says, so a pause confirmed after a lock proves nothing and pause is confirmed first.

## 5. Timing

| Name | Values | Truth |
| --- | --- | --- |
| `timingState` | `idle` / `running` / `paused` | `ManualTimerRules.state(categoryID:isRunning:)` |
| `isCounting` | true / false | `TimingReadout.Reading.isCounting`, answered by `DayTotal` |
| `isRepaintTicking` | true / false | `tick != nil` on both view controllers |
| `isSegmentOpen` | true / false | `events.openSegment() != nil`, over `device_event.finalised = 0` |
| `isAppFace` | true / false | `face > 12` |
| `isHistoryTimerArmed` | true / false | `HistoryTimer.holder.timer != nil` |
| `hasSomethingToFollow` | true / false | an open segment or a connected cube |

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
| `isLimitReached` | true / false | `DailyLimitEnforcement.isReached(totalSeconds:limitMinutes:)` |
| `isLimitHoldingPause` | true / false | `DailyLimitEnforcement.isPausedByLimit` |

`isFaceLocked` stays a boolean even though `FaceStore.isLocked(face:)` returns `Bool?`: there `nil` means the
number is not a face, which is a failed lookup rather than a third answer.

`isLimitReached` was one name for what were four expressions in four files, and this said that naming it did not
merge them, only made the fact that they had to agree visible. **Merged on 2026-09-10**, candidate 4 of
`docs/architecture-review-2026-09.md`: `DailyLimitEnforcement.isResumeRefused(isLimitReached:isResuming:)` is the
one expression, and the five sites that had their own each say only whether they are resuming. There were five
rather than four, `CubeLock` asking twice.

`isResumeRefused` is a **decision**, not a state, so it takes no `is<Name>`/`<name>State` entry of its own: see
*What the convention does not govern* below, alongside `ManualTimerRules.isClickable` and
`DeviceReconnectRules.shouldAttempt`.

## 7. History

| Name | Values | Truth |
| --- | --- | --- |
| `lastEventNumber` | a number | `MAX(device_event.event_number)`, checked against what the cube can reach |
| `historyFrameState` | `event` / `noSuchEvent` / `endOfStream` | `DeviceHistoryRules` |
| `isSingleFrameRequest` | true / false | the request's own shape |

`historyFrameState` is three answers currently spread across two booleans, which is the shape the convention
says is an enum: today `isNoSuchEvent` has to check `!isEndOfStream` first to avoid answering about the wrong
frame.

## 8. Google and calendar sync

| Name | Values | Truth |
| --- | --- | --- |
| `googleAccountState` | `notConnected`, `signedOut`, `unverified`, `connected`, `expired`, `unreachable`, `unreadable` | `GoogleAccountRules.State` |
| `googleSignInState` | `working`, `notSignedIn`, `storeUnavailable`, `unreachable`, `refused` | `GoogleCalendar.SignInState` -- what asking Google came back with, which `googleAccountState` is then worked out from |
| `hasGoogleCredentials` | true / false | client credentials present |
| `hasGoogleIdentity` | true / false | `GoogleAccountRules.Account` |
| `calendarSettlementState` | `check(id)` / `leaveToTheUser` | `GoogleCalendarRules.Settlement` |
| `isCalendarGone` | true / false | `CalendarGone` |
| `hasReportedMissingCalendar` | true / false | `CalendarSync` |
| `hasReportedWriteFailure` | true / false | `DebugLog` |
| `hasReachedCube` | true / false | `CubeNotFoundOffer` |

## 9. Settings window and list UI

View state. Listed because it appears in branches, not because anything outside the window should read it.

| Name | Values | Truth |
| --- | --- | --- |
| `settingsTabState` | `faces`, `categories`, `report`, `app`, `device` | AppKit's `selectedTabViewItem` and GTK's `GtkNotebook` page; the cases are `SettingsTab` |
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
| `isInMonth` | true / false | `ReportCalendarGrid` |
| `isRangeStart` | true / false | `ReportCalendarGrid` |
| `isRangeEnd` | true / false | `ReportCalendarGrid` |
| `isSameDay` | true / false | `ReportCalendarGrid` |
| `isSameMonth` | true / false | `ReportCalendarGrid` |
| `isEmphasised` | true / false | `ReportCalendar` |
| `isSortAscending` | true / false | `ReportSortRules.Direction` |
| `sortColumnState` | `category` / `time` | `ReportSortRules.Column` |

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

## Where the rename list lives

What each of these is called in the code today, and the mapping from that to the name above, is the sweep
list in `docs/state-audit.md`. It belongs there because it is a record of this codebase at this moment,
while this file is the naming itself.

## The second platform has added no state of its own

**Worth saying because it was not obvious it would be true.** `FacetLinux` grew a menu bar, five Settings tabs, a
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
