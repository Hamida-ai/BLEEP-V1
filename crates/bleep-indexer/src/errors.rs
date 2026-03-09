use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("Ingestion channel closed — indexer is shutting down")]
    ChannelClosed,

    #[error("Block not found: height={0}")]
    BlockNotFound(u64),

    #[error("Transaction not found: {0}")]
    TxNotFound(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Reorg error: {0}")]
    ReorgError(String),

    #[error("Serialisation error: {0}")]
    SerialisationError(String),

    #[error("Index inconsistency: {0}")]
    Inconsistency(String),

    #[error("Checkpoint error: {0}")]
    CheckpointError(String),
}

pub type IndexerResult<T> = Result<T, IndexerError>;
