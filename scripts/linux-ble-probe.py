#!/usr/bin/env python3
"""Prove a Linux box can reach a TimeFlip2 cube, over BlueZ, with no app in the way.

**This is a spike, not a scripted check.** It answers the one question the repository cannot:
every firmware observation in `docs/timeflip2-firmware-observations.md` was measured through
CoreBluetooth, so nothing on file says BlueZ can drive this hardware at all. It walks the same
sequence `DeviceLogin` does -- scan, connect, resolve, present a PIN, read the verdict, read the
face, subscribe -- and prints what each step actually returned.

It doubles as the reference for the port. The Swift side will speak the same D-Bus interfaces
(`org.bluez.Adapter1`, `Device1`, `GattCharacteristic1`), so the call order here is the call order
there.

Nothing here writes to a database and nothing is remembered between runs. The cube is left
disconnected on the way out.
"""

import argparse
import sys
import time
from datetime import datetime

try:
    import dbus
    import dbus.mainloop.glib
    from gi.repository import GLib
except ImportError as exc:
    print(f"missing a python module this needs: {exc}", file=sys.stderr)
    print("install with: sudo apt install python3-dbus python3-gi", file=sys.stderr)
    sys.exit(2)

BLUEZ = "org.bluez"
OBJECT_MANAGER = "org.freedesktop.DBus.ObjectManager"
PROPERTIES = "org.freedesktop.DBus.Properties"
ADAPTER_IFACE = "org.bluez.Adapter1"
DEVICE_IFACE = "org.bluez.Device1"
SERVICE_IFACE = "org.bluez.GattService1"
CHARACTERISTIC_IFACE = "org.bluez.GattCharacteristic1"

# From `Sources/FacetMac/TimeFlipUUIDs.swift`, lowercased because BlueZ reports them that way.
# The names match `TimeFlipUUIDs.name(for:)` so a trace here reads like a trace there, and an
# unnamed UUID falls through to its full form for the same reason the app does it: a characteristic
# nothing has a name for is a finding, not noise.
SERVICE_UUID = "f1196f50-71a4-11e6-bdf4-0800200c9a66"
# `DeviceScanRules.vendorName`, matched as a substring the same way and for the same reason.
VENDOR_NAME = "timeflip"
NAMES = {
    "f1196f50-71a4-11e6-bdf4-0800200c9a66": "timeFlipService",
    "f1196f51-71a4-11e6-bdf4-0800200c9a66": "eventsData",
    "f1196f52-71a4-11e6-bdf4-0800200c9a66": "faces",
    "f1196f53-71a4-11e6-bdf4-0800200c9a66": "commandResult",
    "f1196f54-71a4-11e6-bdf4-0800200c9a66": "command",
    "f1196f55-71a4-11e6-bdf4-0800200c9a66": "doubleTap",
    "f1196f56-71a4-11e6-bdf4-0800200c9a66": "systemState",
    "f1196f57-71a4-11e6-bdf4-0800200c9a66": "password",
    "f1196f58-71a4-11e6-bdf4-0800200c9a66": "history",
    "0000180a-0000-1000-8000-00805f9b34fb": "deviceInformation",
    "00002a29-0000-1000-8000-00805f9b34fb": "manufacturerName",
    "00002a24-0000-1000-8000-00805f9b34fb": "modelNumber",
    "00002a27-0000-1000-8000-00805f9b34fb": "hardwareRevision",
    "00002a26-0000-1000-8000-00805f9b34fb": "firmwareRevision",
    "0000180f-0000-1000-8000-00805f9b34fb": "batteryService",
    "00002a19-0000-1000-8000-00805f9b34fb": "batteryLevel",
}
PASSWORD = "f1196f57-71a4-11e6-bdf4-0800200c9a66"
COMMAND_RESULT = "f1196f53-71a4-11e6-bdf4-0800200c9a66"
FACES = "f1196f52-71a4-11e6-bdf4-0800200c9a66"
SYSTEM_STATE = "f1196f56-71a4-11e6-bdf4-0800200c9a66"
BATTERY_LEVEL = "00002a19-0000-1000-8000-00805f9b34fb"
DEVICE_INFO = [
    "00002a29-0000-1000-8000-00805f9b34fb",
    "00002a24-0000-1000-8000-00805f9b34fb",
    "00002a27-0000-1000-8000-00805f9b34fb",
    "00002a26-0000-1000-8000-00805f9b34fb",
]

