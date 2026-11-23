use naga_core::beat::BeatScheduler;

const SAMPLE_RATE: u32 = 48000;

fn run(scheduler: &mut BeatScheduler, frames: i64, beats: &mut Vec<(i64, u32)>) {
    let mut block = [0.0f32; 480];
    let mut done = 0;
    while done < frames {
        scheduler.render(&mut block, |frame, index| beats.push((frame, index)));
        done += block.len() as i64;
    }
}

#[test]
fn beats_land_on_exact_frames_and_cycle_through_bar() {
    let mut scheduler = BeatScheduler::new(SAMPLE_RATE);
    scheduler.bpm = 120.0;
    scheduler.beats_per_bar = 4;
    let mut beats = Vec::new();
    run(&mut scheduler, SAMPLE_RATE as i64 * 10, &mut beats);
    for (i, &(frame, index)) in beats.iter().enumerate() {
        assert_eq!(i as i64 * 24000, frame);
        assert_eq!(i as u32 % 4, index);
    }
    assert_eq!(20, beats.len());
}

#[test]
fn no_drift_over_thirty_minutes_at_awkward_tempo() {
    let mut scheduler = BeatScheduler::new(SAMPLE_RATE);
    scheduler.bpm = 137.0;
    let mut beats = Vec::new();
    run(&mut scheduler, SAMPLE_RATE as i64 * 60 * 30, &mut beats);
    let interval = SAMPLE_RATE as f64 * 60.0 / 137.0;
    let worst = beats.iter().enumerate().map(|(i, &(frame, _))| (frame as f64 - i as f64 * interval).abs()).fold(0.0, f64::max);
    assert!(worst <= 1.0, "最大偏差 {worst} 个采样点");
    // 第 137×30 拍正好落在第 SAMPLE_RATE×60×30 个采样点，也就是第一个没渲染的点，所以不在列表里
    assert_eq!(137 * 30, beats.len());
    println!("30 分钟 {} 拍，最大偏差 {worst:.3} 个采样点 = {:.4} 毫秒", beats.len(), worst * 1000.0 / SAMPLE_RATE as f64);
}

#[test]
fn speeding_up_takes_effect_without_waiting_out_the_old_beat() {
    let mut scheduler = BeatScheduler::new(SAMPLE_RATE);
    scheduler.bpm = 30.0;
    let mut beats = Vec::new();
    run(&mut scheduler, 4800, &mut beats); // 第一拍响过 0.1 秒
    scheduler.bpm = 240.0;
    run(&mut scheduler, SAMPLE_RATE as i64, &mut beats);
    assert_eq!(0, beats[0].0);
    assert_eq!(12000, beats[1].0); // 按新速度 0.25 秒后就响，而不是等满 2 秒
}

#[test]
fn accent_only_on_first_beat() {
    fn first_samples(accent: bool) -> (Vec<f32>, Vec<f32>) {
        let mut scheduler = BeatScheduler::new(SAMPLE_RATE);
        scheduler.bpm = 240.0;
        scheduler.beats_per_bar = 2;
        scheduler.accent = accent;
        let mut out = vec![0.0f32; SAMPLE_RATE as usize];
        scheduler.render(&mut out, |_, _| {});
        (out[0..200].to_vec(), out[12000..12200].to_vec())
    }
    let (first, second) = first_samples(true);
    assert_ne!(first, second);
    let (plain_first, plain_second) = first_samples(false);
    assert_eq!(plain_first, plain_second);
}
