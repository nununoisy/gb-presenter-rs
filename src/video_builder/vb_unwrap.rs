use ffmpeg_next::{Error, format};

pub trait VideoBuilderUnwrap<T> {
    fn vb_unwrap(self) -> Result<T, String>;
}

impl<T> VideoBuilderUnwrap<T> for Result<T, Error> {
    fn vb_unwrap(self) -> Result<T, String> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(format!("FFMPEG error: {}", e))
        }
    }
}

impl<T> VideoBuilderUnwrap<T> for Result<T, format::pixel::ParsePixelError> {
    fn vb_unwrap(self) -> Result<T, String> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(format!("FFMPEG pixel parsing error: {}", e))
        }
    }
}