# Padded to the longest, the way `DebugLog.Tag.bracketed` does it, so the console lines align.
TAG_WIDTH = 7


def say(tag, message):
    stamp = datetime.now().strftime("%H:%M:%S")
    print(f"{stamp} [{tag.ljust(TAG_WIDTH)}] {message}", flush=True)


def fail(tag, message):
    say(tag, f"FAILED: {message}")


def name_for(uuid):
    return NAMES.get(uuid.lower(), uuid)


def hexed(value):
    return " ".join(f"{b:02X}" for b in bytes(value))


def printable(value):
    raw = bytes(value).rstrip(b"\x00")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError:
        return None
    return text if text.isprintable() and text else None


def pump(seconds):
    """Let GLib deliver signals for a while. Every wait in here goes through this."""
    context = GLib.MainContext.default()
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        context.iteration(False)
        time.sleep(0.02)


def wait_until(predicate, seconds, interval=0.05):
    """Pump until the predicate holds. Returns whether it did, never raises on timeout."""
    context = GLib.MainContext.default()
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate():
            return True
        context.iteration(False)
        time.sleep(interval)
    return predicate()


class Probe:
    def __init__(self, bus, args):
        self.bus = bus
        self.args = args
        self.stages = []
        self.device_path = None
        self.characteristics = {}
        self.face_readings = []

    def record(self, stage, passed, detail=""):
        self.stages.append((stage, passed, detail))

    # -- object plumbing ------------------------------------------------------

    def managed(self):
        manager = dbus.Interface(self.bus.get_object(BLUEZ, "/"), OBJECT_MANAGER)
        return manager.GetManagedObjects()

    def props(self, path, iface):
        return dbus.Interface(self.bus.get_object(BLUEZ, path), PROPERTIES).GetAll(iface)

    def prop(self, path, iface, key):
        return dbus.Interface(self.bus.get_object(BLUEZ, path), PROPERTIES).Get(iface, key)

    # -- stages ---------------------------------------------------------------

    def find_adapter(self):
        for path, ifaces in self.managed().items():
            if ADAPTER_IFACE in ifaces:
                address = ifaces[ADAPTER_IFACE].get("Address", "unknown")
                powered = bool(ifaces[ADAPTER_IFACE].get("Powered", False))
                say("probe", f"Adapter {path} at {address}, powered {powered}")
                if not powered:
                    say("probe", "Powering the adapter on")
                    try:
                        dbus.Interface(self.bus.get_object(BLUEZ, path), PROPERTIES).Set(
                            ADAPTER_IFACE, "Powered", dbus.Boolean(True)
                        )
                    except dbus.DBusException as exc:
                        fail("probe", f"could not power the adapter on: {exc}")
                        return None
                return path
        fail("probe", "no Bluetooth adapter is present on this machine")
        return None

    def forget_known_cubes(self, adapter_path):
        """Drop any cached Device1 for the cube before scanning.

        BlueZ caches the GATT database and the name per MAC address, and this cube renames itself
        -- so a stale cache is a real way to get a wrong answer that looks like a firmware fault.
        Off by default because removing a device is a side effect on the user's machine.
        """
        adapter = dbus.Interface(self.bus.get_object(BLUEZ, adapter_path), ADAPTER_IFACE)
        removed = 0
        for path, ifaces in self.managed().items():
            if DEVICE_IFACE not in ifaces:
                continue
            uuids = [str(u).lower() for u in ifaces[DEVICE_IFACE].get("UUIDs", [])]
            name = str(ifaces[DEVICE_IFACE].get("Alias", ""))
            if SERVICE_UUID in uuids or VENDOR_NAME in name.lower():
                try:
                    adapter.RemoveDevice(path)
                    removed += 1
                    say("probe", f"Removed the cached device at {path}")
                except dbus.DBusException as exc:
                    fail("probe", f"could not remove the cached device at {path}: {exc}")
                    return False
        if removed == 0:
            say("probe", "No cached cube to remove")
        return True

    def scan(self, adapter_path):
        adapter = dbus.Interface(self.bus.get_object(BLUEZ, adapter_path), ADAPTER_IFACE)
        # **No UUID filter here, matching `BluetoothRadio.scanForPeripherals(withServices: nil)`.**
        # A 128-bit UUID costs 16 of the 31 bytes an advertisement has, and this cube spends them on
        # its name instead -- so a scan filtered on the service UUID sees nothing at all. Measured on
        # this machine 2026-09-06: twenty seconds filtered saw no cube, unfiltered saw it at once.
        # `DeviceScanRules.vendorName` is the app-side counterpart, matching the name as a substring.
        try:
            adapter.SetDiscoveryFilter({
                "Transport": dbus.String("le"),
                "DuplicateData": dbus.Boolean(False),
            })
        except dbus.DBusException as exc:
            fail("scan", f"the discovery filter was refused: {exc}")
            return None
        try:
            adapter.StartDiscovery()
        except dbus.DBusException as exc:
            fail("scan", f"discovery would not start: {exc}")
            return None
        say("scan", f"Scanning for the TimeFlip service, up to {self.args.scan_timeout}s")

        found = {}
        seen = {}

        def look():
            for path, ifaces in self.managed().items():
                if DEVICE_IFACE not in ifaces or path in found:
                    continue
                device = ifaces[DEVICE_IFACE]
                uuids = [str(u).lower() for u in device.get("UUIDs", [])]
                alias = str(device.get("Alias", ""))
                seen[path] = (alias, str(device.get("Address", "")), device.get("RSSI", "unknown"))
                # Name first, service UUID second, because the name is what actually arrives in the
                # advertisement. `DeviceScanRules.matches` is liberal about the vendor name for the
                # same reason and this mirrors it.
                if VENDOR_NAME not in alias.lower() and SERVICE_UUID not in uuids:
                    continue
                found[path] = device
                rssi = device.get("RSSI", "unknown")
                say("scan", f"Found {alias} at {device.get('Address')}, RSSI {rssi}")
            if self.args.address:
                return any(
                    str(d.get("Address", "")).upper() == self.args.address.upper()
                    for d in found.values()
                )
            return bool(found)

        wait_until(look, self.args.scan_timeout)
        try:
            adapter.StopDiscovery()
        except dbus.DBusException as exc:
            say("scan", f"Note: stopping discovery reported {exc}")

        if not found:
            fail("scan", f"no cube was seen. {len(seen)} LE devices answered in that time")
            # Print what did answer. A scan that says only no is a scan that cannot tell a cube that
            # is off from a cube this probe failed to recognise.
            for alias, address, rssi in sorted(seen.values()):
                say("scan", f"  saw {alias or 'unnamed'} at {address}, RSSI {rssi}")
            return None
        if self.args.address:
            for path, device in found.items():
                if str(device.get("Address", "")).upper() == self.args.address.upper():
                    return path
            fail("scan", f"no cube at {self.args.address} was seen")
            return None
        if len(found) > 1:
            say("scan", f"More than one cube answered; taking the first of {len(found)}")
        return next(iter(found))

    def connect(self, path):
        address = str(self.prop(path, DEVICE_IFACE, "Address"))
        alias = str(self.prop(path, DEVICE_IFACE, "Alias"))
        say("connect", f"Connecting to {alias} at {address}")
        device = dbus.Interface(self.bus.get_object(BLUEZ, path), DEVICE_IFACE)

        def connected():
            return bool(self.prop(path, DEVICE_IFACE, "Connected"))

        # Discovery needs a moment to wind down before a connect will take. Without this BlueZ
        # answers `Operation already in progress` on a cube that is perfectly reachable, which is
        # what it did here on the first run (2026-09-06).
        pump(1.0)
        for attempt in range(1, 4):
            if connected():
                break
            try:
                device.Connect(timeout=self.args.connect_timeout + 5)
                break
            except dbus.DBusException as exc:
                name = exc.get_dbus_name()
                message = str(exc)
                # **In progress is not a refusal.** BlueZ starts its own attempt off the back of
                # discovery, and a second Connect on top of it is the error rather than the cause.
                # Waiting for the property is the answer, and only a wait that expires is a failure.
                if "already in progress" in message.lower() or "InProgress" in name:
                    say("connect", f"A connection is already under way, waiting for it (try {attempt})")
                    if wait_until(connected, self.args.connect_timeout):
                        break
                    fail("connect", f"that attempt never completed within {self.args.connect_timeout}s")
                    return False
                if attempt == 3:
                    fail("connect", f"the cube refused the connection: {exc}")
                    return False
                say("connect", f"Attempt {attempt} was refused ({name}), retrying")
                pump(2.0)
        if not connected():
            fail("connect", "the link never came up")
            return False
        # BlueZ resolves the GATT database after the link is up, and this is the moment that
        # corresponds to CoreBluetooth calling back with discovered services. Reading a
        # characteristic before it lands is how a probe finds nothing on a cube that is fine.
        resolved = wait_until(
            lambda: bool(self.prop(path, DEVICE_IFACE, "ServicesResolved")),
            self.args.connect_timeout,
        )
        if not resolved:
            fail("connect", f"services were not resolved within {self.args.connect_timeout}s")
            return False
        say("connect", "Connected, services resolved")
        return True

    def enumerate(self, device_path):
        objects = self.managed()
        services = {
            path: str(ifaces[SERVICE_IFACE]["UUID"]).lower()
            for path, ifaces in objects.items()
            if SERVICE_IFACE in ifaces and str(ifaces[SERVICE_IFACE].get("Device")) == device_path
        }
        if not services:
            fail("gatt", "the cube resolved no services at all")
            return False
        count = 0
        for service_path, service_uuid in sorted(services.items()):
            say("gatt", f"Service {name_for(service_uuid)} ({service_uuid})")
            for path, ifaces in sorted(objects.items()):
                if CHARACTERISTIC_IFACE not in ifaces:
                    continue
                characteristic = ifaces[CHARACTERISTIC_IFACE]
                if str(characteristic.get("Service")) != service_path:
                    continue
                uuid = str(characteristic["UUID"]).lower()
                flags = ", ".join(str(f) for f in characteristic.get("Flags", []))
                self.characteristics[uuid] = path
                count += 1
                say("gatt", f"  {name_for(uuid).ljust(20)} {flags}")
        say("gatt", f"{len(services)} services, {count} characteristics")
        if SERVICE_UUID not in services.values():
            fail("gatt", "the TimeFlip service is not among them")
            return False
        missing = [
            name_for(u) for u in (PASSWORD, COMMAND_RESULT, FACES) if u not in self.characteristics
        ]
        if missing:
            fail("gatt", f"the TimeFlip service is missing {', '.join(missing)}")
            return False
        return True

    def read(self, uuid, tag="read"):
        path = self.characteristics.get(uuid)
        if path is None:
            fail(tag, f"{name_for(uuid)} is not present on this cube")
            return None
        try:
            value = dbus.Interface(
                self.bus.get_object(BLUEZ, path), CHARACTERISTIC_IFACE
            ).ReadValue({})
        except dbus.DBusException as exc:
            fail(tag, f"reading {name_for(uuid)} was refused: {exc}")
            return None
        raw = bytes(value)
        text = printable(value)
        shown = f"{hexed(value)}" + (f"  ({text})" if text else "")
        say(tag, f"{name_for(uuid)} -> {shown}")
        return raw

    def present_pin(self, pin):
        path = self.characteristics[PASSWORD]
        # `Data(pin.utf8)` on the app side, so the six digits go on the wire as ASCII.
        payload = dbus.Array([dbus.Byte(b) for b in pin.encode("utf-8")], signature="y")
        say("login", f"Presenting a PIN, {len(payload)} bytes: {hexed(pin.encode())}")
        try:
            # "request" is BlueZ for a write with response, which is what the app uses so that a
            # refused write is distinguishable from one the cube took.
            dbus.Interface(self.bus.get_object(BLUEZ, path), CHARACTERISTIC_IFACE).WriteValue(
                payload, {"type": dbus.String("request")}
            )
        except dbus.DBusException as exc:
            fail("login", f"the password write was refused: {exc}")
            return False
        say("login", "The password write was acknowledged")
        # The verdict is read only after the acknowledgement. Reading before the cube has processed
        # the write is how a stale command result gets mistaken for an answer -- finding 2 in
        # `docs/timeflip2-firmware-observations.md`, and the reason `DeviceLogin` orders it this way.
        verdict = self.read(COMMAND_RESULT, tag="login")
        if verdict is None:
            return False
        # `DeviceLoginRules.verdict`: the first byte is the whole answer, and there is no echoed
        # command byte to check it against. 0x02 accepted, 0x01 rejected, anything else unreadable
        # rather than a rejection -- finding 2 again, since this characteristic often holds whatever
        # the previous command left in it. A login is one of the commands that does update it.
        if not verdict:
            fail("login", "the command result came back empty")
            return False
        code = verdict[0]
        if code == 0x02:
            say("login", "The cube accepted the PIN")
            return True
        if code == 0x01:
            fail("login", "the cube rejected the PIN")
            return False
        fail("login", f"the command result is unreadable: {hexed(verdict)}")
        return False

    def watch_faces(self, seconds):
        path = self.characteristics[FACES]
        characteristic = dbus.Interface(self.bus.get_object(BLUEZ, path), CHARACTERISTIC_IFACE)

        def changed(interface, changes, invalidated, sender_path=None):
            if interface != CHARACTERISTIC_IFACE or "Value" not in changes:
                return
            raw = bytes(changes["Value"])
            self.face_readings.append(raw)
            say("notify", f"faces -> {hexed(raw)}  (face {raw[0] if raw else 'empty'})")

        watcher = self.bus.add_signal_receiver(
            changed,
            dbus_interface=PROPERTIES,
            signal_name="PropertiesChanged",
            path=path,
            path_keyword="sender_path",
        )
        try:
            characteristic.StartNotify()
        except dbus.DBusException as exc:
            fail("notify", f"subscribing to faces was refused: {exc}")
            watcher.remove()
            return False
        say("notify", f"Subscribed to faces. TURN THE CUBE NOW, for the next {seconds}s")
        pump(seconds)
        try:
            characteristic.StopNotify()
        except dbus.DBusException as exc:
            say("notify", f"Note: unsubscribing reported {exc}")
        watcher.remove()
        if not self.face_readings:
            fail("notify", "no face change arrived; either the cube was not turned or notify is dead")
            return False
        say("notify", f"{len(self.face_readings)} face changes arrived")
        return True

    def disconnect(self, path):
        try:
            dbus.Interface(self.bus.get_object(BLUEZ, path), DEVICE_IFACE).Disconnect()
            say("probe", "Disconnected")
        except dbus.DBusException as exc:
            say("probe", f"Note: disconnecting reported {exc}")

    # -- the run --------------------------------------------------------------

    def run(self):
        adapter_path = self.find_adapter()
        if adapter_path is None:
            self.record("adapter", False, "none present")
            return False
        self.record("adapter", True)

        if self.args.clear_cache and not self.forget_known_cubes(adapter_path):
            self.record("clear cache", False)
            return False

        device_path = self.scan(adapter_path)
        if device_path is None:
            self.record("scan", False, "no cube seen")
            return False
        self.record("scan", True, str(self.prop(device_path, DEVICE_IFACE, "Address")))
        self.device_path = device_path

        if not self.connect(device_path):
            self.record("connect", False)
            return False
        self.record("connect", True)

        try:
            if not self.enumerate(device_path):
                self.record("gatt", False)
                return False
            self.record("gatt", True, f"{len(self.characteristics)} characteristics")

            logged_in = self.present_pin(self.args.pin)
            self.record("login", logged_in, f"PIN {self.args.pin}")

            say("read", "Reading what the cube says about itself")
            face = self.read(FACES)
            self.record("read face", face is not None, f"face {face[0]}" if face else "")
            self.read(SYSTEM_STATE)
            self.read(BATTERY_LEVEL)
            for uuid in DEVICE_INFO:
                if uuid in self.characteristics:
                    self.read(uuid)

            if self.args.watch_seconds > 0:
                turned = self.watch_faces(self.args.watch_seconds)
                self.record("notify", turned, f"{len(self.face_readings)} changes")
            else:
                say("notify", "Skipping the notify stage, watch-seconds is zero")
        finally:
            self.disconnect(device_path)

        return all(passed for _, passed, _ in self.stages)


