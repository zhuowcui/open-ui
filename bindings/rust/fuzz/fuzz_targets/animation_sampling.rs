#![no_main]

use libfuzzer_sys::fuzz_target;
use openui_style::{
    AnimationOptions, Easing, FillMode, IterationCount, PlaybackDirection, StepPosition,
};

fn number(bytes: &[u8]) -> f64 {
    let mut raw = [0_u8; 8];
    raw[..bytes.len().min(8)].copy_from_slice(&bytes[..bytes.len().min(8)]);
    f64::from_bits(u64::from_le_bytes(raw))
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }
    let easing = match data[0] % 3 {
        0 => Easing::CubicBezier {
            x1: number(&data[1..]),
            y1: number(data.get(2..).unwrap_or_default()),
            x2: number(data.get(3..).unwrap_or_default()),
            y2: number(data.get(4..).unwrap_or_default()),
        },
        1 => Easing::Steps {
            count: u32::from(data[1]),
            position: if data[2] & 1 == 0 {
                StepPosition::JumpStart
            } else {
                StepPosition::JumpEnd
            },
        },
        _ => Easing::Linear,
    };
    let options = AnimationOptions {
        delay_ms: number(&data[1..]),
        duration_ms: number(data.get(2..).unwrap_or_default()),
        iterations: IterationCount::Number(number(data.get(3..).unwrap_or_default())),
        direction: match data[4] & 3 {
            0 => PlaybackDirection::Normal,
            1 => PlaybackDirection::Reverse,
            2 => PlaybackDirection::Alternate,
            _ => PlaybackDirection::AlternateReverse,
        },
        fill: FillMode::Both,
        playback_rate: number(data.get(5..).unwrap_or_default()),
        easing,
        ..AnimationOptions::default()
    };
    let _ = options.validate();
    for chunk in data.chunks(8).take(64) {
        let _ = options.sample(number(chunk));
    }
});
