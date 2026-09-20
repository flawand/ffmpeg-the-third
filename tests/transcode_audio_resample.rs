//! Regression test for https://github.com/shssoichiro/ffmpeg-the-third/issues/95
//!
//! `examples/transcode-audio.rs` is expected to let the caller pick the output
//! sample rate independently of the input file's sample rate. Before the fix,
//! the encoder's rate was hardcoded to the decoder's rate, so the output was
//! always re-resampled back to the input's rate no matter what filter (e.g.
//! `aresample=44100`) or explicit rate argument was requested.

use std::path::PathBuf;
use std::process::Command;

use ffmpeg_the_third as ffmpeg;
use ffmpeg::{codec, format, media};

const INPUT_RATE: u32 = 48_000;
const TARGET_RATE: u32 = 44_100;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// Generates a short sine-wave WAV fixture at `INPUT_RATE` using the system
/// `ffmpeg` binary, so this test has no checked-in binary fixture.
fn generate_input_wav(path: &std::path::Path) {
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:duration=1:sample_rate={INPUT_RATE}"),
            path.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run system ffmpeg to generate the test fixture");

    assert!(status.success(), "ffmpeg fixture generation failed");
}

fn output_sample_rate(path: &std::path::Path) -> u32 {
    ffmpeg::init().unwrap();

    let ictx = format::input(path).unwrap();
    let stream = ictx
        .streams()
        .best(media::Type::Audio)
        .expect("no audio stream in transcoder output");
    let context = codec::context::Context::from_parameters(stream.parameters()).unwrap();
    let decoder = context.decoder().audio().unwrap();

    decoder.rate()
}

#[test]
fn transcode_audio_honors_requested_output_sample_rate() {
    let work_dir = std::env::temp_dir().join("ffmpeg_the_third_test_issue_95");
    let _ = std::fs::remove_dir_all(&work_dir);
    std::fs::create_dir_all(&work_dir).unwrap();

    let input_path = work_dir.join("input.wav");
    let output_path = work_dir.join("output.wav");

    generate_input_wav(&input_path);

    // No seek requested (4th arg left empty), target sample rate 44100 (5th arg).
    let status = Command::new(cargo_bin())
        .current_dir(manifest_dir())
        .args([
            "run",
            "--quiet",
            "--example",
            "transcode-audio",
            "--",
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "aresample=44100",
            "",
            &TARGET_RATE.to_string(),
        ])
        .status()
        .expect("failed to run transcode-audio example");

    assert!(status.success(), "transcode-audio example exited with an error");

    let actual_rate = output_sample_rate(&output_path);
    assert_eq!(
        actual_rate, TARGET_RATE,
        "expected transcode-audio to produce {TARGET_RATE}Hz output, got {actual_rate}Hz \
         (input was {INPUT_RATE}Hz) -- see issue #95"
    );

    let _ = std::fs::remove_dir_all(&work_dir);
}
