use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use gamescope_wayland_client::mangoapp::{message::MangoAppMsgV1, queue::MangoAppMsgQueue};

pub struct MangoAppMsgQueueReader {
    queue: MangoAppMsgQueue,
    rx: std::sync::mpsc::Receiver<()>,
    tx: tokio::sync::watch::Sender<Option<MangoAppMsgV1>>,
}

impl MangoAppMsgQueueReader {
    pub fn new(
        rx: std::sync::mpsc::Receiver<()>,
        tx: tokio::sync::watch::Sender<Option<MangoAppMsgV1>>,
    ) -> Self {
        log::debug!("Starting new mango app msg queue reader");

        let queue = MangoAppMsgQueue::new();
        Self { queue, rx, tx }
    }

    pub fn run(mut self, cancel_flag: Arc<AtomicBool>) {
        while !cancel_flag.load(Ordering::SeqCst) && self.rx.recv().is_ok() {
            let msg = match self.queue.recv() {
                Ok(msg) => msg,
                Err(err) => {
                    log::debug!("Failed to read from queue: {err}");
                    if let Err(err) = self.tx.send(None) {
                        log::error!("Error sending mango app error msg response, err:{err:?}");
                    }
                    continue;
                }
            };

            if let Err(err) = self.tx.send(Some(msg)) {
                log::error!("Error sending mango app msg response, err:{err:?}");
            }
        }
        log::debug!("Stopped reading from message queue");
    }
}
