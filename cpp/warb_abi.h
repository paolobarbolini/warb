// BMD Blackmagic RAW SDK ABI (Linux) — declarations derived from the public
// "Blackmagic RAW SDK — Developer Information" manual (August 2025). These
// are our own pure-virtual class declarations reproducing the documented
// vtable ABI; we do not ship any file from the SDK itself.
//
// Reference: BlackmagicRAW-SDK.pdf §"Basic Types" p.18–26 and §"Interface
// Reference" p.26ff. Method order in each class mirrors the manual's
// "Public Member Functions" tables, which the SDK lists in declaration
// order. Only the subset needed for RGB/YUV decode on Linux is declared.

#pragma once

#include <cstdint>
#include <utility>

namespace warb_abi {

// --- Primitive types (Linux, per manual p.18-21) ---
using HRESULT = int32_t;
using ULONG   = uint32_t;
using Boolean = bool;

struct GUID {
    uint32_t Data1;
    uint16_t Data2;
    uint16_t Data3;
    uint8_t  Data4[8];
};
using REFIID = const GUID&;

// Standard HRESULT codes (same universal Microsoft COM values on all platforms)
inline constexpr HRESULT S_OK           = 0x00000000;
inline constexpr HRESULT S_FALSE        = 0x00000001;
inline constexpr HRESULT E_NOTIMPL      = (HRESULT)0x80004001;
inline constexpr HRESULT E_NOINTERFACE  = (HRESULT)0x80004002;
inline constexpr HRESULT E_POINTER      = (HRESULT)0x80004003;
inline constexpr HRESULT E_FAIL         = (HRESULT)0x80004005;
inline constexpr HRESULT E_UNEXPECTED   = (HRESULT)0x8000FFFF;
inline constexpr HRESULT E_OUTOFMEMORY  = (HRESULT)0x8007000E;
inline constexpr HRESULT E_INVALIDARG   = (HRESULT)0x80070057;

inline constexpr bool SUCCEEDED(HRESULT hr) { return hr >= 0; }
inline constexpr bool FAILED(HRESULT hr)    { return hr <  0; }

// Universal IUnknown IID, fixed by the COM specification
inline constexpr GUID IID_IUnknown = {
    0x00000000, 0x0000, 0x0000,
    { 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46 }
};

// --- FourCC packing matching the SDK's multi-character-literal convention
// under GCC/Clang on Linux (MSB-first: 'abcd' == (a<<24)|(b<<16)|(c<<8)|d) ---
#define WARB_FOURCC(a,b,c,d) ( \
    (static_cast<uint32_t>(static_cast<uint8_t>(a)) << 24) | \
    (static_cast<uint32_t>(static_cast<uint8_t>(b)) << 16) | \
    (static_cast<uint32_t>(static_cast<uint8_t>(c)) <<  8) | \
     static_cast<uint32_t>(static_cast<uint8_t>(d)))

// --- Enums (manual p.21-26) ---
enum BlackmagicRawPipeline : uint32_t {
    blackmagicRawPipelineCPU    = WARB_FOURCC('c','p','u','b'),
    blackmagicRawPipelineCUDA   = WARB_FOURCC('c','u','d','a'),
    blackmagicRawPipelineMetal  = WARB_FOURCC('m','e','t','l'),
    blackmagicRawPipelineOpenCL = WARB_FOURCC('o','p','c','l'),
};

enum BlackmagicRawInterop : uint32_t {
    blackmagicRawInteropNone   = WARB_FOURCC('n','o','n','e'),
    blackmagicRawInteropOpenGL = WARB_FOURCC('o','p','g','l'),
};

enum BlackmagicRawResolutionScale : uint32_t {
    blackmagicRawResolutionScaleFull    = WARB_FOURCC('f','u','l','l'),
    blackmagicRawResolutionScaleHalf    = WARB_FOURCC('h','a','l','f'),
    blackmagicRawResolutionScaleQuarter = WARB_FOURCC('q','r','t','r'),
    blackmagicRawResolutionScaleEighth  = WARB_FOURCC('e','i','t','h'),
};

