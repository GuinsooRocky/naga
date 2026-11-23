package app.naga.metronome

import android.content.Context
import android.content.SharedPreferences
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTimestamp
import android.media.AudioTrack
import android.os.Process
import kotlinx.coroutines.flow.MutableStateFlow
import java.util.concurrent.ConcurrentLinkedQueue

enum class TimeSignature(val label: String, val beats: Int) {
    TWO_FOUR("2/4", 2),
    THREE_FOUR("3/4", 3),
    FOUR_FOUR("4/4", 4),
    SIX_EIGHT("6/8", 6),
}

/** 节拍排布和「哒」声在 Rust（core/src/beat.rs），这里只是 JNI 入口。 */
object BeatNative {
    init {
        System.loadLibrary("naga_core")
    }

    /** maxFrames 是之后每次 render 最多要的采样数。失败返回 0。 */
    external fun create(sampleRate: Int, maxFrames: Int): Long

    /**
     * 把 n 个采样写进 out；这一块里的每一拍按 [采样位置, 小节里第几拍] 依次写进 beats，
     * 返回拍数（beats 放不下的拍不写）。出错返回 -1。
     */
    external fun render(
        handle: Long, out: FloatArray, n: Int, bpm: Double, beatsPerBar: Int, accent: Boolean, beats: LongArray,
    ): Int

    external fun destroy(handle: Long)
}

object MetronomeEngine {
    const val MIN_BPM = 30
    const val MAX_BPM = 240
    private const val SAMPLE_RATE = 48000
    private const val BLOCK = 480

    val bpm = MutableStateFlow(100)
    val signature = MutableStateFlow(TimeSignature.FOUR_FOUR)
    val accent = MutableStateFlow(true)
    val playing = MutableStateFlow(false)
    val currentBeat = MutableStateFlow(-1)

    private var prefs: SharedPreferences? = null
    private var thread: Thread? = null

    @Volatile
    private var track: AudioTrack? = null
    private val beatQueue = ConcurrentLinkedQueue<LongArray>()
    private val timestamp = AudioTimestamp()

    fun load(context: Context) {
        if (prefs != null) return
        val p = context.getSharedPreferences("metronome", Context.MODE_PRIVATE)
        prefs = p
        bpm.value = p.getInt("bpm", 100).coerceIn(MIN_BPM, MAX_BPM)
        signature.value = TimeSignature.entries.getOrElse(p.getInt("signature", 2)) { TimeSignature.FOUR_FOUR }
        accent.value = p.getBoolean("accent", true)
    }

    fun setBpm(value: Int) {
        bpm.value = value.coerceIn(MIN_BPM, MAX_BPM)
        prefs?.edit()?.putInt("bpm", bpm.value)?.apply()
    }

    fun setSignature(value: TimeSignature) {
        signature.value = value
        prefs?.edit()?.putInt("signature", value.ordinal)?.apply()
    }

    fun setAccent(value: Boolean) {
        accent.value = value
        prefs?.edit()?.putBoolean("accent", value)?.apply()
    }

    fun start() {
        if (playing.value) return
        playing.value = true
        thread = Thread({ run() }, "metronome").also { it.start() }
    }

    fun stop() {
        if (!playing.value) return
        playing.value = false
        thread?.join(500)
        thread = null
        currentBeat.value = -1
    }

    /** 界面每帧调用：按扬声器实际播到的位置点亮对应的拍，画面和声音才对得上。 */
    fun syncVisual() {
        val t = track ?: return
        val played = runCatching {
            // 播放头只说明系统取走了多少采样，扬声器真正出声还要晚几十毫秒；时间戳给的才是出声位置
            if (t.getTimestamp(timestamp)) {
                timestamp.framePosition +
                    (System.nanoTime() - timestamp.nanoTime) * SAMPLE_RATE / 1_000_000_000L
            } else {
                t.playbackHeadPosition.toLong() and 0xFFFFFFFFL
            }
        }.getOrNull() ?: return
        while (true) {
            val next = beatQueue.peek() ?: break
            if (next[0] > played) break
            beatQueue.poll()
            currentBeat.value = next[1].toInt()
        }
    }

    private fun run() {
        Process.setThreadPriority(Process.THREAD_PRIORITY_AUDIO)
        val minBuf = AudioTrack.getMinBufferSize(
            SAMPLE_RATE, AudioFormat.CHANNEL_OUT_MONO, AudioFormat.ENCODING_PCM_FLOAT
        )
        val t = AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                    .build()
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setSampleRate(SAMPLE_RATE)
                    .setEncoding(AudioFormat.ENCODING_PCM_FLOAT)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .build()
            )
            .setBufferSizeInBytes(maxOf(minBuf, BLOCK * 4 * 4))
            .setTransferMode(AudioTrack.MODE_STREAM)
            .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
            .build()

        val handle = BeatNative.create(SAMPLE_RATE, BLOCK)
        val block = FloatArray(BLOCK)
        val beats = LongArray(2)
        beatQueue.clear()
        track = t
        t.play()
        try {
            while (playing.value) {
                val count = BeatNative.render(
                    handle, block, BLOCK, bpm.value.toDouble(), signature.value.beats, accent.value, beats
                )
                if (count > 0) beatQueue.add(longArrayOf(beats[0], beats[1]))
                t.write(block, 0, BLOCK, AudioTrack.WRITE_BLOCKING)
            }
        } finally {
            track = null
            runCatching { t.pause(); t.flush() }
            t.release()
            BeatNative.destroy(handle)
        }
    }
}
