use thiserror::Error;

#[derive(Error, Debug)]
pub enum TranscodeError {
    #[error("Transcode failed: {0}")]
    Transcode(String),
    #[error("Unsupported sample rate {0}Hz for file {1}")]
    UnknownSampleRate(u32, String),
    #[error("Multichannel releases are unsupported")]
    Downmix,
    #[error("Tag check failed: {0}")]
    TagCheck(String),
    #[error("Tagging failed: {0}")]
    Tagging(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("API request failed: {0}")]
    Request(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Login failed")]
    Login,
}
