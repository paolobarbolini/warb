//! High-level synchronous decoder layered on top of the low-level handle
//! API. Owns a [`Codec`], an optional [`Clip`], and a channel wired into
//! an internal [`Callback`] that captures events.

use std::{path::Path, sync::mpsc};

use crate::{
    error::{Error, HResult},
    format::{Pipeline, ResolutionScale, ResourceFormat},
    handler::Callback,
    handles::{Clip, Codec, Frame, ProcessedImage},
};

enum DecoderEvent {
    Read {
        result: HResult,
        frame: Option<Frame>,
    },
    Process {
        result: HResult,
        image: Option<ProcessedImage>,
    },
}

struct DecoderHandler {
    tx: mpsc::Sender<DecoderEvent>,
}

impl Callback for DecoderHandler {
    fn on_read_complete(&self, _user_data: u64, result: HResult, frame: Option<Frame>) {
        let _ = self.tx.send(DecoderEvent::Read { result, frame });
    }

    fn on_process_complete(&self, _user_data: u64, result: HResult, image: Option<ProcessedImage>) {
        let _ = self.tx.send(DecoderEvent::Process { result, image });
    }
}

/// High-level synchronous decoder: one frame in, bytes out. Built on top
/// of the low-level primitives; see the crate-level docs for pipelining.
///
/// Constructed via [`Decoder::new`].
pub struct Decoder {
    codec: Codec,
    clip: Option<Clip>,
    events: mpsc::Receiver<DecoderEvent>,
    scale: ResolutionScale,
}

impl Decoder {
    /// Initialise the SDK runtime, prepare the CPU pipeline, and be ready
    /// to accept `open()` calls.
    pub fn new() -> Result<Self, Error> {
        let (tx, rx) = mpsc::channel();
        let mut codec = Codec::new(DecoderHandler { tx })?;
        codec.prepare_pipeline(Pipeline::Cpu)?;
        codec.flush_jobs();
        Ok(Self {
            codec,
            clip: None,
            events: rx,
            scale: ResolutionScale::Full,
        })
    }

    pub fn open(&mut self, path: &Path) -> Result<(), Error> {
        self.clip = Some(self.codec.open_clip(path)?);
        Ok(())
    }

    /// Output [`ResolutionScale`] applied to every subsequent
    /// [`decode_frame`](Self::decode_frame). Defaults to `Full`.
    pub fn set_scale(&mut self, scale: ResolutionScale) {
        self.scale = scale;
    }

    pub fn scale(&self) -> ResolutionScale {
        self.scale
    }

    pub fn width(&self) -> u32 {
        self.clip.as_ref().map_or(0, Clip::width)
    }

    pub fn height(&self) -> u32 {
        self.clip.as_ref().map_or(0, Clip::height)
    }

    pub fn frame_count(&self) -> u64 {
        self.clip.as_ref().map_or(0, Clip::frame_count)
    }

    pub fn frame_rate(&self) -> f32 {
        self.clip.as_ref().map_or(0.0, Clip::frame_rate)
    }

    /// Decode `frame_index` into `format`, blocking until complete. The
    /// returned [`ProcessedImage`] exposes the SDK's pixel buffer
    /// zero-copy; hold it for as long as you need the bytes.
    ///
    /// # Performance warning
    ///
    /// [`Decoder`] processes one frame at a time and keeps the SDK's
    /// worker pool mostly idle. It's convenient for pulling a handful
    /// of frames but far slower than the SDK is capable of. For real
    /// throughput (pipelined decode, many frames in flight), drop down
    /// to the low-level API: implement [`Callback`] and drive the
    /// submissions yourself. See `examples/to_rgba_pipelined.rs`.
    pub fn decode_frame(
        &mut self,
        frame_index: u64,
        format: ResourceFormat,
    ) -> Result<ProcessedImage, Error> {
        // Drain anything left over from a previous call.
        while self.events.try_recv().is_ok() {}

        let clip = self
            .clip
            .as_mut()
            .ok_or_else(|| Error::new("no clip is open"))?;
        clip.create_read_job(frame_index)?.submit()?;
        self.codec.flush_jobs();

        let mut frame = None;
        while let Ok(ev) = self.events.try_recv() {
            if let DecoderEvent::Read { result, frame: f } = ev {
                if result.is_err() {
                    return Err(Error::new(format!("read failed ({result})")));
                }
                frame = f;
            }
        }
        let mut frame = frame.ok_or_else(|| Error::new("on_read_complete delivered no frame"))?;

        frame.set_resource_format(format)?;
        frame.set_resolution_scale(self.scale)?;
        frame.create_decode_and_process_job()?.submit()?;
        self.codec.flush_jobs();

        let mut image = None;
        while let Ok(ev) = self.events.try_recv() {
            if let DecoderEvent::Process { result, image: i } = ev {
                if result.is_err() {
                    return Err(Error::new(format!("process failed ({result})")));
                }
                image = i;
            }
        }
        image.ok_or_else(|| Error::new("on_process_complete delivered no image"))
    }
}
