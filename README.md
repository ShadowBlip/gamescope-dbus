<br>
<h1 align="center">
  Gamescope DBus
</h1>

<p align="center">
  <a href="https://github.com/ShadowBlip/gamescope-dbus/stargazers"><img src="https://img.shields.io/github/stars/ShadowBlip/gamescope-dbus" /></a>
  <a href="https://github.com/ShadowBlip/gamescope-dbus/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ShadowBlip/gamescope-dbus" /></a>
  <a href="https://discord.gg/fKsUbrt"><img src="https://img.shields.io/badge/discord-server-%235865F2" /></a>
  <br>
</p>

## About

Gamescope DBus is an open source daemon for Linux that can be used to control the [Gamescope](https://github.com/ValveSoftware/gamescope)
compositor over [DBus](https://www.freedesktop.org/wiki/Software/dbus/). This interface provides
a UI-agnostic way to interact with Gamescope to make it easier to write your own
user interface and integrate it with Gamescope.

## Install

You can install with:

```bash
make build
sudo make install
```

If you are using ArchLinux, you can install Gamescope DBus from the AUR:

```bash
yay -S gamescope-dbus-bin
```

Then start the service with:

```bash
systemctl --user enable --now gamescope-dbus
```

## Documentation

XML specifications for all interfaces can be found in [bindings/dbus-xml](./bindings/dbus-xml).

Individual interface documentation can be found here:

- [org.shadowblip.Gamescope.Manager](./docs/manager.md)
- [org.shadowblip.Gamescope.XWayland](./docs/xwayland.md)
- [org.shadowblip.Gamescope.Wayland](./docs/wayland.md)

## Usage

When Gamescope DBus is running as a service, you can interact with it over DBus.
There are various DBus libraries available for popular programming languages
like Python, Rust, C++, etc.

You can also interface with DBus using the `busctl` command:

```bash
busctl --user tree org.shadowblip.Gamescope
```

```bash
└─ /org
  └─ /org/shadowblip
    └─ /org/shadowblip/Gamescope
      ├─ /org/shadowblip/Gamescope/Manager
      ├─ /org/shadowblip/Gamescope/Wayland0
      ├─ /org/shadowblip/Gamescope/XWayland0
      └─ /org/shadowblip/Gamescope/XWayland1
```

```bash
busctl --user introspect org.shadowblip.Gamescope /org/shadowblip/Gamescope/XWayland0
```

```bash
NAME                                      TYPE      SIGNATURE RESULT/VALUE            FLAGS
org.freedesktop.DBus.Introspectable       interface -         -                       -
.Introspect                               method    -         s                       -
org.freedesktop.DBus.Peer                 interface -         -                       -
.GetMachineId                             method    -         s                       -
.Ping                                     method    -         -                       -
org.freedesktop.DBus.Properties           interface -         -                       -
.Get                                      method    ss        v                       -
.GetAll                                   method    s         a{sv}                   -
.Set                                      method    ssv       -                       -
.PropertiesChanged                        signal    sa{sv}as  -                       -
org.shadowblip.Gamescope.XWayland         interface -         -                       -
.GetAllWindows                            method    u         au                      -
.GetAppId                                 method    u         u                       -
.GetWindowChildren                        method    u         au                      -
.GetWindowName                            method    u         s                       -
.HasAppId                                 method    u         b                       -
.SetAppId                                 method    uu        -                       -
.Name                                     property  s         ":2"                    emits-change
.Primary                                  property  b         true                    emits-change
.RootWindowId                             property  u         962                     emits-change
org.shadowblip.Gamescope.XWayland.Primary interface -         -                       -
.GetBaselayerWindow                       method    -         u                       -
.GetOverlay                               method    u         u                       -
.IsFocusableApp                           method    u         b                       -
.RemoveBaselayerWindow                    method    -         -                       -
.RequestScreenshot                        method    -         -                       -
.SetBaselayerWindow                       method    u         -                       -
.SetExternalOverlay                       method    uu        -                       -
.SetInputFocus                            method    uu        -                       -
.SetMainApp                               method    u         -                       -
.SetNotification                          method    uu        -                       -
.SetOverlay                               method    uu        -                       -
.AllowTearing                             property  b         false                   emits-change writable
.BlurMode                                 property  u         0                       emits-change writable
.BlurRadius                               property  u         0                       emits-change writable
.FocusableApps                            property  au        1 4194306               emits-change
.FocusableWindowNames                     property  as        -                       emits-change
.FocusableWindows                         property  au        3 4194306 4194306 64337 emits-change
.FocusedApp                               property  u         0                       emits-change
.FocusedAppGfx                            property  u         0                       emits-change
.FocusedWindow                            property  u         4194306                 emits-change
.FpsLimit                                 property  u         0                       emits-change writable
.OverlayFocused                           property  b         false                   emits-change
.BaselayerWindowUpdated                   signal    -         -                       -
```

## Testing

When Gamescope DBus is running, you can test setting properties with:

```bash
busctl set-property org.shadowblip.Gamescope /org/shadowblip/Gamescope/XWayland0 org.shadowblip.Gamescope.Primary BlurMode "u" 2
```

## License

Gamescope DBus is licensed under THE GNU GPLv3+. See LICENSE for details.
