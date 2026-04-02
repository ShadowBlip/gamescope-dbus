
# XWayland DBus Interface API

## org.shadowblip.Gamescope.XWayland.Primary

### Properties


| Name | Access | Type | Description |
| --- | :---: | :---: | --- |
| **AllowTearing** | *readwrite* | *b* |  |
| **BlurMode** | *readwrite* | *u* |  |
| **BlurRadius** | *readwrite* | *u* |  |
| **DisplayRefreshRate** | *read* | *u* |  |
| **FocusableApps** | *read* | *au* |  |
| **FocusableWindowNames** | *read* | *as* |  |
| **FocusableWindows** | *read* | *au* |  |
| **FocusedApp** | *read* | *u* |  |
| **FocusedAppGfx** | *read* | *u* |  |
| **FocusedWindow** | *read* | *u* |  |
| **FpsLimit** | *readwrite* | *u* |  |
| **HdrEnabled** | *readwrite* | *b* |  |
| **HdrSupported** | *read* | *b* |  |
| **IsDisplayExternal** | *read* | *b* |  |
| **OverlayFocused** | *read* | *b* |  |
| **VrrEnabled** | *readwrite* | *b* |  |
| **VrrSupported** | *read* | *b* |  |

### Methods

#### IsFocusableApp



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *b* |  |


#### SetMainApp



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |


#### SetInputFocus



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **value** | *in* | *u* |  |


#### GetOverlay



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *u* |  |


#### SetOverlay



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **value** | *in* | *u* |  |


#### SetNotification



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **value** | *in* | *u* |  |


#### SetExternalOverlay



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **value** | *in* | *u* |  |


#### GetBaselayerAppId



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **** | *out* | *u* |  |


#### SetBaselayerAppId



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **app\_id** | *in* | *u* |  |


#### RemoveBaselayerAppId




#### GetBaselayerWindow



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **** | *out* | *u* |  |


#### SetBaselayerWindow



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |


#### RemoveBaselayerWindow




#### RequestScreenshot




#### SetModeControl



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **xwayland\_id** | *in* | *u* |  |
| **width** | *in* | *u* |  |
| **height** | *in* | *u* |  |
| **super\_res** | *in* | *u* |  |



### Signals

#### BaselayerAppIdUpdated




#### BaselayerWindowUpdated




#### WindowCreated



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | ** | *u* |  |


## org.freedesktop.DBus.Introspectable

### Methods

#### Introspect



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **** | *out* | *s* |  |



### Signals

## org.freedesktop.DBus.Peer

### Methods

#### Ping




#### GetMachineId



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **** | *out* | *s* |  |



### Signals

## org.freedesktop.DBus.Properties

### Methods

#### Get



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **interface\_name** | *in* | *s* |  |
| **property\_name** | *in* | *s* |  |
| **** | *out* | *v* |  |


#### Set



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **interface\_name** | *in* | *s* |  |
| **property\_name** | *in* | *s* |  |
| **value** | *in* | *v* |  |


#### GetAll



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **interface\_name** | *in* | *s* |  |
| **** | *out* | *a{sv}* |  |



### Signals

#### PropertiesChanged



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **interface\_name** | ** | *s* |  |
| **changed\_properties** | ** | *a{sv}* |  |
| **invalidated\_properties** | ** | *as* |  |


## org.shadowblip.Gamescope.XWayland

### Properties


| Name | Access | Type | Description |
| --- | :---: | :---: | --- |
| **Name** | *read* | *s* |  |
| **Primary** | *read* | *b* |  |
| **RootWindowId** | *read* | *u* |  |
| **WatchedWindows** | *read* | *au* |  |

### Methods

#### WatchWindow



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |


#### UnwatchWindow



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |


#### GetPidsForWindow



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *au* |  |


#### GetWindowsForPid



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **pid** | *in* | *u* |  |
| **** | *out* | *au* |  |


#### GetWindowName



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *s* |  |


#### GetGeometryForWindow



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *(qqnn)* |  |


#### GetWindowChildren



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *au* |  |


#### GetAllWindows



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *au* |  |


#### GetExternalOverlay



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *u* |  |


#### GetAppId



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *u* |  |


#### SetAppId



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **app\_id** | *in* | *u* |  |


#### RemoveAppId



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |


#### HasAppId



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window\_id** | *in* | *u* |  |
| **** | *out* | *b* |  |



### Signals

#### WindowPropertyChanged



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **window** | ** | *u* |  |
| **prop** | ** | *s* |  |


#### WindowLifecycle



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **event** | ** | *s* |  |
| **window\_id** | ** | *u* |  |
| **is\_primary** | ** | *b* |  |

