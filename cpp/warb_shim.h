// Thin RAII wrappers around the BMD COM interfaces, exposed to Rust via
// cxx. The higher-level `Decoder` lives entirely in Rust on top of these.

#pragma once

#include "warb_abi.h"
#include "rust/cxx.h"

#include <memory>

// Forward-declared Rust type: the cxx-generated header gives us the
// extern "Rust" dispatch functions (on_read_complete etc.) that take
// `CallbackBridge&`.
struct CallbackBridge;

class BrawCodec;
class BrawClip;
class BrawFrame;
class BrawProcessedImage;
class BrawJob;

// Each wrapper owns one reference to its underlying COM interface via
// `warb_abi::ComPtr`, which releases on destruction. Move-only.

class BrawJob final {
public:
    explicit BrawJob(warb_abi::IBlackmagicRawJob* raw) noexcept : raw_(raw) {}

    void set_user_data(uint64_t user_data);
    void abort();

    friend void submit_job(std::unique_ptr<BrawJob>);

private:
    warb_abi::ComPtr<warb_abi::IBlackmagicRawJob> raw_;
};

class BrawFrame final {
public:
    explicit BrawFrame(warb_abi::IBlackmagicRawFrame* raw) noexcept : raw_(raw) {}

    void set_resource_format(uint32_t format);
    void set_resolution_scale(uint32_t scale);
    std::unique_ptr<BrawJob> create_job_decode_and_process();

private:
    warb_abi::ComPtr<warb_abi::IBlackmagicRawFrame> raw_;
};

class BrawProcessedImage final {
public:
    explicit BrawProcessedImage(warb_abi::IBlackmagicRawProcessedImage* raw) noexcept
        : raw_(raw) {}

    uint32_t width()      const;
    uint32_t height()     const;
    uint32_t format()     const;
    uint32_t size_bytes() const;
    // Borrowed view into the SDK's decoded pixel buffer. The buffer
    // stays live as long as this BrawProcessedImage does; cxx ties the
    // returned slice's lifetime to &self.
    rust::Slice<const uint8_t> data() const;

private:
    warb_abi::ComPtr<warb_abi::IBlackmagicRawProcessedImage> raw_;
};

class BrawClip final {
public:
    explicit BrawClip(warb_abi::IBlackmagicRawClip* raw) noexcept : raw_(raw) {}

    uint32_t width()       const;
    uint32_t height()      const;
    uint64_t frame_count() const;
    float    frame_rate()  const;
    std::unique_ptr<BrawJob> create_job_read_frame(uint64_t frame_index);

private:
    warb_abi::ComPtr<warb_abi::IBlackmagicRawClip> raw_;
};

// Implements IBlackmagicRawCallback, serialises access to the Rust-owned
// CallbackBridge box across SDK worker threads, and forwards events via cxx's
// generated `extern "Rust"` trampolines.
class CallbackDispatcher;

class BrawCodec final {
public:
    BrawCodec(warb_abi::IBlackmagicRawFactory* factory,
              warb_abi::IBlackmagicRaw* codec,
              std::unique_ptr<CallbackDispatcher> dispatcher);
    ~BrawCodec();

    BrawCodec(const BrawCodec&)            = delete;
    BrawCodec& operator=(const BrawCodec&) = delete;

    void prepare_pipeline(uint32_t pipeline);
    std::unique_ptr<BrawClip> open_clip(rust::Slice<const uint8_t> path);
    void flush_jobs();

private:
    // Declared so that implicit teardown (post-body of ~BrawCodec)
    // releases `codec_` first, then `factory_`, and destroys
    // `dispatcher_` (which owns the Rust CallbackBridge box) last.
    std::unique_ptr<CallbackDispatcher>               dispatcher_;
    warb_abi::ComPtr<warb_abi::IBlackmagicRawFactory> factory_;
    warb_abi::ComPtr<warb_abi::IBlackmagicRaw>        codec_;
};

std::unique_ptr<BrawCodec> new_codec(rust::Box<CallbackBridge> handler);
void submit_job(std::unique_ptr<BrawJob> job);
