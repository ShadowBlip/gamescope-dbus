use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, SystemTime},
};

use gamescope_wayland_client::mangoapp::message::MangoAppMsgV1;
use thiserror::Error;
use tokio::time::{error::Elapsed, timeout};
use zbus::{dbus_interface, fdo};

use crate::gamescope::wayland::metrics::mango_app_reader::MangoAppMsgQueueReader;

#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("failed to read metrics message from gamescope: {0}")]
    QueueError(#[from] tokio::sync::watch::error::RecvError),
    #[error("timedout trying to read gamescope queue message: {0}")]
    QueueTimeout(#[from] Elapsed),
}

impl From<MetricsError> for fdo::Error {
    fn from(value: MetricsError) -> Self {
        match value {
            MetricsError::QueueError(queue_error) => fdo::Error::Failed(queue_error.to_string()),
            MetricsError::QueueTimeout(queue_error) => fdo::Error::Failed(queue_error.to_string()),
        }
    }
}

pub struct DBusInterface {
    handle: Option<JoinHandle<()>>,
    cancel_flag: Arc<AtomicBool>,
    state: Option<MangoAppMsgV1>,
    last_update: SystemTime,
    request_tx: std::sync::mpsc::Sender<()>,
    response_rx: tokio::sync::watch::Receiver<Option<MangoAppMsgV1>>,
}

impl Drop for DBusInterface {
    fn drop(&mut self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
        let _ = self.request_tx.send(());

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl DBusInterface {
    pub fn new() -> Self {
        let (response_tx, response_rx) = tokio::sync::watch::channel::<Option<MangoAppMsgV1>>(None);
        let (request_tx, request_rx) = std::sync::mpsc::channel::<()>();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag_thread = cancel_flag.clone();

        let handle = std::thread::spawn(move || {
            let reader = MangoAppMsgQueueReader::new(request_rx, response_tx);
            reader.run(cancel_flag_thread);
        });

        Self {
            handle: Some(handle),
            cancel_flag,
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
        if let Err(err) = self.request_tx.send(()) {
            log::error!("Error requesting mangoapp queue update, err:{err:?}");
            return Err(fdo::Error::Failed("Mangoapp queue reader error".to_owned()));
        }

        timeout(Duration::from_millis(10), self.response_rx.changed())
            .await
            .map_err(MetricsError::from)?
            .map_err(MetricsError::from)?;

        let msg = &*self.response_rx.borrow();

        if let Some(msg) = msg {
            if self.state.is_none_or(|state| state != *msg) {
                self.state = Some(*msg);
                self.last_update = SystemTime::now();
            }
        }

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
