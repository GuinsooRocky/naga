mod common;

use std::f64::consts::PI;

use common::JavaRandom;
use naga_core::tuning::{cents, in_tune, string_freq, Reading, TunerProcessor, STRING_COUNT};

const SAMPLE_RATE: u32 = 44100;
const HOP: usize = 2048;
const PLAIN: [f64; 5] = [0.3, 1.0, 0.6, 0.4, 0.2];
const WEAK_FUNDAMENTAL: [f64; 5] = [0.1, 1.0, 0.5, 0.8, 0.3];

/// 相位连续的合成琴声，频率和音量可以随时间变；底噪约 -80 dBFS。
fn tone(seconds: f64, freq_at: impl Fn(f64) -> f64, amp_at: impl Fn(f64) -> f64, harmonics: &[f64], seed: i64) -> Vec<f32> {
    let mut random = JavaRandom::new(seed);
    let offsets: Vec<f64> = harmonics.iter().map(|_| random.next_double() * 2.0 * PI).collect();
    let mut phase = 0.0;
    (0..(seconds * SAMPLE_RATE as f64) as usize)
        .map(|i| {
            let t = i as f64 / SAMPLE_RATE as f64;
            phase += 2.0 * PI * freq_at(t) / SAMPLE_RATE as f64;
            let mut v = 0.0;
            for (h, amp) in harmonics.iter().enumerate() {
                v += amp * ((h + 1) as f64 * phase + offsets[h]).sin();
            }
            (amp_at(t) * v + 1e-4 * random.next_gaussian()) as f32
        })
        .collect()
}

/// 按录音循环的节奏一段段喂进去；第 i 个读数在 (i+1)×HOP 个采样时产出。
fn feed(signal: &[f32], locked: Option<usize>) -> Vec<Option<Reading>> {
    let mut processor = TunerProcessor::new(SAMPLE_RATE, 4096);
    signal.chunks_exact(HOP).map(|block| processor.process(block, locked)).collect()
}

fn time_of(i: usize) -> f64 {
    (i + 1) as f64 * HOP as f64 / SAMPLE_RATE as f64
}

fn at(cents: f64, base: f64) -> f64 {
    base * 2f64.powf(cents / 1200.0)
}

#[test]
fn green_rule_enters_at_three_and_holds_up_to_six() {
    assert!(in_tune(3.0, false));
    assert!(in_tune(-3.0, false));
    assert!(!in_tune(3.1, false));
    assert!(!in_tune(-5.0, false));
    assert!(in_tune(6.0, true));
    assert!(in_tune(-5.9, true));
    assert!(!in_tune(6.1, true));
    assert!(!in_tune(-7.0, true));
}