enum BlackmagicRawResourceType : uint32_t {
    blackmagicRawResourceTypeBufferCPU    = WARB_FOURCC('c','p','u','b'),
    blackmagicRawResourceTypeBufferMetal  = WARB_FOURCC('m','e','t','b'),
    blackmagicRawResourceTypeBufferCUDA   = WARB_FOURCC('c','u','d','b'),
    blackmagicRawResourceTypeBufferOpenCL = WARB_FOURCC('o','c','l','b'),
};

enum BlackmagicRawResourceFormat : uint32_t {
    blackmagicRawResourceFormatRGBAU8       = WARB_FOURCC('r','g','b','a'),
    blackmagicRawResourceFormatBGRAU8       = WARB_FOURCC('b','g','r','a'),
    blackmagicRawResourceFormatRGBU16       = WARB_FOURCC('1','6','i','l'),
    blackmagicRawResourceFormatRGBAU16      = WARB_FOURCC('1','6','a','l'),
    blackmagicRawResourceFormatBGRAU16      = WARB_FOURCC('1','6','l','a'),
    blackmagicRawResourceFormatRGBU16Planar = WARB_FOURCC('1','6','p','l'),
    blackmagicRawResourceFormatRGBF32       = WARB_FOURCC('f','3','2','s'),
    blackmagicRawResourceFormatRGBAF32      = WARB_FOURCC('f','3','2','l'),
    blackmagicRawResourceFormatBGRAF32      = WARB_FOURCC('f','3','2','a'),
    blackmagicRawResourceFormatRGBF32Planar = WARB_FOURCC('f','3','2','p'),
    blackmagicRawResourceFormatRGBF16       = WARB_FOURCC('f','1','6','s'),
    blackmagicRawResourceFormatRGBAF16      = WARB_FOURCC('f','1','6','l'),
    blackmagicRawResourceFormatBGRAF16      = WARB_FOURCC('f','1','6','a'),
    blackmagicRawResourceFormatRGBF16Planar = WARB_FOURCC('f','1','6','p'),
};

// --- Forward declarations for opaque interfaces we never call but which
// appear in method signatures we do declare. Only the type name needs to
// exist; we treat these as incomplete types passed by pointer. ---
class IBlackmagicRawClipProcessingAttributes;
class IBlackmagicRawFrameProcessingAttributes;
class IBlackmagicRawClipGeometry;
class IBlackmagicRawPipelineDevice;
class IBlackmagicRawPipelineIterator;
class IBlackmagicRawPipelineDeviceIterator;
class IBlackmagicRawMetadataIterator;

// --- Interface forward declarations (mutual recursion) ---
class IBlackmagicRaw;
class IBlackmagicRawClip;
class IBlackmagicRawFrame;
class IBlackmagicRawProcessedImage;
class IBlackmagicRawJob;
class IBlackmagicRawCallback;

// --- IUnknown (COM/BMD standard) ---
class IUnknown {
public:
    virtual HRESULT QueryInterface(REFIID iid, void** ppv) = 0;
    virtual ULONG   AddRef() = 0;
    virtual ULONG   Release() = 0;
};

// --- IBlackmagicRawFactory (manual p.29-30) ---
class IBlackmagicRawFactory : public IUnknown {
public:
    virtual HRESULT CreateCodec(IBlackmagicRaw** codec) = 0;
    virtual HRESULT CreatePipelineIterator(BlackmagicRawInterop interop,
                                           IBlackmagicRawPipelineIterator** pipelineIterator) = 0;
    virtual HRESULT CreatePipelineDeviceIterator(BlackmagicRawPipeline pipeline,
                                                 BlackmagicRawInterop interop,
                                                 IBlackmagicRawPipelineDeviceIterator** deviceIterator) = 0;
    virtual HRESULT CreateClipGeometry(IBlackmagicRawClipGeometry** geometry) = 0;
};

