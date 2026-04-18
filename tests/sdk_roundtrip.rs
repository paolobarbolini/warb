//! Integration test that exercises the full decode path against a real
//! BRAW clip. Gated on two environment variables so CI without the SDK
//! skips silently:
//!
//! - `BRAW_RUNTIME_DIR` — directory containing `libBlackmagicRawAPI.so`.
//! - `BRAW_TEST_CLIP`   — path to a `.braw` file to open.
//!
//! Run locally with:
//! ```sh
//! BRAW_RUNTIME_DIR=/opt/BlackmagicRAW/BlackmagicRawAPI \
//! BRAW_TEST_CLIP=/path/to/sample.braw \
//! cargo test --test sdk_roundtrip
//! ```
//!
//! Intentionally does **not** attempt to download a clip or the runtime
//! from anywhere: the crate's license rules (no redistribution of the
//! SDK) and general-purpose-repo hygiene (no multi-GB test fixtures)
//! both argue against it.

use std::{env, path::PathBuf};

fn test_clip() -> Option<PathBuf> {
    let p = env::var_os("BRAW_TEST_CLIP").map(PathBuf::from)?;
    if !p.is_file() {
        eprintln!(
            "BRAW_TEST_CLIP={} is not a regular file; skipping.",
            p.display()
        );
        return None;
    }
    if env::var_os("BRAW_RUNTIME_DIR").is_none() {
        eprintln!("BRAW_TEST_CLIP is set but BRAW_RUNTIME_DIR is not; skipping.");
        return None;
    }
    Some(p)
}

#[test]
fn decode_first_frame_metadata_and_pixels() {
    let Some(clip_path) = test_clip() else {
        eprintln!("SDK integration test skipped (env vars unset).");
        return;
    };

    let mut decoder = warb::Decoder::new().expect("Decoder::new");
    decoder.open(&clip_path).expect("Decoder::open");

    // Clip metadata should be sensible — non-zero dims, at least one frame.
    assert!(decoder.width() > 0, "width must be > 0");
    assert!(decoder.height() > 0, "height must be > 0");
    assert!(decoder.frame_count() > 0, "frame_count must be > 0");
    assert!(decoder.frame_rate() > 0.0, "frame_rate must be > 0");

    let image = decoder
        .decode_frame(0, warb::ResourceFormat::RgbaU8)
        .expect("decode_frame(0)");

    // Dimensions come back as asked (no silent clamping here since we
    // requested Full scale).
    assert_eq!(image.width(), decoder.width());
    assert_eq!(image.height(), decoder.height());
    assert_eq!(image.format(), warb::ResourceFormat::RgbaU8);

    // `data()` must match width * height * 4 exactly (the truncation
    // contract) and differ from `full_data()` only by the SDK's 2048-byte
    // tail padding.
    let expected_pixel_bytes = (image.width() as usize) * (image.height() as usize) * 4;
    assert_eq!(image.data().len(), expected_pixel_bytes);
    assert!(image.full_data().len() >= image.data().len());
    assert_eq!(
        image.full_data().len() % 2048,
        0,
        "SDK pads allocations to 2048-byte boundary"
    );

    // Sanity: the alpha channel of RGBA_U8 is fully opaque for a real
    // decode, and at least one R/G/B byte is non-zero.
    assert!(
        image.data().chunks_exact(4).all(|px| px[3] == 0xFF),
        "alpha channel should be 0xFF on RGBA_U8 output"
    );
    assert!(
        image.data().iter().any(|&b| b != 0),
        "decoded frame must not be all zeros"
    );
}

#[test]
fn decode_at_quarter_scale_downsamples_dimensions() {
    let Some(clip_path) = test_clip() else {
        eprintln!("SDK integration test skipped (env vars unset).");
        return;
    };

    let mut decoder = warb::Decoder::new().expect("Decoder::new");
    decoder.open(&clip_path).expect("Decoder::open");
    decoder.set_scale(warb::ResolutionScale::Quarter);

    let (fw, fh) = (decoder.width(), decoder.height());
    let image = decoder
        .decode_frame(0, warb::ResourceFormat::RgbaU8)
        .expect("decode_frame(0) at quarter scale");

    // At least one of the dimensions must have shrunk; allow for the
    // silent Eighth → Quarter clamping the SDK does on some clips.
    assert!(image.width() <= fw && image.height() <= fh);
    assert!(
        image.width() * 4 <= fw || image.height() * 4 <= fh,
        "quarter request should produce meaningfully smaller output"
    );
}
