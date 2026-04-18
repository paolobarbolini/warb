#include "warb_shim.h"
#include "warb_dispatch.h"
#include "warb/src/lib.rs.h"

#include <cstdio>
#include <stdexcept>
#include <string>
#include <utility>

// Enable with `cargo build --features trace`. The no-op template keeps
// every arg used so -Wunused-parameter stays quiet when disabled.
#ifdef WARB_TRACE
#define WARB_DBG(fmt, ...) \
    do { std::fprintf(stderr, "[warb] " fmt "\n", ##__VA_ARGS__); std::fflush(stderr); } while (0)
#else
namespace warb_trace { template <class... Ts> inline void noop(Ts&&...) noexcept {} }
#define WARB_DBG(fmt, ...) warb_trace::noop(fmt, ##__VA_ARGS__)
#endif

namespace {

[[noreturn]] void throw_hr(const char* what, warb_abi::HRESULT hr) {
    char buf[64];
    std::snprintf(buf, sizeof(buf), " (HRESULT=0x%08x)", static_cast<uint32_t>(hr));
    throw std::runtime_error(std::string(what) + buf);
}

uint64_t job_user_data(warb_abi::IBlackmagicRawJob* job) {
    if (!job) return 0;
    void* p = nullptr;
    (void)job->GetUserData(&p);
    return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(p));
}

} // namespace

// --- CallbackDispatcher ---

class CallbackDispatcher final : public warb_abi::IBlackmagicRawCallback {
public:
    explicit CallbackDispatcher(rust::Box<CallbackBridge> handler) noexcept
        : handler_(std::move(handler)) {}

    // IUnknown — we own ourselves via std::unique_ptr; ignore refcount.
    warb_abi::HRESULT QueryInterface(warb_abi::REFIID, void** ppv) override {
        if (!ppv) return warb_abi::E_POINTER;
        *ppv = this;
        return warb_abi::S_OK;
    }
    warb_abi::ULONG AddRef()  override { return 1; }
    warb_abi::ULONG Release() override { return 1; }

    void ReadComplete(warb_abi::IBlackmagicRawJob* job,
                      warb_abi::HRESULT result,
                      warb_abi::IBlackmagicRawFrame* frame) override {
        WARB_DBG("ReadComplete job=%p result=0x%x frame=%p",
                 (void*)job, (unsigned)result, (void*)frame);
        std::unique_ptr<BrawFrame> owned;
        if (frame) {
            frame->AddRef();
            owned = std::make_unique<BrawFrame>(frame);
        }
        handler_->on_read_complete(job_user_data(job),
                                   static_cast<int32_t>(result), std::move(owned));
    }

    void DecodeComplete(warb_abi::IBlackmagicRawJob* job,
                        warb_abi::HRESULT result) override {
        WARB_DBG("DecodeComplete job=%p result=0x%x", (void*)job, (unsigned)result);
        handler_->on_decode_complete(job_user_data(job), static_cast<int32_t>(result));
    }

    void ProcessComplete(warb_abi::IBlackmagicRawJob* job,
                         warb_abi::HRESULT result,
                         warb_abi::IBlackmagicRawProcessedImage* image) override {
        WARB_DBG("ProcessComplete job=%p result=0x%x image=%p",
                 (void*)job, (unsigned)result, (void*)image);
        std::unique_ptr<BrawProcessedImage> owned;
        if (image) {
            image->AddRef();
            owned = std::make_unique<BrawProcessedImage>(image);
        }
        handler_->on_process_complete(job_user_data(job),
                                      static_cast<int32_t>(result), std::move(owned));
    }

    void PreparePipelineComplete(void* userData, warb_abi::HRESULT result) override {
        WARB_DBG("PreparePipelineComplete ud=%p result=0x%x", userData, (unsigned)result);
        handler_->on_prepare_pipeline_complete(
            static_cast<uint64_t>(reinterpret_cast<uintptr_t>(userData)),
            static_cast<int32_t>(result));
    }

    // Unused for the current Rust-level API — no dispatch.
    void TrimProgress(warb_abi::IBlackmagicRawJob*, float) override {}
    void TrimComplete(warb_abi::IBlackmagicRawJob*, warb_abi::HRESULT) override {}
    void SidecarMetadataParseWarning(warb_abi::IBlackmagicRawClip*, const char*, uint32_t, const char*) override {}
    void SidecarMetadataParseError(warb_abi::IBlackmagicRawClip*, const char*, uint32_t, const char*) override {}

private:
    // The Rust-side `CallbackBridge` methods take `&self`, so the SDK
    // can dispatch callbacks from multiple worker threads concurrently
    // without any exclusion here. Whatever mutation a user handler
    // needs it provides via its own interior mutability.
    rust::Box<CallbackBridge> handler_;
};

// --- BrawJob ---

void BrawJob::set_user_data(uint64_t user_data) {
    void* p = reinterpret_cast<void*>(static_cast<uintptr_t>(user_data));
    auto hr = raw_->SetUserData(p);
    if (!warb_abi::SUCCEEDED(hr)) throw_hr("IBlackmagicRawJob::SetUserData failed", hr);
}

void BrawJob::abort() {
    auto hr = raw_->Abort();
    if (!warb_abi::SUCCEEDED(hr)) throw_hr("IBlackmagicRawJob::Abort failed", hr);
}

// --- BrawFrame ---

void BrawFrame::set_resource_format(uint32_t format) {
    auto hr = raw_->SetResourceFormat(
        static_cast<warb_abi::BlackmagicRawResourceFormat>(format));
    if (!warb_abi::SUCCEEDED(hr)) throw_hr("IBlackmagicRawFrame::SetResourceFormat failed", hr);
}

void BrawFrame::set_resolution_scale(uint32_t scale) {
    auto hr = raw_->SetResolutionScale(
        static_cast<warb_abi::BlackmagicRawResolutionScale>(scale));
    if (!warb_abi::SUCCEEDED(hr)) throw_hr("IBlackmagicRawFrame::SetResolutionScale failed", hr);
}

std::unique_ptr<BrawJob> BrawFrame::create_job_decode_and_process() {
    warb_abi::IBlackmagicRawJob* job = nullptr;
    auto hr = raw_->CreateJobDecodeAndProcessFrame(nullptr, nullptr, &job);
    if (!warb_abi::SUCCEEDED(hr) || !job) throw_hr("CreateJobDecodeAndProcessFrame failed", hr);
    return std::make_unique<BrawJob>(job);
}

// --- BrawProcessedImage ---

uint32_t BrawProcessedImage::width() const {
    uint32_t v = 0; (void)raw_->GetWidth(&v); return v;
}
uint32_t BrawProcessedImage::height() const {
    uint32_t v = 0; (void)raw_->GetHeight(&v); return v;
}
uint32_t BrawProcessedImage::format() const {
    warb_abi::BlackmagicRawResourceFormat f{};
    (void)raw_->GetResourceFormat(&f);
    return static_cast<uint32_t>(f);
}
uint32_t BrawProcessedImage::size_bytes() const {
    uint32_t v = 0; (void)raw_->GetResourceSizeBytes(&v); return v;
}
rust::Slice<const uint8_t> BrawProcessedImage::data() const {
    uint32_t size = 0;
    void*    resource = nullptr;
    (void)raw_->GetResourceSizeBytes(&size);
    auto hr = raw_->GetResource(&resource);
    if (!warb_abi::SUCCEEDED(hr) || !resource) {
        // Empty slice signals failure; callers can cross-check size_bytes().
        return rust::Slice<const uint8_t>();
    }
    return rust::Slice<const uint8_t>(static_cast<const uint8_t*>(resource), size);
}

// --- BrawClip ---

uint32_t BrawClip::width() const {
    uint32_t v = 0; (void)raw_->GetWidth(&v); return v;
}
uint32_t BrawClip::height() const {
    uint32_t v = 0; (void)raw_->GetHeight(&v); return v;
}
uint64_t BrawClip::frame_count() const {
    uint64_t v = 0; (void)raw_->GetFrameCount(&v); return v;
}
float BrawClip::frame_rate() const {
    float v = 0.0f; (void)raw_->GetFrameRate(&v); return v;
}
std::unique_ptr<BrawJob> BrawClip::create_job_read_frame(uint64_t frame_index) {
    warb_abi::IBlackmagicRawJob* job = nullptr;
    auto hr = raw_->CreateJobReadFrame(frame_index, &job);
    if (!warb_abi::SUCCEEDED(hr) || !job) throw_hr("CreateJobReadFrame failed", hr);
    return std::make_unique<BrawJob>(job);
}

// --- BrawCodec ---

BrawCodec::BrawCodec(warb_abi::IBlackmagicRawFactory* factory,
                     warb_abi::IBlackmagicRaw* codec,
                     std::unique_ptr<CallbackDispatcher> dispatcher)
    : dispatcher_(std::move(dispatcher)),
      factory_(factory),
      codec_(codec) {}

BrawCodec::~BrawCodec() {
    if (codec_) {
        // Drain any in-flight work so no SDK thread calls our dispatcher
        // after we return, then detach so newly-submitted jobs won't.
        codec_->FlushJobs();
        codec_->SetCallback(nullptr);
    }
    // `codec_`, `factory_`, and `dispatcher_` tear down in reverse
    // declaration order: codec → factory → dispatcher (CallbackBridge box).
}

void BrawCodec::prepare_pipeline(uint32_t pipeline) {
    auto hr = codec_->PreparePipeline(
        static_cast<warb_abi::BlackmagicRawPipeline>(pipeline),
        nullptr, nullptr, nullptr);
    if (!warb_abi::SUCCEEDED(hr)) throw_hr("IBlackmagicRaw::PreparePipeline failed", hr);
}

std::unique_ptr<BrawClip> BrawCodec::open_clip(rust::Slice<const uint8_t> path) {
    std::string path_z(reinterpret_cast<const char*>(path.data()), path.size());
    warb_abi::IBlackmagicRawClip* clip = nullptr;
    auto hr = codec_->OpenClip(path_z.c_str(), &clip);
    if (!warb_abi::SUCCEEDED(hr) || !clip) throw_hr("IBlackmagicRaw::OpenClip failed", hr);
    return std::make_unique<BrawClip>(clip);
}

void BrawCodec::flush_jobs() {
    codec_->FlushJobs();
}

// --- Factory + free functions ---

std::unique_ptr<BrawCodec> new_codec(rust::Box<CallbackBridge> handler) {
    const auto& lib = warb_abi::load_library();
    if (!lib.factory_fn) {
        throw std::runtime_error("Blackmagic RAW runtime not loaded: " + lib.error);
    }
    // Wrap each raw COM pointer in a ComPtr immediately so any subsequent
    // throw releases it.
    warb_abi::ComPtr<warb_abi::IBlackmagicRawFactory> factory(lib.factory_fn());
    if (!factory) throw std::runtime_error("CreateBlackmagicRawFactoryInstance() returned null");

    warb_abi::IBlackmagicRaw* codec_raw = nullptr;
    auto hr = factory->CreateCodec(&codec_raw);
    if (!warb_abi::SUCCEEDED(hr) || !codec_raw) {
        throw_hr("IBlackmagicRawFactory::CreateCodec failed", hr);
    }
    warb_abi::ComPtr<warb_abi::IBlackmagicRaw> codec(codec_raw);

    auto dispatcher = std::make_unique<CallbackDispatcher>(std::move(handler));
    hr = codec->SetCallback(dispatcher.get());
    if (!warb_abi::SUCCEEDED(hr)) throw_hr("IBlackmagicRaw::SetCallback failed", hr);

    return std::make_unique<BrawCodec>(
        factory.release(), codec.release(), std::move(dispatcher));
}

void submit_job(std::unique_ptr<BrawJob> job) {
    // Submit transfers ownership to the SDK internally (see project memory
    // on SDK runtime quirks); the caller's ref is released as the
    // unique_ptr and its ComPtr member unwind at function exit.
    if (!job || !job->raw_) throw std::runtime_error("submit_job called with null job");
    auto hr = job->raw_->Submit();
    if (!warb_abi::SUCCEEDED(hr)) throw_hr("IBlackmagicRawJob::Submit failed", hr);
}
