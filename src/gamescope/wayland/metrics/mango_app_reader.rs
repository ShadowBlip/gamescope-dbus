use gamescope_wayland_client::mangoapp::{message::MangoAppMsgV1, queue::MangoAppMsgQueue};
use tokio::sync::{mpsc, watch};

pub struct MangoAppMsgQueueReader {
    queue: MangoAppMsgQueue,
    rx: mpsc::Receiver<()>,
    tx: watch::Sender<Option<MangoAppMsgV1>>,
}

impl MangoAppMsgQueueReader {
    pub fn new(rx: mpsc::Receiver<()>, tx: watch::Sender<Option<MangoAppMsgV1>>) -> Self {
        log::debug!("Starting new mango app msg queue reader");

        let queue = MangoAppMsgQueue::new();
        Self { queue, rx, tx }
    }

    pub fn run(mut self) {
        while self.rx.blocking_recv().is_some() {
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
