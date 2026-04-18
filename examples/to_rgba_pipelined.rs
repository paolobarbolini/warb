// Pipelined .braw → raw RGBA decoder. Keeps `WINDOW` read jobs in flight,
// chains each one's decode+process from inside `on_read_complete`, and
// emits images on stdout in frame order. This is the shape of use the
// low-level API is built for.
//
//   BRAW_RUNTIME_DIR=/path/to/BlackmagicRawAPI \
//   cargo run --release --example to_rgba_pipelined -- clip.braw > out.rgba

use std::{
    collections::HashMap,
    env,
    io::{self, Write},
    path::PathBuf,
    sync::mpsc,
    time::Instant,
};

use warb::{
    Callback, Codec, Frame, HResult, Pipeline, ProcessedImage, ResolutionScale, ResourceFormat,
};

const WINDOW: u64 = 24;

fn parse_scale(s: &str) -> Option<ResolutionScale> {
    match s.to_ascii_lowercase().as_str() {
        "full" | "1" => Some(ResolutionScale::Full),
        "half" | "2" => Some(ResolutionScale::Half),
        "quarter" | "qrtr" | "4" => Some(ResolutionScale::Quarter),
        "eighth" | "eith" | "8" => Some(ResolutionScale::Eighth),
        _ => None,
    }
}

enum Event {
    Image { slot: u64, image: ProcessedImage },
    Failure { slot: u64, what: String },
}

struct PipelinedHandler {
    tx: mpsc::Sender<Event>,
    format: ResourceFormat,
    scale: ResolutionScale,
}

impl PipelinedHandler {
    fn fail(&self, slot: u64, what: impl Into<String>) {
        let _ = self.tx.send(Event::Failure {
            slot,
            what: what.into(),
        });
    }
}

impl Callback for PipelinedHandler {
    // Called from an SDK worker thread once the bitstream is on-host. We
    // chain straight into decode+process so the worker pool stays busy.
    fn on_read_complete(&self, slot: u64, result: HResult, frame: Option<Frame>) {
        if result.is_err() {
            return self.fail(slot, format!("read failed: {result}"));
        }
        let Some(mut frame) = frame else {
            return self.fail(slot, "read returned no frame");
        };
        if let Err(e) = frame.set_resource_format(self.format) {
            return self.fail(slot, format!("SetResourceFormat: {e}"));
        }
        if let Err(e) = frame.set_resolution_scale(self.scale) {
            return self.fail(slot, format!("SetResolutionScale: {e}"));
        }
        let mut job = match frame.create_decode_and_process_job() {
            Ok(j) => j,
            Err(e) => return self.fail(slot, format!("CreateJobDecodeAndProcessFrame: {e}")),
        };
        if let Err(e) = job.set_user_data(slot) {
            return self.fail(slot, format!("SetUserData: {e}"));
        }
        if let Err(e) = job.submit() {
            return self.fail(slot, format!("decode+process Submit: {e}"));
        }
    }

    fn on_process_complete(&self, slot: u64, result: HResult, image: Option<ProcessedImage>) {
        match (result.is_ok(), image) {
            (true, Some(image)) => {
                let _ = self.tx.send(Event::Image { slot, image });
            }
            (_, _) => self.fail(slot, format!("process failed: {result}")),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/braw-sample/A001_09091040_C068.braw"));
    let limit: Option<u64> = env::args().nth(2).and_then(|s| s.parse().ok());
    let scale = env::args()
        .nth(3)
        .map(|s| parse_scale(&s).expect("scale must be full/half/quarter/eighth"))
        .unwrap_or(ResolutionScale::Full);

    let (tx, rx) = mpsc::channel();
    let format = ResourceFormat::RgbaU8;
    let mut codec = Codec::new(PipelinedHandler { tx, format, scale })?;
    codec.prepare_pipeline(Pipeline::Cpu)?;
    codec.flush_jobs();

    let mut clip = codec.open_clip(&input)?;
    let (fw, fh) = (clip.width(), clip.height());
    let fps = clip.frame_rate();
    let total = limit.unwrap_or_else(|| clip.frame_count());

    eprintln!("clip {fw}x{fh} @ {fps} fps, decoding {total} frames at {scale:?}, window={WINDOW}");

    // Frames are multi-MB, already larger than any reasonable BufWriter
    // buffer, so buffering would be a no-op — write to the raw stdout.
    let mut out = io::stdout().lock();

    // Prime the window.
    let mut next_read: u64 = 0;
    for _ in 0..WINDOW.min(total) {
        let mut job = clip.create_read_job(next_read)?;
        job.set_user_data(next_read)?;
        job.submit()?;
        next_read += 1;
    }

    // Drain the channel, emit images in order, top up the window as we go.
    let mut pending: HashMap<u64, ProcessedImage> = HashMap::new();
    let mut emit: u64 = 0;
    let start = Instant::now();

    while emit < total {
        match rx.recv()? {
            Event::Image { slot, image } => {
                pending.insert(slot, image);
            }
            Event::Failure { slot, what } => {
                return Err(format!("slot {slot}: {what}").into());
            }
        }
        while let Some(image) = pending.remove(&emit) {
            if emit == 0 {
                let (iw, ih) = (image.width(), image.height());
                eprintln!(
                    "first image {iw}x{ih} {:?}, full buffer {} B, pixel bytes {} B",
                    image.format(),
                    image.size_bytes(),
                    image.data().len(),
                );
                eprintln!(
                    "  ffmpeg -f rawvideo -pixel_format rgba -video_size {iw}x{ih} \
                     -framerate {fps} -i - -c:v libx264 -preset ultrafast -crf 23 \
                     -pix_fmt yuv420p out.mp4"
                );
            }
            // ProcessedImage derefs to the truncated pixel slice; SDK tail
            // padding is dropped so the rawvideo stream stays aligned.
            out.write_all(&image)?;
            drop(image);
            emit += 1;

            if next_read < total {
                let mut job = clip.create_read_job(next_read)?;
                job.set_user_data(next_read)?;
                job.submit()?;
                next_read += 1;
            }
            if emit % 24 == 0 || emit == total {
                let el = start.elapsed().as_secs_f64();
                eprint!(
                    "\r  frame {emit}/{total} ({:.2} fps, {:.0}% of realtime)    ",
                    emit as f64 / el,
                    (emit as f64 / el) * 100.0 / fps as f64
                );
            }
        }
    }
    codec.flush_jobs();
    out.flush()?;
    eprintln!();
    Ok(())
}
