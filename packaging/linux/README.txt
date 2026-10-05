Facet @VERSION@ for Linux (@ARCH@)
==================================

Facet records your time from a TimeFlip2 device. Turn the device, and the face that
lands up is what you are working on.

Running it
----------

    tar xzf Facet-@VERSION@-linux-@ARCH@.tar.gz
    cd Facet-@VERSION@-linux-@ARCH@
    ./facet-linux

Facet puts an icon in the system tray and has no window until you ask for one: open the
tray icon's menu and choose Settings. With no device paired, Facet times by hand: pick a
category on the Faces tab and the clock starts on it.

To have it in your applications menu, copy facet-linux to a folder on your PATH (for
example ~/.local/bin), facet.svg to ~/.local/share/icons/hicolor/scalable/apps/facet.svg,
and facet.desktop to ~/.local/share/applications/.

What it needs
-------------

 - Linux on @ARCH@ with glibc @GLIBC@ or newer, which is the lowest version the program asks
   for. It was built on Linux Mint 22.3, and older distributions are untested.
 - BlueZ, the Linux Bluetooth service, running, and a Bluetooth adapter.
 - libdbus-1, libfontconfig and the usual X11 or Wayland libraries of a desktop.
 - A system tray that can show StatusNotifierItem icons. The Linux Mint MATE panel does,
   which is where Facet was tested. GNOME needs the AppIndicator extension.
 - A Secret Service provider, such as GNOME Keyring, to keep your device PIN. Without one,
   Facet keeps the PIN in config.json in its data folder instead.

It was tested on Linux Mint 22.3 with the MATE desktop and no other.

Where things are kept
---------------------

Your recorded time is in ~/.local/share/Facet. Nothing is sent anywhere unless you connect
a Google account yourself.

More
----

User guide, privacy policy and terms: https://facet.tux.com.au/
Questions and faults: facet@tux.com.au
Licence: LICENSE.md and NOTICE.md in this folder.
