// Decode a .braw clip to a raw RGBA stream on stdout.
//
// Usage:
//   BRAW_RUNTIME_DIR=/path/to/BlackmagicRawAPI \
//   cargo run --release --example to_rgba -- CLIP.braw [FRAME_LIMIT] > out.rgba
//
// A ready-to-use ffmpeg command is printed on stderr, copy it verbatim.

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let Some(input) = args.next().map(PathBuf::from) else {
        eprintln!("usage: to_rgba CLIP.braw [FRAME_LIMIT] > out.rgba");
        process::exit(2);
    };
    // Optional second arg caps the frame count (useful for smoke-testing).
    let limit: Option<u64> = args
        .next()
        .and_then(|s| s.into_string().ok())
        .and_then(|s| s.parse().ok());

    let mut decoder = warb::Decoder::new()?;
    decoder.open(&input)?;
    let w = decoder.width();
    let h = decoder.height();
    let n = limit.unwrap_or_else(|| decoder.frame_count());
    let fps = decoder.frame_rate();

    eprintln!("{w}x{h} @ {fps} fps, {n} frames → raw RGBA on stdout");
    eprintln!("Pipe into ffmpeg with:");
    eprintln!(
        "    ffmpeg -f rawvideo -pixel_format rgba -video_size {w}x{h} \
         -framerate {fps} -i - -c:v libx264 -pix_fmt yuv420p out.mp4"
    );

    // Frames are multi-MB, already larger than any reasonable BufWriter
    // buffer, so buffering would be a no-op — write to the raw stdout.
    let mut out = io::stdout().lock();

    for idx in 0..n {
        let image = decoder.decode_frame(idx, warb::ResourceFormat::RgbaU8)?;
        out.write_all(&image)?;
        if idx % 24 == 0 {
            eprint!("\r  frame {}/{}", idx + 1, n);
        }
    }
    eprintln!("\r  frame {n}/{n} done");
    out.flush()?;
    Ok(())
}
