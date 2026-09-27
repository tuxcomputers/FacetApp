#!/usr/bin/env python3
"""Read and drive Facet's tray menu on Linux, over `com.canonical.dbusmenu`.

**The Linux counterpart of `scripts/status-item-click.py`, and it needs no mouse.** A status item is
absent from the accessibility tree on both platforms (`docs/linux-port.md`), so on the Mac the menu
has to be opened with a real `CGEvent`. Here the menu is a D-Bus object with methods for reading it
and for choosing an item, so a check presses by name and never touches the pointer -- which also
means it works while the session is doing something else.

Two facts about this interface cost time to find, both measured 2026-09-13, and both are why this
addresses items by **label**:

* **No identifier crosses.** `StatusItemMenu.Item.identifier` reaches `AXIdentifier` on the Mac and
  reaches nothing here: neither `gtk_widget_set_name` nor the accessible description is carried, and
  what `GetLayout` answers is the label plus `enabled`.
* **The numeric ids are libdbusmenu's own** and are reassigned every time the menu is rebuilt, so an
  id read in one step is meaningless in the next.

`Event` takes a **variant** as its third argument (`isvu`, not `issu`), which python-dbus will not
infer: the signature has to be given.
"""

import argparse
import subprocess
import sys

try:
    import dbus
except ImportError as exc:
    print(f"missing a python module this needs: {exc}", file=sys.stderr)
    print("install with: sudo apt install python3-dbus", file=sys.stderr)
    sys.exit(2)

MENU_IFACE = "com.canonical.dbusmenu"

# The indicator itself, which is a different object from its menu and carries the words the status
# item shows. `--label` reads these, and it is the Linux answer to `status_item` in `lib.sh`: on the
# Mac that line comes out of the accessibility tree, and here the tray is not in the tree at all.
#
# **`/StatusNotifierItem`, which is where ksni publishes**, and not the Ayatana layout this file was
# written against. `/org/ayatana/NotificationItem/facet` was libayatana-appindicator's path, used by the
# Swift app; the Rust app registers through ksni and the object is at the spec's own path. Corrected
# 2026-09-22 against the running app, the old path answering `UnknownObject`.
ITEM_PATH = "/StatusNotifierItem"
ITEM_IFACE = "org.kde.StatusNotifierItem"
PROPERTIES_IFACE = "org.freedesktop.DBus.Properties"
WATCHER = "org.kde.StatusNotifierWatcher"
WATCHER_PATH = "/StatusNotifierWatcher"


def facet_item(bus):
    """The app's tray item as `(bus name, object path)`, found through the StatusNotifierWatcher.

    **Asked of the watcher, which is where the panel finds it.** The watcher lists every registered item as
    `name/path`; the one whose owner is a `facet-linux` process is the app's. The app holds more than one
    connection on the session bus and only the one ksni registered serves the item: the others answer no
    incoming call, so a call to one of them waits out the 25s timeout and fails with `NoReply`.

    **The owner is asked of the bus daemon rather than of the peers**, so no call here waits on a client that
    does not answer.
    """
    # **`facet-linux`, matched exactly, and the name is the Rust binary's.** `-x` rather than `-f`: an exact
    # match on the process name cannot also match some other command line that happens to contain it.
    found = subprocess.run(["pgrep", "-x", "facet-linux"], capture_output=True, text=True)
    pids = {int(line) for line in found.stdout.split()}
    if not pids:
        raise SystemExit("facet is not running")
    daemon = dbus.Interface(
        bus.get_object("org.freedesktop.DBus", "/org/freedesktop/DBus"), "org.freedesktop.DBus"
    )
    try:
        watcher = dbus.Interface(bus.get_object(WATCHER, WATCHER_PATH), PROPERTIES_IFACE)
        registered = [str(entry) for entry in watcher.Get(WATCHER, "RegisteredStatusNotifierItems")]
    except dbus.DBusException as exc:
        raise SystemExit(f"there is no StatusNotifierWatcher to ask for the tray item: {exc}")
    for entry in registered:
        name, slash, rest = entry.partition("/")
        path = slash + rest if slash else ITEM_PATH
        try:
            owner = int(daemon.GetConnectionUnixProcessID(name))
        except dbus.DBusException:
            continue
        if owner in pids:
            return name, path
    raise SystemExit("facet is running but has no tray item registered with the StatusNotifierWatcher")


def lines(node, depth=0, out=None):
    """Every item of the menu, flattened, as (id, label, kind, enabled)."""
    out = [] if out is None else out
    ident, properties, children = node
    if depth:
        out.append(
            (
                int(ident),
                str(properties.get("label", "")),
                str(properties.get("type", "standard")),
                bool(properties.get("enabled", True)),
            )
        )
    for child in children:
        lines(child, depth + 1, out)
    return out


