
# Wayland DBus Interface API

## org.freedesktop.DBus.Introspectable

### Methods

#### Introspect



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **** | *out* | *s* |  |



### Signals

## org.shadowblip.Gamescope.Wayland.Metrics

### Properties


| Name | Access | Type | Description |
| --- | :---: | :---: | --- |
| **AppFrametimeNs** | *read* | *t* |  |
| **AppWantsHdr** | *read* | *b* |  |
| **DisplayRefresh** | *read* | *q* |  |
| **FsrSharpness** | *read* | *y* |  |
| **FsrUpscale** | *read* | *y* |  |
| **LastUpdateTime** | *read* | *t* |  |
| **LatencyNs** | *read* | *t* |  |
| **OutputHeight** | *read* | *u* |  |
| **OutputWidth** | *read* | *u* |  |
| **OverlayFocused** | *read* | *b* |  |
| **Pid** | *read* | *u* |  |
| **VisibleFrametimeNs** | *read* | *t* |  |

### Methods

#### Update





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


## org.shadowblip.Gamescope.Wayland

### Properties


| Name | Access | Type | Description |
| --- | :---: | :---: | --- |
| **RefreshRates** | *read* | *au* |  |

### Methods

#### TakeScreenshot



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **file\_path** | *in* | *s* |  |
| **screenshot\_type** | *in* | *y* |  |


#### DisplaySleep



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **display\_type\_flags** | *in* | *y* |  |
| **sleep** | *in* | *b* |  |


#### SetAppTargetRefreshCycle



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **fps** | *in* | *u* |  |
| **refresh\_cycle\_flags** | *in* | *y* |  |


#### RequestAppPerformanceStats



##### Arguments

| Name | Direction | Type | Description |
| --- | :---: | :---: | --- |
| **app\_id** | *in* | *u* |  |
| **** | *out* | *t* |  |



### Signals
