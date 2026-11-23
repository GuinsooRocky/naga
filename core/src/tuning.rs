use crate::pitch::PitchDetector;

/// 标准调弦，下标 0 是 6 弦（E2），下标 5 是 1 弦（E4）。
const STRING_MIDI: [i32; 6] = [40, 45, 50, 55, 59, 64];
pub const STRING_COUNT: usize = STRING_MIDI.len();

pub const IN_TUNE_CENTS: f32 = 3.0;
pub const STAY_IN_TUNE_CENTS: f32 = 6.0;

/// 基准 A4 = 440Hz。
pub fn string_freq(index: usize) -> f64 {
    440.0 * 2f64.powf((STRING_MIDI[index] - 69) as f64 / 12.0)
}

pub fn cents(freq: f64, target: f64) -> f64 {
    1200.0 * (freq / target).ln() / 2f64.ln()
}

pub fn nearest(freq: f64) -> usize {
    let mut best = 0;
    for i in 1..STRING_COUNT {
        if cents(freq, string_freq(i)).abs() < cents(freq, string_freq(best)).abs() {
            best = i;
        }
    }
    best
}

/// 已经绿了就放宽到 ±6，读数在 ±3 边上抖时颜色才不会一闪一闪。
pub fn in_tune(cents: f32, was_in_tune: bool) -> bool {
    cents.abs() <= if was_in_tune { STAY_IN_TUNE_CENTS } else { IN_TUNE_CENTS }
}

const HISTORY: usize = 5;
const MIN_FRAMES: usize = 3;

/// 把逐帧的检测结果稳成一个读数：取最近几帧的中位数，丢掉单帧跳变。
#[derive(Default)]
pub struct ReadingStabilizer {
    history: [f32; HISTORY],
    len: usize,
    jumps: u32,
    misses: u32,
}

impl ReadingStabilizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// 传入本帧频率（没有就传 None），返回稳定后的频率。
    pub fn push(&mut self, freq: Option<f32>) -> Option<f32> {
        let Some(freq) = freq else {
            self.misses += 1;
            if self.misses >= 3 {
                self.len = 0;
            }
            return self.current();
        };
        self.misses = 0;
        if let Some(med) = self.median() {
            if cents(freq as f64, med as f64).abs() > 80.0 {
                self.jumps += 1;
                if self.jumps < 2 {
                    return self.current();
                }
                self.len = 0;
            }
        }
        self.jumps = 0;
        if self.len == HISTORY {
            self.history.copy_within(1.., 0);
            self.len -= 1;
        }
        self.history[self.len] = freq;
        self.len += 1;
        self.current()
    }

    fn current(&self) -> Option<f32> {
        if self.len >= MIN_FRAMES { self.median() } else { None }
    }

    fn median(&self) -> Option<f32> {
        if self.len == 0 {
            return None;
        }
        let mut sorted = self.history;
        let sorted = &mut sorted[..self.len];
        sorted.sort_unstable_by(f32::total_cmp);
        Some(sorted[self.len / 2])
    }
}

/// 一帧的读数。in_tune 是界面上该不该显示绿色；tuned 是这根弦这次算不算调准过。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub string_index: usize,
    pub cents: f32,
    pub in_tune: bool,
    pub tuned: bool,
}

// 按 CDD 对 VOICE_RECOGNITION 音源的灵敏度约定（90 dB 声压 ≈ -22 dBFS）换算：
// 起音约 66 dB，比一米外正常说话响；延音约 56 dB
pub const RMS_START: f32 = 0.005;
pub const RMS_HOLD: f32 = 0.0015;
const FREQ_RANGE: std::ops::RangeInclusive<f32> = 60.0..=500.0;
const TUNED_FRAMES: u32 = 4;

/// 录音循环每读到一段采样就调一次。
pub struct TunerProcessor {
    detector: PitchDetector,
    stabilizer: ReadingStabilizer,
    window: Vec<f32>,
    work: Vec<f32>,
    sounding: bool,
    last_index: Option<usize>,
    green: bool,
    green_frames: u32,
}

impl TunerProcessor {
    pub fn new(sample_rate: u32, window_size: usize) -> Self {
        Self {
            detector: PitchDetector::new(sample_rate, window_size),
            stabilizer: ReadingStabilizer::new(),
            window: vec![0.0; window_size],
            work: vec![0.0; window_size],
            sounding: false,
            last_index: None,
            green: false,
            green_frames: 0,
        }
    }

    pub fn window_size(&self) -> usize {
        self.window.len()
    }

    /// samples 是新读到的采样，不能多于 window_size 个；locked 是手动锁定的弦。
    /// 没听到稳定音高时返回 None。
    pub fn process(&mut self, samples: &[f32], locked: Option<usize>) -> Option<Reading> {
        let size = self.window.len();
        let n = samples.len();
        self.window.copy_within(n.., 0);
        self.window[size - n..].copy_from_slice(samples);

        let mut mean = 0.0f32;
        for v in &self.window {
            mean += v;
        }
        mean /= size as f32;
        let mut energy = 0.0f32;
        for (w, v) in self.work.iter_mut().zip(&self.window) {
            *w = v - mean;
            energy += *w * *w;
        }
        let rms = (energy / size as f32).sqrt();
        // 起音门槛高、延音门槛低：远处说话声进不来，琴声衰减时读数不断
        self.sounding = rms > if self.sounding { RMS_HOLD } else { RMS_START };

        let raw = if self.sounding { self.detector.detect(&self.work) } else { None };
        let Some(freq) = self.stabilizer.push(raw.filter(|f| FREQ_RANGE.contains(f))) else {
            self.last_index = None;
            self.green = false;
            self.green_frames = 0;
            return None;
        };
        let index = locked.unwrap_or_else(|| nearest(freq as f64));
        let cents = cents(freq as f64, string_freq(index)) as f32;
        self.green = in_tune(cents, self.green && self.last_index == Some(index));
        self.last_index = Some(index);
        self.green_frames = if self.green { self.green_frames + 1 } else { 0 };
        Some(Reading { string_index: index, cents, in_tune: self.green, tuned: self.green_frames >= TUNED_FRAMES })
    }
}
