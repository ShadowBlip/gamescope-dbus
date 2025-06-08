use std::time::{Duration, SystemTime};

use gamescope_wayland_client::mangoapp::message::MangoAppMsgV1;
use thiserror::Error;
use tokio::{
    sync::{
        mpsc::{self, error::SendTimeoutError},
        watch,
    },
    time::{error::Elapsed, timeout},
};
use zbus::{dbus_interface, fdo};

use crate::gamescope::wayland::metrics::mango_app_reader::MangoAppMsgQueueReader;

const TIMEOUT_DURATION: Duration = Duration::from_millis(200);

#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("failed to read metrics message from gamescope: {0}")]
    QueueError(#[from] watch::error::RecvError),
    #[error("timed out trying to read gamescope queue message: {0}")]
    QueueTimeout(#[from] Elapsed),
    #[error("timed out requesting message from gamescope queue")]
    RequestTimeout(#[from] SendTimeoutError<()>),
}

impl From<MetricsError> for fdo::Error {
    fn from(value: MetricsError) -> Self {
        match value {
            MetricsError::QueueError(queue_error) => fdo::Error::Failed(queue_error.to_string()),
            MetricsError::QueueTimeout(queue_error) => fdo::Error::Timeout(queue_error.to_string()),
            MetricsError::RequestTimeout(queue_error) => {
                fdo::Error::Timeout(queue_error.to_string())
            }
        }
    }
}

pub struct DBusInterface {
    state: Option<MangoAppMsgV1>,
    last_update: SystemTime,
    request_tx: mpsc::Sender<()>,
    response_rx: watch::Receiver<Option<MangoAppMsgV1>>,
}

impl DBusInterface {
    pub fn new() -> Self {
        let (response_tx, response_rx) = watch::channel::<Option<MangoAppMsgV1>>(None);
        let (request_tx, request_rx) = mpsc::channel(256);

        tokio::task::spawn_blocking(move || {
            let reader = MangoAppMsgQueueReader::new(request_rx, response_tx);
            reader.run();
        });

        Self {
            state: None,
            last_update: SystemTime::now(),
            request_tx,
            response_rx,
        }
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.Wayland.Metrics")]
impl DBusInterface {
    /// Update the metrics from gamescope. This should be called regularly to
    /// get the latest frame metrics. Note that calling this frequently can
    /// interfere with MangoApp framerate stats.
    pub async fn update(&mut self) -> fdo::Result<()> {
        self.request_tx
            .send_timeout((), TIMEOUT_DURATION)
            .await
            .map_err(MetricsError::from)?;

        timeout(TIMEOUT_DURATION, self.response_rx.changed())
            .await
            .map_err(MetricsError::from)?
            .map_err(MetricsError::from)?;

        let msg = &*self.response_rx.borrow();
        let Some(msg) = msg else {
            return Ok(());
        };
        self.state = Some(*msg);
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