def main():
    parser = argparse.ArgumentParser(
        description="Prove BlueZ can reach a TimeFlip2 cube. Needs the cube disconnected from "
                    "everything else, since it holds one connection at a time."
    )
    parser.add_argument("--pin", default="000000",
                        help="the six digit PIN, ASCII on the wire (default: the vendor default)")
    parser.add_argument("--address", default=None,
                        help="target one cube by MAC, for when more than one answers")
    parser.add_argument("--scan-timeout", type=float, default=20.0)
    parser.add_argument("--connect-timeout", type=float, default=25.0)
    parser.add_argument("--watch-seconds", type=float, default=30.0,
                        help="how long to watch for face changes; 0 skips that stage")
    parser.add_argument("--clear-cache", action="store_true",
                        help="remove any cached BlueZ device for the cube first. BlueZ caches the "
                             "GATT database and the name per MAC, and this cube renames itself")
    args = parser.parse_args()

    if len(args.pin) != 6 or not args.pin.isdigit():
        print("the PIN must be six digits", file=sys.stderr)
        return 2

    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    try:
        bus = dbus.SystemBus()
    except dbus.DBusException as exc:
        print(f"cannot reach the system bus: {exc}", file=sys.stderr)
        return 2

    say("probe", "TimeFlip2 over BlueZ, a reachability probe")
    say("probe", "The cube holds one connection. Quit Facet on the Mac before this runs")

    probe = Probe(bus, args)
    try:
        passed = probe.run()
    except KeyboardInterrupt:
        say("probe", "Interrupted")
        if probe.device_path:
            probe.disconnect(probe.device_path)
        return 130
    except dbus.DBusException as exc:
        fail("probe", f"an unexpected D-Bus error ended the run: {exc}")
        if probe.device_path:
            probe.disconnect(probe.device_path)
        return 1

    print()
    say("result", "-- what happened --")
    for stage, ok, detail in probe.stages:
        mark = "pass" if ok else "FAIL"
        say("result", f"  {mark}  {stage}" + (f"  ({detail})" if detail else ""))
    say("result", "Every stage passed" if passed else "Something failed, see above")
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
