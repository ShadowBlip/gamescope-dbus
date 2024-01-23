# Gamescope DBus
Provides a DBus interface to Gamescope

## Architecture

```
└─ /org
  └─ /org/shadowblip
    └─ /org/shadowblip/Gamescope
      ├─ /org/shadowblip/Gamescope/Manager
      ├─ /org/shadowblip/Gamescope/Wayland0
      │ ├─ /org/shadowblip/Gamescope/XWayland0
      │ └─ /org/shadowblip/Gamescope/XWayland1
      └─ /org/shadowblip/Gamescope/Wayland1
        └─ /org/shadowblip/Gamescope/XWayland0
```
