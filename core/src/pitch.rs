const MIN_FREQ: f32 = 60.0;
const MAX_FREQ: f32 = 1200.0;
const THRESHOLD: f32 = 0.15;

/// YIN 基频检测。按「第一个足够深的谷」取周期而不是取最深的谷，
/// 这样泛音比基音响时也不会认成高八度。
pub struct PitchDetector {
    sample_rate: f32,
    window_size: usize,
    tau_min: usize,
    tau_max: usize,
    w: usize,
    d: Vec<f32>,
}

impl PitchDetector {
    pub fn new(sample_rate: u32, window_size: usize) -> Self {
        let tau_max = (sample_rate as f32 / MIN_FREQ) as usize;
        let tau_min = ((sample_rate as f32 / MAX_FREQ) as usize).max(2);
        assert!(window_size > 2 * tau_max + 1, "window too small for MIN_FREQ");
        Self {
            sample_rate: sample_rate as f32,
            window_size,
            tau_min,
            tau_max,
            w: window_size - tau_max - 1,
            d: vec![0.0; tau_max + 2],
        }
    }

    /// 返回频率（Hz）；听不出稳定音高时返回 None。x 至少要有 window_size 个采样。
    pub fn detect(&mut self, x: &[f32]) -> Option<f32> {
        let x = &x[..self.window_size];
        let w = self.w;
        let d = &mut self.d;
        let mut cum = 0.0f64;
        d[0] = 1.0;
        for tau in 1..=self.tau_max + 1 {
            let mut s = 0.0f32;
            for (a, b) in x[..w].iter().zip(&x[tau..tau + w]) {
                let diff = a - b;
                s += diff * diff;
            }
            cum += s as f64;
            d[tau] = if cum > 0.0 { ((s * tau as f32) as f64 / cum) as f32 } else { 1.0 };
            let prev = tau - 1;
            if prev >= self.tau_min && d[prev] < THRESHOLD && d[tau] >= d[prev] {
                let (a, b, c) = (d[prev - 1], d[prev], d[tau]);
                let denom = a - 2.0 * b + c;
                let shift = if denom != 0.0 { 0.5 * (a - c) / denom } else { 0.0 };
                return Some(self.sample_rate / (prev as f32 + shift));
            }
        }
        None
    }
}