def menu_of(bus):
    """The item's menu object, at whatever path the item says it is.

    **Asked for rather than assumed**, because the path is the implementation's choice and this file has
    already been wrong about it once: ksni answers `/MenuBar` where libayatana-appindicator answered
    `/org/ayatana/NotificationItem/facet/Menu`. The StatusNotifierItem specification makes `Menu` a
    property of the item for exactly this reason, so reading it survives the next backend too.
    """
    name, item_path = facet_item(bus)
    properties = dbus.Interface(bus.get_object(name, item_path), PROPERTIES_IFACE)
    try:
        path = str(properties.Get(ITEM_IFACE, "Menu"))
    except dbus.DBusException as exc:
        raise SystemExit(f"the tray item publishes no Menu property, so its menu cannot be found: {exc}")
    obj = bus.get_object(name, path, introspect=False)
    return dbus.Interface(obj, MENU_IFACE)


def read(menu):
    _, layout = menu.GetLayout(0, -1, dbus.Array([], signature="s"))
    return lines(layout)


def label_of(bus):
    """What the tray is showing: its label, and the tooltip behind it.

    **Two properties, because the panel decides which it draws.** `XAyatanaLabel` is the text beside
    the icon and is what a person reads at a glance; `ToolTip` is the longer form. A check asserting
    on the figure wants the first and a check asserting on the state usually wants the second, so
    both are printed and the caller greps.

    **Read through Properties.Get rather than GetAll**, because a panel that has never asked for a
    property may leave it unset and `GetAll` then answers a dict missing the key -- which reads as an
    empty label rather than as a property that was never published.
    """
    name, item_path = facet_item(bus)
    properties = dbus.Interface(bus.get_object(name, item_path), PROPERTIES_IFACE)
    out = []
    for key in ("XAyatanaLabel", "Title", "ToolTip"):
        try:
            value = properties.Get(ITEM_IFACE, key)
        except dbus.DBusException:
            continue
        if key == "ToolTip":
            # A struct: icon name, icon pixmaps, title, description. The words are the last two.
            try:
                value = " ".join(str(part) for part in (value[2], value[3]) if part)
            except (IndexError, TypeError):
                value = str(value)
        value = str(value)
        if value:
            out.append(f"{key}: {value}")
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--press",
        metavar="LABEL",
        help="choose the item whose label is this, or begins with it",
    )
    parser.add_argument(
        "--activate",
        action="store_true",
        help="send the item the Activate call a left click on the panel sends",
    )
    parser.add_argument(
        "--label",
        action="store_true",
        help="print what the tray icon itself is showing, rather than its menu",
    )
    arguments = parser.parse_args()

    bus = dbus.SessionBus()

    # **The left click, as the panel delivers it.** A StatusNotifierHost turns a left click into this one
    # method call on the item, so calling it is the gesture itself minus the pointer: addressed at the app
    # and nowhere else, which a synthetic click on the panel would not be. Measured against MATE's own
    # left click on 2026-09-25 (handover-linux 10): the same `Status item left clicked` row.
    if arguments.activate:
        name, item_path = facet_item(bus)
        item = dbus.Interface(bus.get_object(name, item_path), ITEM_IFACE)
        item.Activate(dbus.Int32(0), dbus.Int32(0))
        print("activated the status item")
        return 0

    if arguments.label:
        lines_out = label_of(bus)
        if not lines_out:
            print("the tray item publishes no label, title or tooltip", file=sys.stderr)
            return 1
        for line in lines_out:
            print(line)
        return 0

    menu = menu_of(bus)
    items = read(menu)

    if arguments.press is None:
        for ident, label, kind, enabled in items:
            print(f"{ident:>3}  {kind:<10} {label!r}{'' if enabled else '   (insensitive)'}")
        return 0

    for ident, label, _, enabled in items:
        if label != arguments.press and not label.startswith(arguments.press):
            continue
        if not enabled:
            print(f"the item {label!r} is insensitive, so it cannot be chosen", file=sys.stderr)
            return 1
        # The third argument is a variant, and python-dbus cannot infer that from a bare string:
        # without the signature the call is refused as `(issu)` against the expected `(isvu)`.
        menu.Event(dbus.Int32(ident), "clicked", dbus.String("", variant_level=1),
                   dbus.UInt32(0), signature="isvu")
        print(f"pressed {label!r}")
        return 0

    print(f"no menu item matching {arguments.press!r}", file=sys.stderr)
    for _, label, _, _ in items:
        print(f"  the menu holds {label!r}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
