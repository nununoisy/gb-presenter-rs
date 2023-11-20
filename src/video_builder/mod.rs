pub mod video_options;
mod vb_unwrap;
mod ffmpeg_hacks;
mod encoding;
pub mod backgrounds;

use anyhow::{Result, Context};
use std::collections::VecDeque;
use std::{mem, slice};
use std::str::FromStr;
use ffmpeg_next::{self, format, encoder, codec, ChannelLayout, Dictionary, software, frame};
use video_options::VideoOptions;
use vb_unwrap::VideoBuilderUnwrap;
use crate::video_builder::backgrounds::{get_video_background, VideoBackground};
use crate::video_builder::ffmpeg_hacks::{ffmpeg_copy_codec_params, ffmpeg_copy_context_params, ffmpeg_create_context, ffmpeg_sample_format_from_string, ffmpeg_get_audio_context_frame_size};
pub use ffmpeg_hacks::ffmpeg_version;

pub fn init() -> Result<()> {
    ffmpeg_next::init().context("Initializing FFmpeg")
}

pub fn as_u8_slice<T: Sized>(s: &[T]) -> &[u8] {
    unsafe {
        slice::from_raw_parts(
            s.as_ptr() as *const u8,
            s.len() * mem::size_of::<T>()
        )
    }
}

pub struct VideoBuilder {
    options: VideoOptions,

    background: Option<Box<dyn VideoBackground>>,

    out_ctx: format::context::Output,

    v_encoder: encoder::Video,
    v_swc_ctx: software::scaling::Context,
    v_sws_ctx: software::scaling::Context,
    v_frame_buf: VecDeque<frame::Video>,
    v_stream_idx: usize,
    v_pts: i64,
    v_pts_muxed: i64,

    a_encoder: encoder::Audio,
    a_swr_ctx: software::resampling::Context,
    a_frame_buf: VecDeque<frame::Audio>,
    a_stream_idx: usize,
    a_frame_size: usize,
    a_pts: i64,
    a_pts_muxed: i64
}

impl VideoBuilder {
    pub fn new(options: VideoOptions) -> Result<Self> {
        let mut out_ctx = format::output(&options.output_path).vb_unwrap()?;

        let mut metadata = Dictionary::new();
        for (k, v) in options.metadata.iter() {
            metadata.set(k.as_str(), v.as_str());
        }
        out_ctx.set_metadata(metadata);

        let pix_fmt_in = format::Pixel::from_str(&options.pixel_format_in).vb_unwrap()?;
        let pix_fmt_out = format::Pixel::from_str(&options.pixel_format_out).vb_unwrap()?;
        let channel_layout = ChannelLayout::default(options.audio_channels);

        let aspect_in = options.resolution_in.0 as f32 / options.resolution_in.1 as f32;
        let aspect_out = options.resolution_out.0 as f32 / options.resolution_out.1 as f32;
        let scaling_flags = if aspect_in == aspect_out {
            software::scaling::Flags::POINT
        } else {
            println!("Warning: input and output aspect do not match. Falling back to bilinear scaling");
            software::scaling::Flags::FAST_BILINEAR
        };

        let background = match &options.background_path {
            Some(p) => get_video_background(p, options.resolution_out.0, options.resolution_out.1),
            None => None
        };
        let v_swc_ctx: software::scaling::Context;
        let v_sws_ctx: software::scaling::Context;

        if background.is_some() {
            // Do scaling first since we need to preserve the alpha information before blitting to the background
            v_swc_ctx = software::converter(
                options.resolution_out,
                pix_fmt_in,
                pix_fmt_out
            ).vb_unwrap()?;

            v_sws_ctx = software::scaler(
                pix_fmt_in,
                scaling_flags,
                options.resolution_in,
                options.resolution_out
            ).vb_unwrap()?;
        } else {
            // Do conversion first if there isn't a background since yuv420p is a lot faster to scale than RGBA
            v_swc_ctx = software::converter(
                options.resolution_in,
                pix_fmt_in,
                pix_fmt_out
            ).vb_unwrap()?;

            v_sws_ctx = software::scaler(
                pix_fmt_out,
                scaling_flags,
                options.resolution_in,
                options.resolution_out
            ).vb_unwrap()?;
        }

        let swr_in = (
            ffmpeg_sample_format_from_string(&options.sample_format_in),
            channel_layout,
            options.sample_rate as u32
        );
        let swr_out = (
            ffmpeg_sample_format_from_string(&options.sample_format_out),
            channel_layout,
            options.sample_rate as u32
        );
        let a_swr_ctx = software::resampler(swr_in, swr_out).vb_unwrap()?;

        let (v_encoder, v_stream_idx) = Self::create_video_encoder(options.clone(), &mut out_ctx)?;
        let (a_encoder, a_stream_idx, a_frame_size) = Self::create_audio_encoder(options.clone(), &mut out_ctx)?;

        Ok(Self {
            options,
            background,
            out_ctx,
            v_encoder,
            v_swc_ctx,
            v_sws_ctx,
            v_frame_buf: VecDeque::new(),
            v_stream_idx,
            v_pts: 0,
            v_pts_muxed: 0,
            a_encoder,
            a_swr_ctx,
            a_frame_buf: VecDeque::new(),
            a_stream_idx,
            a_frame_size,
            a_pts: 0,
            a_pts_muxed: 0
        })
    }

