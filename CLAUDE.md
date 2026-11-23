# naga

目标：一个只给 owner 自己用的安卓 app，能给吉他调音、能打节拍，没有会员、广告和登录。

需求以 `docs/PRD.md` 为准。

## 技术分工

- **Rust（`core/`）**：所有决定准不准、稳不稳的算法都放这里，包括音高检测、读数稳定、变绿判定、弦表和音分计算、节拍排布、「哒」声合成。算法测试用 `cargo test --release`。
- **Kotlin（`app/`）**：只做安卓系统相关的部分，包括界面（Jetpack Compose）、权限、录音放音（AudioRecord / AudioTrack）、前台服务、通知、保存设置。Kotlin 把每块音频经 JNI 交给 Rust 处理。
- 新加的逻辑能放 Rust 就放 Rust；只有必须调安卓系统接口的才写 Kotlin。
- 录音放音留在 Kotlin，因为安卓 8 上 AAudio 有兼容问题，而最低版本要保持安卓 8（minSdk 26）。

## 构建

- `./gradlew assembleDebug`：自动编 Rust（只编 arm64-v8a）并打包，安装包在 `app/build/outputs/apk/debug/`。
- 每次打完包，把安装包移动（不是复制）到 `~/Desktop/naga.apk`，覆盖旧版。
- 本机没有 Android Studio，也没有模拟器，全部用命令行。最终验收靠 owner 在真机上拨弦试。
