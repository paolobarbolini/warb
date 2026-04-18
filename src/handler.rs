//! The async-callback interface. SDK worker threads call into
//! `CallbackBridge`'s bridge methods (via cxx-generated trampolines)
//! concurrently; we take `&self` everywhere so the compiler enforces
//! that callbacks don't need mutual exclusion on the Rust side. Users
//! who need mutable state pick their own interior mutability (channel,
//! atomic, mutex).

use cxx::UniquePtr;

use crate::{
    error::HResult,
    ffi,
    handles::{Frame, ProcessedImage},
};

/// Async-callback receiver. Implementations typically forward events to
/// a channel or atomic slot. Methods take `&self`; use interior
/// mutability (`mpsc::Sender`, `AtomicU64`, `Mutex`, …) if you need to
/// mutate.
///
/// The trait requires `Send + Sync + 'static` as supertraits: the SDK
/// hands callbacks to worker threads and the codec may outlive the
/// stack frame that created it, so any impl must satisfy all three.
pub trait Callback: Send + Sync + 'static {
    fn on_read_complete(&self, user_data: u64, result: HResult, frame: Option<Frame>);
    fn on_process_complete(&self, user_data: u64, result: HResult, image: Option<ProcessedImage>);
    fn on_decode_complete(&self, _user_data: u64, _result: HResult) {}
    fn on_prepare_pipeline_complete(&self, _user_data: u64, _result: HResult) {}
}

// Rust-owned wrapper the cxx bridge passes to C++ as
// `rust::Box<CallbackBridge>`. Not user-visible: has no public
// constructor and no public methods; it's only here because cxx needs a
// named type to attach trampolines to.
pub(crate) struct CallbackBridge {
    // `dyn Callback` alone is not Send/Sync on the trait-object level
    // (auto traits aren't propagated from supertraits onto `dyn T`), so
    // we spell them explicitly here.
    inner: Box<dyn Callback + Send + Sync>,
}

impl CallbackBridge {
    pub(crate) fn new<C: Callback>(callback: C) -> Self {
        Self {
            inner: Box::new(callback),
        }
    }

    // --- cxx bridge targets. `pub(crate)` so the cxx-generated trampolines
    // in `mod ffi` can reach them; not meant to be called from user code.

    pub(crate) fn on_read_complete(
        &self,
        user_data: u64,
        result: i32,
        frame: UniquePtr<ffi::BrawFrame>,
    ) {
        let f = (!frame.is_null()).then(|| Frame::from_cxx(frame));
        self.inner.on_read_complete(user_data, HResult(result), f);
    }

    pub(crate) fn on_process_complete(
        &self,
        user_data: u64,
        result: i32,
        image: UniquePtr<ffi::BrawProcessedImage>,
    ) {
        let i = (!image.is_null()).then(|| ProcessedImage::from_cxx(image));
        self.inner
            .on_process_complete(user_data, HResult(result), i);
    }

    pub(crate) fn on_decode_complete(&self, user_data: u64, result: i32) {
        self.inner.on_decode_complete(user_data, HResult(result));
    }

    pub(crate) fn on_prepare_pipeline_complete(&self, user_data: u64, result: i32) {
        self.inner
            .on_prepare_pipeline_complete(user_data, HResult(result));
    }
}