    fn create_video_encoder(options: VideoOptions, out_ctx: &mut format::context::Output) -> Result<(encoder::Video, usize)> {
        let global_header = out_ctx.format().flags().contains(format::Flags::GLOBAL_HEADER);
        let output_format = format::Pixel::from_str(&options.pixel_format_out).vb_unwrap()?;
        let codec = encoder::find_by_name(&options.video_codec)
            .with_context(|| format!("Unknown codec {}", options.video_codec))?;

        let mut stream = out_ctx.add_stream(codec).vb_unwrap()?;
        let mut context = ffmpeg_create_context(codec, stream.parameters())?
            .encoder()
            .video()
            .vb_unwrap()?;

        context.set_format(output_format);
        context.set_width(options.resolution_out.0);
        context.set_height(options.resolution_out.1);
        context.set_max_b_frames(2);
        context.set_gop(12);
        context.set_time_base(options.video_time_base);

        let mut flags = codec::Flags::empty();
        if global_header {
            flags.insert(codec::Flags::GLOBAL_HEADER);
        }
        flags.insert(codec::Flags::CLOSED_GOP);  // Needed for Twitter uploads to function properly
        context.set_flags(flags);

        ffmpeg_copy_codec_params(&mut stream, &context, &codec)?;

        stream.set_time_base(options.video_time_base);

        let mut context_options = Dictionary::new();
        // Add some default options for certain codecs
        match codec.id() {
            codec::Id::H264 | codec::Id::H265 => {
                context_options.set("preset", "veryfast");
                context_options.set("crf", "20");
                context_options.set("tune", "film");
            },
            _ => ()
        };
        for (k, v) in options.video_codec_params.iter() {
            context_options.set(k.as_str(), v.as_str());
        }

        let v_encoder = context.open_as_with(codec, context_options).vb_unwrap()?;
        let v_stream_idx = stream.index();

        ffmpeg_copy_context_params(&mut stream, v_encoder.as_ref())?;

        Ok((v_encoder, v_stream_idx))
    }

    fn create_audio_encoder(options: VideoOptions, out_ctx: &mut format::context::Output) -> Result<(encoder::Audio, usize, usize)> {
        let output_format = ffmpeg_sample_format_from_string(&options.sample_format_out);
        let channel_layout = ChannelLayout::default(options.audio_channels);
        let codec = encoder::find_by_name(&options.audio_codec)
            .with_context(|| format!("Unknown codec {}", options.audio_codec))?;

        let mut stream = out_ctx.add_stream(codec).vb_unwrap()?;
        let mut context = ffmpeg_create_context(codec, stream.parameters())?
            .encoder()
            .audio()
            .vb_unwrap()?;

        context.set_rate(options.sample_rate);
        context.set_format(output_format);
        context.set_channels(options.audio_channels);
        context.set_channel_layout(channel_layout);
        context.set_time_base(options.audio_time_base);
        context.set_bit_rate(384_000);  // TODO make configurable

        ffmpeg_copy_codec_params(&mut stream, &context, &codec)?;

        stream.set_time_base(options.audio_time_base);

        let a_frame_size = ffmpeg_get_audio_context_frame_size(&context, 1024);

        let mut context_options = Dictionary::new();
        // Add some default options for certain codecs
        match codec.id() {
            codec::Id::AAC => {
                context_options.set("profile", "aac_low");
            },
            _ => ()
        };
        for (k, v) in options.audio_codec_params.iter() {
            context_options.set(k.as_str(), v.as_str());
        }

        let a_encoder = context.open_as_with(codec, context_options).vb_unwrap()?;
        let a_stream_idx = stream.index();

        ffmpeg_copy_context_params(&mut stream, a_encoder.as_ref())?;

        Ok((a_encoder, a_stream_idx, a_frame_size))
    }
}
