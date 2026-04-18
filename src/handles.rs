//! RAII wrappers around the SDK's COM-like interfaces. Each type owns one
//! reference to the underlying object and releases it on drop.

use std::{mem, ops::Deref, os::unix::ffi::OsStrExt, path::Path, pin::Pin};

use cxx::UniquePtr;

use crate::{
    error::BrawError,
    ffi,
    format::{Pipeline, ResolutionScale, ResourceFormat},
    handler::{Callback, CallbackBridge},
};

/// The decoder codec, owning the SDK factory, codec, and callback
/// dispatcher. Drop order handles SDK teardown safely.
pub struct Codec(UniquePtr<ffi::BrawCodec>);

impl Codec {
    /// Initialise the SDK runtime, create a codec, and register
    /// `callback` for asynchronous events.
    pub fn new<C: Callback>(callback: C) -> Result<Self, BrawError> {
        let c = Box::new(CallbackBridge::new(callback));
        Ok(Self(ffi::new_codec(c)?))
    }

    /// Activate a decode pipeline. Fires `on_prepare_pipeline_complete`
    /// asynchronously; call [`flush_jobs`](Self::flush_jobs) to wait. The
    /// SDK refuses to dispatch decode jobs until a pipeline is prepared.
    pub fn prepare_pipeline(&mut self, pipeline: Pipeline) -> Result<(), BrawError> {
        self.pin_mut().prepare_pipeline(pipeline as u32)?;
        Ok(())
    }

    /// Open a `.braw` clip. The path is passed to the SDK as raw bytes —
    /// it doesn't have to be valid UTF-8 (Linux paths seldom are). An
    /// embedded NUL byte is the only disallowed content.
    pub fn open_clip(&mut self, path: impl AsRef<Path>) -> Result<Clip, BrawError> {
        self.do_open_clip(path.as_ref())
    }

    fn do_open_clip(&mut self, path: &Path) -> Result<Clip, BrawError> {
        let bytes = path.as_os_str().as_bytes();
        if bytes.contains(&0) {
            return Err(BrawError::new("path contains an interior NUL byte"));
        }
        Ok(Clip(self.pin_mut().open_clip(bytes)?))
    }

    /// Block until every in-flight job has finished and its callback has
    /// returned.
    pub fn flush_jobs(&mut self) {
        self.pin_mut().flush_jobs();
    }

    fn pin_mut(&mut self) -> Pin<&mut ffi::BrawCodec> {
        self.0.pin_mut()
    }
}

/// An opened `.braw` clip. Metadata getters are cheap; frame reads are
/// asynchronous via [`create_read_job`](Self::create_read_job).
pub struct Clip(UniquePtr<ffi::BrawClip>);

impl Clip {
    pub fn width(&self) -> u32 {
        self.0.width()
    }

    pub fn height(&self) -> u32 {
        self.0.height()
    }

    pub fn frame_count(&self) -> u64 {
        self.0.frame_count()
    }

    pub fn frame_rate(&self) -> f32 {
        self.0.frame_rate()
    }

    /// Build a job that will read the raw bitstream for `frame_index`.
    /// Pair with [`Job::set_user_data`] to route the subsequent
    /// `on_read_complete` callback to the right slot.
    pub fn create_read_job(&mut self, frame_index: u64) -> Result<Job, BrawError> {
        Ok(Job(self.0.pin_mut().create_job_read_frame(frame_index)?))
    }
}

/// A frame as received in `on_read_complete` — bitstream loaded, not yet
/// decoded. Configure its output format, then create a decode+process job.
pub struct Frame(UniquePtr<ffi::BrawFrame>);

impl Frame {
    pub(crate) fn from_cxx(inner: UniquePtr<ffi::BrawFrame>) -> Self {
        Self(inner)
    }

    pub fn set_resource_format(&mut self, format: ResourceFormat) -> Result<(), BrawError> {
        self.0.pin_mut().set_resource_format(format as u32)?;
        Ok(())
    }

    /// Set the output [`ResolutionScale`] for the subsequent decode. Must
    /// be called before [`create_decode_and_process_job`](Self::create_decode_and_process_job).
    pub fn set_resolution_scale(&mut self, scale: ResolutionScale) -> Result<(), BrawError> {
        self.0.pin_mut().set_resolution_scale(scale as u32)?;
        Ok(())
    }

