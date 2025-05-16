use std::array::TryFromSliceError;

use crate::gamescope::mangoapp::{
    message::{MangoAppMsgHeader, MangoAppMsgV1, MSG_HEADER_SIZE, MSG_V1_SIZE},
    sysvipc::{ftok, msgget, msgrcv, IPC_CREAT},
};
use packed_struct::prelude::*;
use thiserror::Error;

const BUFFER_SIZE: usize = 1024;

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("invalid mangoapp message size reading from queue: {0}")]
    InvalidSize(usize),
    #[error("unsupported mangoapp struct version: {0}")]
    UnsupportedVersion(u32),
    #[error("buffer size is not big enough to unpack message")]
    BadBufferSize(#[from] TryFromSliceError),
    #[error("unable to unpack message")]
    UnpackingError(#[from] PackingError),
}

/// Queue for reading mangoapp messages from Gamescope
pub struct MangoAppMsgQueue {
    buffer: [u8; BUFFER_SIZE],
    msg_queue_id: i32,
}

impl MangoAppMsgQueue {
    /// Create a new queue instance to read [MangoAppMsgV1] messages from Gamescope
    pub fn new() -> Self {
        let key = ftok("mangoapp", 65);
        let msgqid = msgget(key, 0o0666 | IPC_CREAT);
        let buffer = [0u8; 1024];

        Self {
            buffer,
            msg_queue_id: msgqid,
        }
    }

    /// Receives a [MangoAppMsgV1] from the queue. Blocks until a message has been sent.
    pub fn recv(&mut self) -> Result<MangoAppMsgV1, QueueError> {
        let bytes_read = msgrcv(self.msg_queue_id, &mut self.buffer, BUFFER_SIZE, 1, 0);
        if bytes_read < MSG_HEADER_SIZE {
            return Err(QueueError::InvalidSize(bytes_read));
        }

        let slice = &self.buffer[0..MSG_HEADER_SIZE].try_into()?;
        let msg = MangoAppMsgHeader::unpack(slice)?;
        if msg.version() != 1 {
            return Err(QueueError::UnsupportedVersion(msg.version()));
        }

        let slice = &self.buffer[0..MSG_V1_SIZE].try_into()?;
        let msg = MangoAppMsgV1::unpack(slice)?;

        Ok(msg)
    }
}

impl Default for MangoAppMsgQueue {
    fn default() -> Self {
        Self::new()
    }
}