#[test]
fn stays_green_while_wobbling_within_six_and_resets_after_silence() {
    let a = string_freq(1);
    // 先 +1 音分，滑到 +5，滑出到 +7.5，滑回 +5，再滑到 +2；之后静音，再来一次直接 +5 的新音
    let segments = [(0.0, 1.0), (0.6, 1.0), (0.8, 5.0), (1.4, 5.0), (1.6, 7.5), (2.2, 7.5), (2.4, 5.0), (3.0, 5.0),
        (3.2, 2.0), (3.8, 2.0)];
    let cents_at = |t: f64| {
        let k = segments.iter().rposition(|s| s.0 <= t).unwrap();
        if k == segments.len() - 1 {
            return segments[k].1;
        }
        let ((t0, c0), (t1, c1)) = (segments[k], segments[k + 1]);
        c0 + (c1 - c0) * (t - t0) / (t1 - t0)
    };
    let signal = tone(5.2, |t| at(if t < 4.4 { cents_at(t) } else { 5.0 }, a),
        |t| if (3.8..=4.4).contains(&t) { 0.0 } else { 0.03 }, &PLAIN, 1);
    let readings = feed(&signal, None);
    let window = |from: f64, to: f64| -> Vec<Option<Reading>> {
        (0..readings.len()).filter(|&i| (from..=to).contains(&time_of(i))).map(|i| readings[i]).collect()
    };

    let first_green = readings.iter().position(|r| r.is_some_and(|r| r.in_tune)).unwrap();
    assert!(time_of(first_green) < 0.3, "变绿太晚：{} 秒", time_of(first_green));
    for r in window(time_of(first_green), 1.4) {
        assert!(r.unwrap().in_tune, "在 ±6 内抖动时闪了：{r:?}");
    }
    assert!(window(0.0, 0.6).iter().any(|r| r.is_some_and(|r| r.tuned)));
    for r in window(2.0, 2.2) {
        assert!(!r.unwrap().in_tune, "超出 ±6 仍是绿的：{r:?}");
    }
    for r in window(2.7, 3.0) {
        assert!(!r.unwrap().in_tune, "出过 ±6 后没回到 ±3 就又绿了：{r:?}");
    }
    for r in window(3.5, 3.8) {
        assert!(r.unwrap().in_tune, "回到 ±3 内没变绿：{r:?}");
    }
    for r in window(4.2, 4.4) {
        assert_eq!(None, r, "静音时还有读数");
    }
    let fresh: Vec<Reading> = window(4.4, 5.2).into_iter().flatten().collect();
    assert!(!fresh.is_empty());
    for r in &fresh {
        assert!(!r.in_tune, "新的一声直接 +5 音分不该变绿：{r:?}");
    }
    assert_eq!(1, fresh.last().unwrap().string_index);
}

#[test]
fn first_reading_within_point_three_seconds_and_steady_through_whole_decay() {
    let lead = 0.25;
    let mut slowest = 0.0f64;
    let mut worst = 0.0f64;
    for harmonics in [&PLAIN, &WEAK_FUNDAMENTAL] {
        for index in 0..STRING_COUNT {
            let number = 6 - index;
            let signal = tone(lead + 3.0, |_| string_freq(index),
                |t| if t < lead { 0.0 } else { 0.03 * (-(t - lead) / 1.5).exp() }, harmonics, index as i64 + 1);
            let readings = feed(&signal, None);
            let first = readings.iter().position(Option::is_some).unwrap_or_else(|| panic!("{number} 弦一直没有读数"));
            let delay = time_of(first) - lead;
            slowest = slowest.max(delay);
            assert!(delay <= 0.3, "{number} 弦 {delay} 秒才出读数");
            // 衰减的 3 秒里读数不断、不换弦、不跳八度
            for r in &readings[first..] {
                let r = r.unwrap_or_else(|| panic!("{number} 弦读数中途断了"));
                assert_eq!(index, r.string_index, "{number} 弦被认成别的弦：{r:?}");
                worst = worst.max((r.cents as f64).abs());
            }
        }
    }
    assert!(worst < 1.0, "一次拨弦中读数最大偏 {worst} 音分");
    println!("最慢出读数 = {slowest:.3} 秒，衰减全程最大偏差 = {worst:.3} 音分");
}

#[test]
fn locked_string_measures_against_that_string_even_far_away() {
    let a = string_freq(1);
    let signal = tone(1.0, |_| a, |_| 0.03, &PLAIN, 1);
    let auto: Vec<Reading> = feed(&signal, None).into_iter().flatten().collect();
    assert_eq!(1, auto.last().unwrap().string_index);
    assert!(auto.last().unwrap().cents.abs() <= 1.0);
    let locked: Vec<Reading> = feed(&signal, Some(0)).into_iter().flatten().collect();
    assert_eq!(0, locked.last().unwrap().string_index);
    assert!((locked.last().unwrap().cents - 500.0).abs() <= 1.0);
    assert!(locked.iter().all(|r| !r.in_tune));
}

#[test]
fn covers_slack_low_string_up_to_above_high_e() {
    for freq in [65.0, 450.0] {
        let r = feed(&tone(1.0, |_| freq, |_| 0.03, &PLAIN, 1), None).into_iter().flatten().last().unwrap();
        let got = at(r.cents as f64, string_freq(r.string_index));
        assert!(cents(got, freq).abs() <= 1.0, "{freq} Hz：测得 {got}");
    }
}
