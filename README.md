# warb

Rust bindings for Blackmagic RAW (BRAW) decoding on Linux, built on top of Blackmagic Design's Blackmagic RAW SDK.

**Unofficial.** Not affiliated with, endorsed by, or supported by Blackmagic Design. Linux x86\_64 only.

## Who this is for

You shoot on Blackmagic cameras, you have `.braw` files on Linux, and you want to decode them from Rust, typically to generate proxies, thumbnails, or low-res previews for a web UI. That's the case the author uses it for.

**`warb` has not been tested for image-fidelity use cases**, and the decoded output may not match what you'd get from the official SDK tools (DaVinci Resolve, Blackmagic RAW Player) across clips, formats, or scales. Clip processing attributes, sidecar metadata, custom gamma, tone curves, and 3D LUTs are not currently applied; the SDK runs with its defaults. If pixel-accurate correctness matters to your workflow, verify against the reference tools yourself before relying on this crate.

## Reporting issues

Please file any bugs, unexpected SDK behaviour, or feature requests on this repository's GitHub Issues. **Do not report `warb` issues to Blackmagic Design's support channels.** This crate is not a Blackmagic product, and surfacing our bugs to their support team wastes their time and misrepresents the source of the defect. If you've narrowed something down to the SDK runtime itself (reproducible with Blackmagic RAW Player or DaVinci Resolve), that's when to talk to Blackmagic.

## Quick start

```rust
use std::path::Path;
use warb::{Decoder, ResolutionScale, ResourceFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = Decoder::new()?;
    decoder.set_scale(ResolutionScale::Quarter);
    decoder.open(Path::new("clip.braw"))?;

    let image = decoder.decode_frame(0, ResourceFormat::RgbaU8)?;
    println!("{}×{}, {} bytes", image.width(), image.height(), image.data().len());
    // `image` derefs to &[u8]; write straight to a file, socket, encoder, etc.
    Ok(())
}
```

## Prerequisites

`warb` **does not link against `libBlackmagicRawAPI.so` at build time.** There are no SDK headers, import libraries, or `build.rs` dependencies on BMD files at all. The runtime is loaded dynamically via `dlopen` the first time a `Codec` is created, which means:

- The crate compiles on machines that have never seen the SDK.
- Binaries can be built once and shipped anywhere; each end user installs the BMD runtime on their own machine and accepts Blackmagic's license directly.
- You can upgrade the SDK runtime without recompiling `warb`.

You do need the runtime installed on every machine that actually decodes `.braw` files:

1. **Download the Blackmagic RAW runtime** from Blackmagic Design's support portal at <https://www.blackmagicdesign.com/support/>. The runtime comes with Blackmagic Design's own license; read it before installing and make sure you understand the obligations it places on you.
2. **Point `warb` at the runtime directory** at runtime:

   ```sh
   export BRAW_RUNTIME_DIR=/opt/BlackmagicRAW/BlackmagicRawAPI
   ```

   (The directory that contains `libBlackmagicRawAPI.so`.) If unset, the crate falls through to the default `dlopen` search path: `LD_LIBRARY_PATH`, `/etc/ld.so.conf.d/*`, and so on. When the library cannot be found, `Codec::new` / `Decoder::new` return a `BrawError` carrying the `dlopen` failure message.

## API overview

Two layers, both in `warb`:

**High level: `Decoder`.** Synchronous, one frame in, one `ProcessedImage` out. Good for scrubbing, thumbnail extraction, or anything that doesn't care about throughput. Shown above.

**Low level: handles + `Callback` trait.** The shape of the SDK's C++ interfaces mapped 1:1 onto Rust types:

```
Codec   ─ open_clip ─→ Clip ─ create_read_job ─→ Job ─(submit)─╮
                                                               │  SDK worker threads
  Callback trait ← on_read_complete ←──────────── Frame ←──────╯
      │
      └── (set format, set scale, create_decode_and_process_job, submit)
            ↓
          on_process_complete ← ProcessedImage
                                   │
                                   └── data() → &[u8]
```

You implement `Callback` (`on_read_complete` / `on_process_complete`) and route events however you like: channel, atomics, whatever. Methods take `&self` and the SDK may dispatch them from multiple worker threads concurrently, so use interior mutability if you need it. `examples/to_rgba_pipelined.rs` shows a sliding-window pipelined decoder that sustains ~50 fps on 6K, ~200 fps at quarter scale.

