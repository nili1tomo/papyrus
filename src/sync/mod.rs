mod sources;

use futures_util::pin_mut;
use log::info;
use tokio_stream::StreamExt;

use crate::starknet_client::ClientError;
use crate::storage::{SNStorageWriter, StorageError, StorageWriter};

pub use self::sources::CentralSource;

// Orchestrates specific network interfaces (e.g. central, p2p, l1) and writes to Storage.
pub struct StateSync {
    central_source: CentralSource,
    writer: SNStorageWriter,
}

#[derive(thiserror::Error, Debug)]
pub enum StateSyncError {
    #[error(transparent)]
    StorageError(#[from] StorageError),
    #[error(transparent)]
    CentralSourceError(#[from] ClientError),
}

#[allow(clippy::new_without_default)]
impl StateSync {
    pub fn new(central_source: CentralSource, writer: SNStorageWriter) -> StateSync {
        StateSync {
            central_source,
            writer,
        }
    }
    pub async fn run(&mut self) -> anyhow::Result<(), StateSyncError> {
        info!("State sync started.");
        let initial_block_number = self.central_source.get_block_number().await?;
        let stream = self.central_source.stream_new_blocks(initial_block_number);
        pin_mut!(stream);
        while let Some(received) = stream.next().await {
            info!("Received new block number: {:?}.", received);
            self.writer.set_latest_block_number(received).await?;
        }

        Ok(())
    }
}
