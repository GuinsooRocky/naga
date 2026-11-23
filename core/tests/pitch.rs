mod common;

use std::f64::consts::PI;
use std::time::Instant;

use common::JavaRandom;
use naga_core::pitch::PitchDetector;
use naga_core::tuning::{cents, string_freq, ReadingStabilizer, STRING_COUNT};

const SAMPLE_RATE: u32 = 44100;
const WINDOW: usize = 4096;
const PLAIN: [f64; 5] = [0.3, 1.0, 0.6, 0.4, 0.2];

/// 合成一段拨弦声：基音弱、二次泛音最响，带衰减和底噪。
fn pluck(freq: f64, harmonics: &[f64], noise: f64, seed: i64) -> Vec<f32> {
    let mut random = JavaRandom::new(seed);
    let phases: Vec<f64> = harmonics.iter().map(|_| random.next_double() * 2.0 * PI).collect();
    (0..WINDOW)
        .map(|i| {
            let t = i as f64 / SAMPLE_RATE as f64;
            let mut v = 0.0;
            for (h, amp) in harmonics.iter().enumerate() {
                v += amp * (2.0 * PI * freq * (h + 1) as f64 * t + phases[h]).sin();
            }
            (0.1 * (v * (-t / 1.5).exp() + noise * random.next_gaussian())) as f32
        })
        .collect()
}

#[test]
fn every_string_within_one_cent_across_detunings() {
    let mut detector = PitchDetector::new(SAMPLE_RATE, WINDOW);
    let mut worst = 0.0f64;
    for string in 0..STRING_COUNT {
        for detune in [-40.0, -20.0, -3.0, -1.0, 0.0, 1.0, 3.0, 20.0, 40.0] {
            let freq = string_freq(string) * 2f64.powf(detune / 1200.0);
            for seed in 1..=5 {
                let got = detector.detect(&pluck(freq, &PLAIN, 0.01, seed)).expect("没有读数") as f64;
                let error = cents(got, freq).abs();
                worst = worst.max(error);
                assert!(error < 1.0, "{} 弦 偏 {detune} 音分：测得 {got}，误差 {error} 音分", 6 - string);
            }
        }
    }
    println!("最大误差 = {worst:.3} 音分");
}

#[test]
fn weak_fundamental_is_not_mistaken_for_octave_up() {
    let mut detector = PitchDetector::new(SAMPLE_RATE, WINDOW);
    let low_e = string_freq(0);
    let got = detector.detect(&pluck(low_e, &[0.1, 1.0, 0.5, 0.8, 0.3], 0.01, 1)).unwrap() as f64;
    assert!((got - low_e).abs() <= 0.1, "测得 {got}");
}

#[test]
fn slack_string_still_detected() {
    let mut detector = PitchDetector::new(SAMPLE_RATE, WINDOW);
    let got = detector.detect(&pluck(65.0, &PLAIN, 0.01, 1)).unwrap() as f64;
    assert!((got - 65.0).abs() <= 0.1, "测得 {got}");
}

#[test]
fn silence_and_noise_give_no_reading() {
    let mut detector = PitchDetector::new(SAMPLE_RATE, WINDOW);
    assert_eq!(None, detector.detect(&[0.0; WINDOW]));
    let mut random = JavaRandom::new(7);
    let noise: Vec<f32> = (0..WINDOW).map(|_| (0.1 * random.next_gaussian()) as f32).collect();
    assert_eq!(None, detector.detect(&noise));
}

#[test]
fn stabilizer_ignores_single_frame_jump() {
    let mut s = ReadingStabilizer::new();
    assert_eq!(None, s.push(Some(110.0)));
    assert_eq!(None, s.push(Some(110.0)));
    assert_eq!(Some(110.0), s.push(Some(110.0)));
    assert_eq!(Some(110.0), s.push(Some(220.0))); // 单帧跳八度被丢掉
    assert_eq!(Some(110.0), s.push(Some(110.0)));
    for _ in 0..3 {
        s.push(None);
    }
    assert_eq!(None, s.push(Some(110.0))); // 断了三帧后重新起算
}

/// 不是正确性测试，只量耗时：`cargo test --release -- --ignored --nocapture`。
#[test]
#[ignore]
fn time_per_window() {
    let mut detector = PitchDetector::new(SAMPLE_RATE, WINDOW);
    let mut random = JavaRandom::new(7);
    let noise: Vec<f32> = (0..WINDOW).map(|_| (0.1 * random.next_gaussian()) as f32).collect();
    let cases = [("噪声（扫满全部周期，最慢）", noise), ("6 弦 E2", pluck(string_freq(0), &PLAIN, 0.01, 1)),
        ("1 弦 E4", pluck(string_freq(5), &PLAIN, 0.01, 1))];
    for (name, x) in &cases {
        let rounds = 200;
        let start = Instant::now();
        for _ in 0..rounds {
            std::hint::black_box(detector.detect(std::hint::black_box(x)));
        }
        println!("{name}：每个 4096 样本窗口 {:.3} 毫秒", start.elapsed().as_secs_f64() * 1000.0 / rounds as f64);
    }
}