// --- IBlackmagicRaw (manual p.26-29) ---
class IBlackmagicRaw : public IUnknown {
public:
    virtual HRESULT OpenClip(const char* fileName, IBlackmagicRawClip** clip) = 0;
    virtual HRESULT OpenClipWithGeometry(const char* fileName,
                                         IBlackmagicRawClipGeometry* geometry,
                                         IBlackmagicRawClip** clip) = 0;
    virtual HRESULT SetCallback(IBlackmagicRawCallback* callback) = 0;
    virtual HRESULT PreparePipeline(BlackmagicRawPipeline pipeline,
                                    void* pipelineContext,
                                    void* pipelineCommandQueue,
                                    void* userData) = 0;
    virtual HRESULT PreparePipelineForDevice(IBlackmagicRawPipelineDevice* pipelineDevice,
                                             void* userData) = 0;
    virtual HRESULT FlushJobs() = 0;
};

// --- IBlackmagicRawClip (manual p.89-95). Method order from the Public
// Member Functions table on p.90. We declare all 18 methods in order so
// the vtable offset of CreateJobReadFrame is correct. Methods we don't
// use still consume a vtable slot — their parameter types can be sketchy
// since we never call them. ---
class IBlackmagicRawClip : public IUnknown {
public:
    virtual HRESULT GetWidth(uint32_t* width) = 0;
    virtual HRESULT GetHeight(uint32_t* height) = 0;
    virtual HRESULT GetFrameRate(float* frameRate) = 0;
    virtual HRESULT GetFrameCount(uint64_t* frameCount) = 0;
    virtual HRESULT GetTimecodeForFrame(uint64_t frameIndex, const char** timecode) = 0;
    virtual HRESULT GetMetadataIterator(IBlackmagicRawMetadataIterator** iterator) = 0;
    virtual HRESULT GetMetadata(const char* key, void* value /*Variant*/) = 0;
    virtual HRESULT SetMetadata(const char* key, void* value /*Variant*/) = 0;
    virtual HRESULT GetCameraType(const char** cameraType) = 0;
    virtual HRESULT CloneClipProcessingAttributes(IBlackmagicRawClipProcessingAttributes** clipProcessingAttributes) = 0;
    virtual HRESULT GetMulticardFileCount(uint32_t* multicardFileCount) = 0;
    virtual HRESULT IsMulticardFilePresent(uint32_t cardIndex, Boolean* present) = 0;
    virtual HRESULT GetSidecarFileAttached(Boolean* attached) = 0;
    virtual HRESULT SaveSidecarFile() = 0;
    virtual HRESULT ReloadSidecarFile() = 0;
    virtual HRESULT CreateJobReadFrame(uint64_t frameIndex, IBlackmagicRawJob** job) = 0;
    virtual HRESULT CreateJobTrim(const char* fileName,
                                  uint64_t frameIndex, uint64_t frameCount,
                                  IBlackmagicRawClipProcessingAttributes* clipProcessingAttributes,
                                  IBlackmagicRawFrameProcessingAttributes** frameProcessingAttributes,
                                  IBlackmagicRawJob** job) = 0;
    virtual HRESULT CloneWithGeometry(IBlackmagicRawClipGeometry* geometry, IBlackmagicRawClip** clip) = 0;
};

// --- IBlackmagicRawFrame (manual p.75-79). Method order from the tables
// spanning p.75-76. ---
class IBlackmagicRawFrame : public IUnknown {
public:
    virtual HRESULT GetFrameIndex(uint64_t* frameIndex) = 0;
    virtual HRESULT GetTimecode(const char** timecode) = 0;
    virtual HRESULT GetMetadataIterator(IBlackmagicRawMetadataIterator** iterator) = 0;
    virtual HRESULT GetMetadata(const char* key, void* value /*Variant*/) = 0;
    virtual HRESULT SetMetadata(const char* key, void* value /*Variant*/) = 0;
    virtual HRESULT CloneFrameProcessingAttributes(IBlackmagicRawFrameProcessingAttributes** frameProcessingAttributes) = 0;
    virtual HRESULT SetResolutionScale(BlackmagicRawResolutionScale resolutionScale) = 0;
    virtual HRESULT GetResolutionScale(BlackmagicRawResolutionScale* resolutionScale) = 0;
    virtual HRESULT SetResourceFormat(BlackmagicRawResourceFormat resourceFormat) = 0;
    virtual HRESULT GetResourceFormat(BlackmagicRawResourceFormat* resourceFormat) = 0;
    virtual HRESULT GetSensorRate(float* sensorRate) = 0;
    virtual HRESULT CreateJobDecodeAndProcessFrame(IBlackmagicRawClipProcessingAttributes* clipProcessingAttributes,
                                                   IBlackmagicRawFrameProcessingAttributes* frameProcessingAttributes,
                                                   IBlackmagicRawJob** job) = 0;
};

