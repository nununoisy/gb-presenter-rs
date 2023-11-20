use anyhow::{Result, anyhow};
use ffmpeg_next::{Error, format};

pub trait VideoBuilderUnwrap<T> {
    fn vb_unwrap(self) -> Result<T>;
}

impl<T> VideoBuilderUnwrap<T> for Result<T, Error> {
    fn vb_unwrap(self) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!("FFMPEG error: {}", e))
        }
    }
}

impl<T> VideoBuilderUnwrap<T> for Result<T, format::pixel::ParsePixelError> {
    fn vb_unwrap(self) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!("FFMPEG pixel parsing error: {}", e))
        }
    }
}
