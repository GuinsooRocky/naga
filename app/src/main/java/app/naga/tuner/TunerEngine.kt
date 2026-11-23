package app.naga.tuner

import android.annotation.SuppressLint
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.update

/** 只给界面显示；顺序和 Rust 弦表一致，下标 0 是 6 弦。 */
val STRING_NAMES = listOf("E", "A", "D", "G", "B", "E")

/** 调音算法在 Rust（core/src/tuning.rs），这里只是 JNI 入口。 */
object TunerNative {
    init {
        System.loadLibrary("naga_core")
    }

    /** 失败返回 0。 */
    external fun create(sampleRate: Int, windowSize: Int): Long

    /**
     * samples 的前 n 个是新读到的采样；locked 为 -1 表示自动识弦。
     * 有读数时返回 true，并把 [弦下标, 音分, 是否变绿, 是否算调准] 写进 out 的前 4 格。
     */
    external fun process(handle: Long, samples: FloatArray, n: Int, locked: Int, out: FloatArray): Boolean

    external fun destroy(handle: Long)
}

data class TunerState(
    val active: Boolean = false,
    val stringIndex: Int = -1,
    val cents: Float = 0f,
    val inTune: Boolean = false,
    val tuned: Set<Int> = emptySet(),
    val locked: Int? = null,
)

class TunerEngine {
    val state = MutableStateFlow(TunerState())

    @Volatile
    private var running = false
    private var thread: Thread? = null

    fun toggleLock(index: Int) {
        state.update { it.copy(locked = if (it.locked == index) null else index) }
    }

    fun start() {
        if (running) return
        running = true
        thread = Thread({ loop() }, "tuner").also { it.start() }
    }

    fun stop() {
        running = false
        thread?.join(500)
        thread = null
        state.update { it.copy(active = false, inTune = false) }
    }

    @SuppressLint("MissingPermission")
    private fun loop() {
        val format = AudioFormat.Builder()
            .setSampleRate(SAMPLE_RATE)
            .setEncoding(AudioFormat.ENCODING_PCM_FLOAT)
            .setChannelMask(AudioFormat.CHANNEL_IN_MONO)
            .build()
        val minBuf = AudioRecord.getMinBufferSize(
            SAMPLE_RATE, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_FLOAT
        )
        // 这个音源按规范不加降噪和自动增益，灵敏度也有统一约定，音量门槛才在各台手机上都管用。
        // UNPROCESSED 虽然也不加处理，但约定灵敏度低约 18 dB，用同一套门槛会听不到琴声
        val record = runCatching {
            AudioRecord.Builder()
                .setAudioSource(MediaRecorder.AudioSource.VOICE_RECOGNITION)
                .setAudioFormat(format)
                .setBufferSizeInBytes(maxOf(minBuf, WINDOW * 4 * 2))
                .build()
        }.getOrNull()?.takeIf { it.state == AudioRecord.STATE_INITIALIZED } ?: return

        val handle = TunerNative.create(SAMPLE_RATE, WINDOW)
        val hop = FloatArray(HOP)
        val out = FloatArray(4)

        record.startRecording()
        try {
            while (running) {
                val n = record.read(hop, 0, HOP, AudioRecord.READ_BLOCKING)
                if (n <= 0) break
                val heard = TunerNative.process(handle, hop, n, state.value.locked ?: -1, out)
                state.update {
                    if (!heard) {
                        it.copy(active = false, inTune = false)
                    } else {
                        val index = out[0].toInt()
                        it.copy(
                            active = true,
                            stringIndex = index,
                            cents = out[1],
                            inTune = out[2] != 0f,
                            tuned = if (out[3] != 0f) it.tuned + index else it.tuned,
                        )
                    }
                }
            }
        } finally {
            record.stop()
            record.release()
            TunerNative.destroy(handle)
        }
    }

    private companion object {
        const val SAMPLE_RATE = 44100
        const val WINDOW = 4096
        const val HOP = 2048
    }
}