// --- IBlackmagicRawProcessedImage (manual p.59-61) ---
class IBlackmagicRawProcessedImage : public IUnknown {
public:
    virtual HRESULT GetWidth(uint32_t* width) = 0;
    virtual HRESULT GetHeight(uint32_t* height) = 0;
    virtual HRESULT GetResource(void** resource) = 0;
    virtual HRESULT GetResourceType(BlackmagicRawResourceType* type) = 0;
    virtual HRESULT GetResourceFormat(BlackmagicRawResourceFormat* format) = 0;
    virtual HRESULT GetResourceSizeBytes(uint32_t* sizeBytes) = 0;
    virtual HRESULT GetResourceContextAndCommandQueue(void** context, void** commandQueue) = 0;
};

// --- IBlackmagicRawJob (manual p.61-63) ---
class IBlackmagicRawJob : public IUnknown {
public:
    virtual HRESULT Submit() = 0;
    virtual HRESULT Abort() = 0;
    virtual HRESULT SetUserData(void* userData) = 0;
    virtual HRESULT GetUserData(void** userData) = 0;
};

// --- IBlackmagicRawCallback (manual p.64-67). Unlike other interfaces the
// callback methods return void. Method order per the Public Member
// Functions table on p.64. ---
class IBlackmagicRawCallback : public IUnknown {
public:
    virtual void ReadComplete(IBlackmagicRawJob* job, HRESULT result, IBlackmagicRawFrame* frame) = 0;
    virtual void DecodeComplete(IBlackmagicRawJob* job, HRESULT result) = 0;
    virtual void ProcessComplete(IBlackmagicRawJob* job, HRESULT result,
                                 IBlackmagicRawProcessedImage* processedImage) = 0;
    virtual void TrimProgress(IBlackmagicRawJob* job, float progress) = 0;
    virtual void TrimComplete(IBlackmagicRawJob* job, HRESULT result) = 0;
    virtual void SidecarMetadataParseWarning(IBlackmagicRawClip* clip, const char* fileName,
                                             uint32_t lineNumber, const char* info) = 0;
    virtual void SidecarMetadataParseError(IBlackmagicRawClip* clip, const char* fileName,
                                           uint32_t lineNumber, const char* info) = 0;
    virtual void PreparePipelineComplete(void* userData, HRESULT result) = 0;
};

// --- Dynamic-library entry point exported from libBlackmagicRawAPI.so ---
using CreateBlackmagicRawFactoryInstanceFn = IBlackmagicRawFactory* (*)();

// --- Smart pointer that releases a COM-style refcounted interface on
// destruction. Move-only. Takes ownership of the raw pointer at
// construction (refcount assumed to be 1 for a fresh out-parameter). ---
template <typename T>
class ComPtr {
public:
    constexpr ComPtr() noexcept = default;
    explicit ComPtr(T* raw) noexcept : p_(raw) {}
    ~ComPtr() { reset(); }

    ComPtr(const ComPtr&)            = delete;
    ComPtr& operator=(const ComPtr&) = delete;

    ComPtr(ComPtr&& other) noexcept : p_(std::exchange(other.p_, nullptr)) {}
    ComPtr& operator=(ComPtr&& other) noexcept {
        if (this != &other) {
            reset();
            p_ = std::exchange(other.p_, nullptr);
        }
        return *this;
    }

    T*   get()        const noexcept { return p_; }
    T*   operator->() const noexcept { return p_; }
    T&   operator*()  const noexcept { return *p_; }
    explicit operator bool() const noexcept { return p_ != nullptr; }

    T* release() noexcept { return std::exchange(p_, nullptr); }
    void reset(T* raw = nullptr) noexcept {
        T* old = std::exchange(p_, raw);
        if (old) old->Release();
    }

private:
    T* p_ = nullptr;
};

} // namespace warb_abi