## Examples

Both examples print the matching `ffmpeg` invocation on stderr, filled in with the actual `-video_size` and `-framerate` for your clip; copy it verbatim rather than guessing.

```sh
# one full-res frame out as raw RGBA
cargo run --release --example to_rgba -- clip.braw 1 > frame0.rgba

# full clip at quarter scale; see stderr for the matching ffmpeg command
cargo run --release --example to_rgba_pipelined -- clip.braw "" quarter > frames.rgba
```

## Features

| Feature | Effect |
| --- | --- |
| `trace` | Emit `[warb] …` callback/vtable tracing to stderr from the C++ shim. Useful when debugging against a new SDK runtime. Zero cost when off. |

## Testing

Two tiers. Unit tests run anywhere; SDK-gated tests only on a machine that has the Blackmagic RAW runtime + a clip.

**Pure-Rust unit tests** cover the format/enum plumbing (`ResourceFormat` round-trip, `bytes_per_pixel`, `HResult` sign classification, `BrawError` display, …) and run with no SDK present:

```sh
cargo test
```

**SDK integration tests** (`tests/sdk_roundtrip.rs`) exercise the full decode path — open a clip, decode frame 0 at full and quarter scale, verify dimensions / padding / non-zero pixels. They self-skip (printing a notice and returning Ok) when either of these env vars is absent, so CI without the SDK stays green:

```sh
BRAW_RUNTIME_DIR=/opt/BlackmagicRAW/BlackmagicRawAPI \
BRAW_TEST_CLIP=/path/to/sample.braw \
cargo test --test sdk_roundtrip
```

`BRAW_TEST_CLIP` is any `.braw` file you have the right to read; the crate doesn't ship one. A camera-native clip from any Blackmagic camera you own is the obvious choice.

**CI** (`.github/workflows/ci.yml`) runs `cargo-deny`, rustfmt, clippy, and the test suite across stable/beta/nightly/MSRV. The SDK-gated tests self-skip there; the unit tests always run.

## SDK quirks you'll hit

Behaviours discovered empirically, not documented in the SDK manual. `warb` already handles them at the API boundary, but they're good to know:

- **`ResolutionScale::Eighth` is silently clamped to `Quarter`** on some BRAW variants (observed on 6048×4032 Q0 clips). Always trust `ProcessedImage::width()` / `height()` over `ResolutionScale::divisor()`.
- **`ProcessedImage` buffers are padded to a 2048-byte boundary.** `data()` already truncates to the real pixel extent; `full_data()` exposes the padded buffer if you want it.
- **`Job::submit()` consumes the handle.** The SDK takes ownership inside `Submit()`; keeping your Rust-side reference past submit deadlocks `Codec::flush_jobs()`. The `self`-consuming signature makes this unrepresentable.
- **`PreparePipeline` is mandatory**, not an optimisation. `Codec::prepare_pipeline` must be called once before any decode. `Decoder::new` does this for you.

## Performance

Measured on a Ryzen 5950X decoding a 6048×4032 Q0 clip with `libBlackmagicRawAPI.so` 5.1:

| Mode | fps (full scale) | fps (quarter) |
| --- | --- | --- |
| `Decoder` (one frame at a time) | 17.8 | 180 |
| Pipelined (window = 24) | 51 | 200+ |

The high-level `Decoder` is synchronous by design. For real work pipe into ffmpeg (or keep your own window of in-flight jobs) via the low-level API.

## Licensing

- **`warb` itself** (Rust + C++ shim) is MIT-licensed. See `LICENSE`.
- **The Blackmagic RAW SDK and its runtime libraries are separately licensed** by Blackmagic Design. This crate does not redistribute the SDK and does not sublicense it. Your use of the runtime is governed entirely by Blackmagic Design's own agreement.
- **If you build products that ship or depend on the BMD runtime**, you are responsible for reading Blackmagic Design's license yourself and operating within its terms. Nothing in this README should be read as legal advice about what that license does or does not permit.

## Trademarks

"Blackmagic", "Blackmagic Design", "Blackmagic RAW", "BRAW", "URSA", "Pocket Cinema Camera", and related marks are property of Blackmagic Design Pty. Ltd. This crate is described only as "compatible with Blackmagic Design Blackmagic RAW"; it is not offered as a Blackmagic-branded product and has no affiliation with Blackmagic Design.
