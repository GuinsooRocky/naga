use std::f64::consts::PI;

#[derive(Clone, Copy)]
enum Click {
    Silent,
    Accent,
    Normal,
}

/// 按采样点数把节拍声写进音频流。节拍位置用小数累加、落到最近的采样点，
/// 所以每拍误差不到一个采样点，也不会越走越偏。
pub struct BeatScheduler {
    pub bpm: f64,
    pub beats_per_bar: u32,
    pub accent: bool,
    sample_rate: u32,
    frame: i64,
    last_beat: f64,
    started: bool,
    beat_index: u32,
    click: Click,
    click_pos: usize,
    accent_click: Vec<f32>,
    normal_click: Vec<f32>,
}

impl BeatScheduler {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            bpm: 120.0,
            beats_per_bar: 4,
            accent: true,
            sample_rate,
            frame: 0,
            last_beat: 0.0,
            started: false,
            beat_index: 0,
            click: Click::Silent,
            click_pos: 0,
            accent_click: make_click(sample_rate, 1600.0),
            normal_click: make_click(sample_rate, 1000.0),
        }
    }

    /// 写满 out；每响一拍回调一次（这一拍的采样位置，小节里第几拍）。
    pub fn render(&mut self, out: &mut [f32], mut on_beat: impl FnMut(i64, u32)) {
        let interval = self.sample_rate as f64 * 60.0 / self.bpm;
        for sample in out.iter_mut() {
            if !self.started || self.frame as f64 >= self.last_beat + interval {
                let ideal = self.last_beat + interval;
                // 速度没变时沿用理想位置；刚调快、理想位置已经过去时从当前点重新起算
                self.last_beat = if self.started && self.frame as f64 - ideal < 1.0 { ideal } else { self.frame as f64 };
                self.started = true;
                if self.beat_index >= self.beats_per_bar {
                    self.beat_index = 0;
                }
                self.click = if self.accent && self.beat_index == 0 { Click::Accent } else { Click::Normal };
                self.click_pos = 0;
                on_beat(self.frame, self.beat_index);
                self.beat_index = (self.beat_index + 1) % self.beats_per_bar;
            }
            let wave: &[f32] = match self.click {
                Click::Silent => &[],
                Click::Accent => &self.accent_click,
                Click::Normal => &self.normal_click,
            };
            *sample = match wave.get(self.click_pos) {
                Some(v) => {
                    self.click_pos += 1;
                    *v
                }
                None => 0.0,
            };
            self.frame += 1;
        }
    }
}

/// 30 毫秒的「哒」：正弦从零相位起再指数衰减，起音没有跳变，不会爆音。
fn make_click(sample_rate: u32, freq: f64) -> Vec<f32> {
    let n = (sample_rate * 30 / 1000) as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            (0.9 * (2.0 * PI * freq * t).sin() * (-t / 0.006).exp()) as f32
        })
        .collect()
}
