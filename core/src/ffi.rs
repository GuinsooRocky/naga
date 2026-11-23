//! JNI 入口。对应 Kotlin 的 app.naga.tuner.TunerNative 和 app.naga.metronome.BeatNative。
//! 每个入口都包在 catch_unwind 里，panic 不会穿过 FFI；出错时返回「没有结果」。
//! 句柄是 Box 的裸指针，create 时交出、destroy 时收回，0 表示没有。

use std::panic::{catch_unwind, AssertUnwindSafe};

use jni::objects::{JFloatArray, JLongArray, JObject};
use jni::sys::{jboolean, jdouble, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;

use crate::beat::BeatScheduler;
use crate::tuning::{TunerProcessor, STRING_COUNT};

fn guarded<T>(fallback: T, body: impl FnOnce() -> Option<T>) -> T {
    catch_unwind(AssertUnwindSafe(body)).ok().flatten().unwrap_or(fallback)
}

fn into_handle<T>(value: T) -> jlong {
    Box::into_raw(Box::new(value)) as jlong
}

/// 调用方保证 handle 来自同类型的 into_handle 且还没 destroy。
unsafe fn borrow_handle<'a, T>(handle: jlong) -> Option<&'a mut T> {
    (handle as *mut T).as_mut()
}

unsafe fn free_handle<T>(handle: jlong) {
    if handle != 0 {
        drop(Box::from_raw(handle as *mut T));
    }
}

struct Tuner {
    processor: TunerProcessor,
    input: Vec<f32>,
}

#[no_mangle]
pub extern "system" fn Java_app_naga_tuner_TunerNative_create(
    _env: JNIEnv,
    _this: JObject,
    sample_rate: jint,
    window_size: jint,
) -> jlong {
    guarded(0, || {
        let window_size = usize::try_from(window_size).ok()?;
        let processor = TunerProcessor::new(u32::try_from(sample_rate).ok()?, window_size);
        Some(into_handle(Tuner { processor, input: vec![0.0; window_size] }))
    })
}

/// 有读数时返回 true，并把 [弦下标, 音分, 是否变绿, 是否算调准] 写进 out 的前 4 格。
#[no_mangle]
pub extern "system" fn Java_app_naga_tuner_TunerNative_process(
    env: JNIEnv,
    _this: JObject,
    handle: jlong,
    samples: JFloatArray,
    n: jint,
    locked: jint,
    out: JFloatArray,
) -> jboolean {
    guarded(JNI_FALSE, || {
        let tuner = unsafe { borrow_handle::<Tuner>(handle) }?;
        let n = usize::try_from(n).ok().filter(|&n| n <= tuner.processor.window_size())?;
        let input = &mut tuner.input[..n];
        env.get_float_array_region(&samples, 0, input).ok()?;
        let locked = usize::try_from(locked).ok().filter(|&i| i < STRING_COUNT);
        let Some(r) = tuner.processor.process(input, locked) else {
            return Some(JNI_FALSE);
        };
        let result = [r.string_index as f32, r.cents, f32::from(u8::from(r.in_tune)), f32::from(u8::from(r.tuned))];
        env.set_float_array_region(&out, 0, &result).ok()?;
        Some(JNI_TRUE)
    })
}

#[no_mangle]
pub extern "system" fn Java_app_naga_tuner_TunerNative_destroy(_env: JNIEnv, _this: JObject, handle: jlong) {
    guarded((), || {
        unsafe { free_handle::<Tuner>(handle) };
        Some(())
    })
}

/// 一块里最多记这么多拍；块长远小于最快一拍（240 BPM）的间隔，实际每块最多 1 拍。
const MAX_BEATS_PER_BLOCK: usize = 8;

struct Metronome {
    scheduler: BeatScheduler,
    output: Vec<f32>,
}

#[no_mangle]
pub extern "system" fn Java_app_naga_metronome_BeatNative_create(
    _env: JNIEnv,
    _this: JObject,
    sample_rate: jint,
    max_frames: jint,
) -> jlong {
    guarded(0, || {
        let scheduler = BeatScheduler::new(u32::try_from(sample_rate).ok()?);
        Some(into_handle(Metronome { scheduler, output: vec![0.0; usize::try_from(max_frames).ok()?] }))
    })
}

/// 把 n 个采样写进 out；这一块里的每一拍按 [采样位置, 小节里第几拍] 依次写进 beats，
/// 返回拍数（beats 放不下的拍不写）。出错返回 -1。
#[no_mangle]
pub extern "system" fn Java_app_naga_metronome_BeatNative_render(
    env: JNIEnv,
    _this: JObject,
    handle: jlong,
    out: JFloatArray,
    n: jint,
    bpm: jdouble,
    beats_per_bar: jint,
    accent: jboolean,
    beats: JLongArray,
) -> jint {
    guarded(-1, || {
        let metronome = unsafe { borrow_handle::<Metronome>(handle) }?;
        let n = usize::try_from(n).ok().filter(|&n| n <= metronome.output.len())?;
        let room = (usize::try_from(env.get_array_length(&beats).ok()?).ok()? / 2).min(MAX_BEATS_PER_BLOCK);
        let mut events = [0 as jlong; 2 * MAX_BEATS_PER_BLOCK];
        let mut count = 0;

        let scheduler = &mut metronome.scheduler;
        scheduler.bpm = bpm;
        scheduler.beats_per_bar = u32::try_from(beats_per_bar).ok()?;
        scheduler.accent = accent != JNI_FALSE;
        let output = &mut metronome.output[..n];
        scheduler.render(output, |frame, index| {
            if count < room {
                events[2 * count] = frame;
                events[2 * count + 1] = jlong::from(index);
                count += 1;
            }
        });
        env.set_float_array_region(&out, 0, output).ok()?;
        env.set_long_array_region(&beats, 0, &events[..2 * count]).ok()?;
        Some(count as jint)
    })
}

#[no_mangle]
pub extern "system" fn Java_app_naga_metronome_BeatNative_destroy(_env: JNIEnv, _this: JObject, handle: jlong) {
    guarded((), || {
        unsafe { free_handle::<Metronome>(handle) };
        Some(())
    })
}
