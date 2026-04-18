//! Unofficial Rust bindings for the Blackmagic RAW SDK on Linux. Not
//! affiliated with or endorsed by Blackmagic Design.
//!
//! The crate exposes a low-level, handle-based mirror of the SDK's C++
//! interfaces (`Codec`, `Clip`, `Frame`, `ProcessedImage`, `Job`) plus a
//! [`Callback`] trait that receives the SDK's asynchronous callbacks.
//! A small high-level [`Decoder`] is layered on top for single-frame
//! synchronous decode; anything more sophisticated (pipelining, batching,
//! GPU backends) is the caller's to build.
//!
//! At runtime `libBlackmagicRawAPI.so` is loaded via `dlopen`; set
//! `BRAW_RUNTIME_DIR` or place it on the default loader search path.

// Needed at this scope so `#[cxx::bridge]` can name it in the extern
// "Rust" block. Not part of the public API.
pub(crate) use handler::CallbackBridge;

#[cxx::bridge]
pub(crate) mod ffi {
    extern "Rust" {
        type CallbackBridge;
        fn on_read_complete(
            self: &CallbackBridge,
            user_data: u64,
            result: i32,
            frame: UniquePtr<BrawFrame>,
        );
        fn on_process_complete(
            self: &CallbackBridge,
            user_data: u64,
            result: i32,
            image: UniquePtr<BrawProcessedImage>,
        );
        fn on_decode_complete(self: &CallbackBridge, user_data: u64, result: i32);
        fn on_prepare_pipeline_complete(self: &CallbackBridge, user_data: u64, result: i32);
    }

    unsafe extern "C++" {
        include!("warb_shim.h");

        type BrawCodec;
        type BrawClip;
        type BrawFrame;
        type BrawProcessedImage;
        type BrawJob;

        fn new_codec(callback: Box<CallbackBridge>) -> Result<UniquePtr<BrawCodec>>;
        fn prepare_pipeline(self: Pin<&mut BrawCodec>, pipeline: u32) -> Result<()>;
        fn open_clip(self: Pin<&mut BrawCodec>, path: &[u8]) -> Result<UniquePtr<BrawClip>>;
        fn flush_jobs(self: Pin<&mut BrawCodec>);

        fn width(self: &BrawClip) -> u32;
        fn height(self: &BrawClip) -> u32;
        fn frame_count(self: &BrawClip) -> u64;
        fn frame_rate(self: &BrawClip) -> f32;
        fn create_job_read_frame(
            self: Pin<&mut BrawClip>,
            frame_index: u64,
        ) -> Result<UniquePtr<BrawJob>>;

        fn set_resource_format(self: Pin<&mut BrawFrame>, format: u32) -> Result<()>;
        fn set_resolution_scale(self: Pin<&mut BrawFrame>, scale: u32) -> Result<()>;
        fn create_job_decode_and_process(self: Pin<&mut BrawFrame>) -> Result<UniquePtr<BrawJob>>;

        #[cxx_name = "width"]
        fn image_width(self: &BrawProcessedImage) -> u32;
        #[cxx_name = "height"]
        fn image_height(self: &BrawProcessedImage) -> u32;
        #[cxx_name = "format"]
        fn image_format(self: &BrawProcessedImage) -> u32;
        #[cxx_name = "size_bytes"]
        fn image_size_bytes(self: &BrawProcessedImage) -> u32;
        #[cxx_name = "data"]
        fn image_data<'a>(self: &'a BrawProcessedImage) -> &'a [u8];

        fn set_user_data(self: Pin<&mut BrawJob>, user_data: u64) -> Result<()>;
        #[cxx_name = "abort"]
        fn abort_job(self: Pin<&mut BrawJob>) -> Result<()>;
        fn submit_job(job: UniquePtr<BrawJob>) -> Result<()>;
    }
}

mod decoder;
mod error;
mod format;
mod handler;
mod handles;

pub use decoder::Decoder;
pub use error::{BrawError, HResult};
pub use format::{Pipeline, ResolutionScale, ResourceFormat};
pub use handler::Callback;
pub use handles::{Clip, Codec, Frame, Job, ProcessedImage};
