use std::ffi::{c_int, CString};
use std::mem;
use ffmpeg_next::{ChannelLayout, codec, Codec, Error, filter, format, StreamMut};
use ffmpeg_sys_next::{AV_CODEC_CAP_VARIABLE_FRAME_SIZE, av_get_sample_fmt, AV_OPT_SEARCH_CHILDREN, av_opt_set_bin, avcodec_alloc_context3, avcodec_parameters_from_context, avcodec_parameters_to_context, AVSampleFormat};

pub fn ffmpeg_create_context(codec: Codec, parameters: codec::Parameters) -> Result<codec::Context, String> {
    // ffmpeg-next does not provide a way to pass a codec to avcodec_alloc_context3, which
    // is necessary for initializing certain contexts (e.g. mp4/libx264).
    // Safety: The return value of avcodec_alloc_context3() is checked to ensure that the allocation
    //         succeeded.
    // Safety: The allocated context is wrapped in a safe abstraction, which handles freeing the
    //         associated resources later.
    // Safety: The value of avcodec_parameters_to_context is checked to ensure errors are handled.
    unsafe {
        let context = avcodec_alloc_context3(codec.as_ptr());
        if context.is_null() {
            return Err("FFMPEG error: avcodec_alloc_context3() failed".to_string());
        }

        let mut context = codec::Context::wrap(context, None);
        match avcodec_parameters_to_context(context.as_mut_ptr(), parameters.as_ptr()) {
            0 => Ok(context),
            e => Err(Error::from(e).to_string())
        }
    }
}

pub fn ffmpeg_copy_context_params(stream: &mut StreamMut, context: &codec::Context) -> Result<(), String> {
    // This context copy is required to fully initialize some codecs (e.g. AAC). ffmpeg-next does not
    // provide a safe abstraction so it must be done here.
    // Safety: The value of avcodec_parameters_from_context is checked to ensure errors are handled.
    // Safety: All mutable pointer dereferences are done strictly on initialized memory since they
    //         come from a mutable reference to a safe abstraction.
    unsafe {
        match avcodec_parameters_from_context((*stream.as_mut_ptr()).codecpar, context.as_ptr()) {
            0 => Ok(()),
            e => Err(Error::from(e).to_string())
        }
    }
}

pub fn ffmpeg_copy_codec_params(stream: &mut StreamMut, context: &codec::Context, codec: &Codec) -> Result<(), String> {
    // This augmented context copy is required to initialize some codecs. ffmpeg-next does not
    // provide a safe abstraction so it must be done here.
    // Safety: All mutable pointer dereferences are done strictly on initialized memory since they
    //         come from a mutable reference to a safe abstraction.
    unsafe {
        ffmpeg_copy_context_params(stream, context)?;
        (*(*stream.as_mut_ptr()).codecpar).codec_id = codec.id().into();
        (*(*stream.as_mut_ptr()).codecpar).codec_type = codec.medium().into();
    }
    Ok(())
}

pub fn ffmpeg_sample_format_from_string(value: &str) -> format::Sample {
    // This is provided by ffmpeg-next, but only for `&'static str`, presumably due to
    // some confusion over the `const char*` in the method signature?
    unsafe {
        let value = CString::new(value).unwrap();

        format::Sample::from(av_get_sample_fmt(value.as_ptr()))
    }
}

pub fn ffmpeg_set_audio_stream_frame_size(stream: &mut StreamMut, variable_frame_size: usize) -> usize {
    unsafe {
        let frame_size = (*(*stream.as_ptr()).codecpar).frame_size as usize;
        if frame_size == 0 || (frame_size & AV_CODEC_CAP_VARIABLE_FRAME_SIZE as usize) != 0 {
            (*(*stream.as_mut_ptr()).codecpar).frame_size = variable_frame_size as _;
            return variable_frame_size;
        }
        frame_size
    }
}

pub fn ffmpeg_context_bytes_written(context: &format::context::Output) -> usize {
    #[cfg(not(feature = "ffmpeg_6_0"))]
    let bytes_written = unsafe { (*(*context.as_ptr()).pb).written };
    #[cfg(feature = "ffmpeg_6_0")]
    let bytes_written = unsafe { (*(*context.as_ptr()).pb).bytes_written };
    std::cmp::max(bytes_written, 0) as usize
}