    /// Build a job that will decode and process this frame into its
    /// currently-configured [`ResourceFormat`]. The resulting image
    /// arrives via `on_process_complete`.
    pub fn create_decode_and_process_job(&mut self) -> Result<Job, BrawError> {
        Ok(Job(self.0.pin_mut().create_job_decode_and_process()?))
    }
}

/// A fully-decoded image received via `on_process_complete`. Metadata
/// (width, height, format, size) is queried from the SDK once at
/// construction and cached; subsequent calls are free.
pub struct ProcessedImage {
    inner: UniquePtr<ffi::BrawProcessedImage>,
    width: u32,
    height: u32,
    format: ResourceFormat,
    size_bytes: u32,
    // Exactly width * height * format.bytes_per_pixel().
    pixel_len: usize,
}

impl ProcessedImage {
    pub(crate) fn from_cxx(inner: UniquePtr<ffi::BrawProcessedImage>) -> Self {
        let width = inner.image_width();
        let height = inner.image_height();
        let raw_format = inner.image_format();
        // The SDK returns whatever format the caller requested via
        // `Frame::set_resource_format`; seeing one we don't know about
        // means the runtime is ahead of our `ResourceFormat` enum and
        // the library needs updating. Fail loudly rather than silently
        // misinterpret the buffer.
        let format = ResourceFormat::from_raw(raw_format).unwrap_or_else(|| {
            panic!("warb: SDK returned unknown ResourceFormat 0x{raw_format:08x}")
        });
        let size_bytes = inner.image_size_bytes();
        let pixel_len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(format.bytes_per_pixel() as usize);
        Self {
            inner,
            width,
            height,
            format,
            size_bytes,
            pixel_len,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn format(&self) -> ResourceFormat {
        self.format
    }

    /// Total size of the SDK's buffer, including any tail padding (the
    /// SDK rounds allocations up to a 2048-byte boundary).
    pub fn size_bytes(&self) -> u32 {
        self.size_bytes
    }

    /// Borrowed, zero-copy view of the pixel data truncated to exactly
    /// `width * height * format.bytes_per_pixel()` bytes.
    pub fn data(&self) -> &[u8] {
        let full = self.inner.image_data();
        &full[..self.pixel_len.min(full.len())]
    }

    /// Borrowed view of the SDK's entire allocation, including any tail
    /// padding. Use this when you specifically need the raw buffer.
    pub fn full_data(&self) -> &[u8] {
        self.inner.image_data()
    }
}

impl Deref for ProcessedImage {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.data()
    }
}

impl AsRef<[u8]> for ProcessedImage {
    fn as_ref(&self) -> &[u8] {
        self.data()
    }
}

/// A queued-but-not-yet-submitted job. Calling [`submit`](Self::submit)
/// consumes the handle (the SDK takes ownership internally).
pub struct Job(UniquePtr<ffi::BrawJob>);

impl Job {
    /// Attach a 64-bit tag retrievable inside the completion callback.
    pub fn set_user_data(&mut self, user_data: u64) -> Result<(), BrawError> {
        self.0.pin_mut().set_user_data(user_data)?;
        Ok(())
    }

    /// Attempt to cancel the job. Per the SDK manual (p.63) this can
    /// fail if the job has already been started by the internal decoder.
    /// The Rust handle may be dropped normally afterwards.
    pub fn abort(&mut self) -> Result<(), BrawError> {
        self.0.pin_mut().abort_job()?;
        Ok(())
    }

    /// Submit the job to the decoder. Consumes the handle: the SDK takes
    /// ownership inside `Submit()`, so holding our Rust-side reference
    /// past this call deadlocks `flush_jobs`.
    pub fn submit(mut self) -> Result<(), BrawError> {
        let inner = mem::replace(&mut self.0, UniquePtr::null());
        ffi::submit_job(inner)?;
        Ok(())
    }
}

// SAFETY: BMD's COM handles are internally refcounted and designed to be
// moved between threads; the callback delivery pattern in the SDK assumes
// frames and images can cross thread boundaries. We don't implement Sync
// because concurrent access from two threads would race on method calls.
unsafe impl Send for Codec {}
unsafe impl Send for Clip {}
unsafe impl Send for Frame {}
unsafe impl Send for ProcessedImage {}
unsafe impl Send for Job {}
