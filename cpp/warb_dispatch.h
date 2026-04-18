// Runtime loader for libBlackmagicRawAPI.so. The BMD Linux runtime exposes a
// single C entry point (`CreateBlackmagicRawFactoryInstance`, verified via
// `nm -D`). We dlopen it rather than link-time-depending on it so the crate
// builds on systems without the SDK installed.

#pragma once

#include "warb_abi.h"

#include <dlfcn.h>
#include <cstdlib>
#include <mutex>
#include <string>

namespace warb_abi {

inline std::string runtime_library_path() {
    if (const char* dir = std::getenv("BRAW_RUNTIME_DIR")) {
        std::string p(dir);
        if (!p.empty() && p.back() != '/') p += '/';
        p += "libBlackmagicRawAPI.so";
        return p;
    }
    return "libBlackmagicRawAPI.so";
}

// Single process-wide handle. The SDK spins up internal worker threads, so
// unloading is unsafe; we never dlclose.
struct LibraryState {
    void* handle = nullptr;
    CreateBlackmagicRawFactoryInstanceFn factory_fn = nullptr;
    std::string error;
};

inline const LibraryState& load_library() {
    static LibraryState state;
    static std::once_flag once;
    std::call_once(once, []{
        const std::string path = runtime_library_path();
        state.handle = dlopen(path.c_str(), RTLD_NOW | RTLD_LOCAL);
        if (!state.handle) {
            state.error = "dlopen(" + path + "): " + dlerror();
            return;
        }
        void* sym = dlsym(state.handle, "CreateBlackmagicRawFactoryInstance");
        if (!sym) {
            state.error = std::string("dlsym(CreateBlackmagicRawFactoryInstance): ") + dlerror();
            return;
        }
        state.factory_fn = reinterpret_cast<CreateBlackmagicRawFactoryInstanceFn>(sym);
    });
    return state;
}

} // namespace warb_abi
