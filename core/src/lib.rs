//! naga 的算法核心：音高检测、读数稳定、变绿判定、弦表和音分、节拍排布和「哒」声。
//! 安卓那边只负责录音放音，每块采样经 JNI（见 ffi.rs）交给这里处理。

pub mod beat;
mod ffi;
pub mod pitch;
pub mod tuning;
