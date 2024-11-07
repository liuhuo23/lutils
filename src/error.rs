use thiserror::Error;

#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("I/O error")]
    IoError(#[from] std::io::Error),
    #[error("Blkid error: `{0}`")]
    BlkidError(String),
    #[error("Device error: `{0}`")]
    DeviceError(String),
    #[error("Block error: `{0}`")]
    BlockError(String),
}
