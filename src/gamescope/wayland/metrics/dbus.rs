use std::time::SystemTime;

use gamescope_wayland_client::mangoapp::{
    message::MangoAppMsgV1,
    queue::{MangoAppMsgQueue, QueueError},
};
use thiserror::Error;
use zbus::{dbus_interface, fdo};

#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("failed to read metrics message from gamescope: {0}")]
    QueueError(#[from] QueueError),
}

impl From<MetricsError> for fdo::Error {
    fn from(value: MetricsError) -> Self {
        match value {
            MetricsError::QueueError(queue_error) => fdo::Error::Failed(queue_error.to_string()),
        }
    }
}

pub struct DBusInterface {
    queue: MangoAppMsgQueue,
    state: Option<MangoAppMsgV1>,
    last_update: SystemTime,
}

impl DBusInterface {
    pub fn new() -> Self {
        let queue = MangoAppMsgQueue::new();
        Self {
            queue,
            state: None,
            last_update: SystemTime::now(),
        }
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.Wayland.Metrics")]
impl DBusInterface {
    /// Update the metrics from gamescope. This should be called regularly to
    /// get the latest frame metrics. Note that calling this frequently can
    /// interfere with MangoApp framerate stats.
    pub fn update(&mut self) -> fdo::Result<()> {
        let msg = self.queue.recv().map_err(MetricsError::from)?;
        self.state = Some(msg);
        self.last_update = SystemTime::now();
        Ok(())
    }

    /// Unix timestamp (in milliseconds) when `update()` was last called.
    #[dbus_interface(property)]
    pub fn last_update_time(&self) -> fdo::Result<u64> {
        let ts = self
            .last_update
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        Ok(ts.as_millis() as u64)
    }

    #[dbus_interface(property)]
    pub fn app_frametime_ns(&self) -> fdo::Result<u64> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.app_frametime().as_nanos() as u64)
    }

    #[dbus_interface(property)]
    pub fn pid(&self) -> fdo::Result<u32> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.pid())
    }

    #[dbus_interface(property)]
    pub fn fsr_upscale(&self) -> fdo::Result<u8> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.fsr_upscale())
    }

    #[dbus_interface(property)]
    pub fn fsr_sharpness(&self) -> fdo::Result<u8> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.fsr_sharpness())
    }

    #[dbus_interface(property)]
    pub fn visible_frametime_ns(&self) -> fdo::Result<u64> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.visible_frametime().as_nanos() as u64)
    }

    #[dbus_interface(property)]
    pub fn latency_ns(&self) -> fdo::Result<u64> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.latency().as_nanos() as u64)
    }

    #[dbus_interface(property)]
    pub fn output_width(&self) -> fdo::Result<u32> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.output_width())
    }

    #[dbus_interface(property)]
    pub fn output_height(&self) -> fdo::Result<u32> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.output_height())
    }

    #[dbus_interface(property)]
    pub fn display_refresh(&self) -> fdo::Result<u16> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.display_refresh())
    }

    #[dbus_interface(property)]
    pub fn app_wants_hdr(&self) -> fdo::Result<bool> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.app_wants_hdr())
    }

    #[dbus_interface(property)]
    pub fn overlay_focused(&self) -> fdo::Result<bool> {
        let Some(state) = self.state.as_ref() else {
            return Ok(Default::default());
        };
        Ok(state.steam_focused())
    }
}
