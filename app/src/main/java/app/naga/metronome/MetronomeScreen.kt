package app.naga.metronome

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.SystemClock
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import app.naga.Palette
import kotlin.math.roundToInt

@Composable
fun MetronomeScreen() {
    val context = LocalContext.current
    val bpm by MetronomeEngine.bpm.collectAsState()
    val signature by MetronomeEngine.signature.collectAsState()
    val accent by MetronomeEngine.accent.collectAsState()
    val playing by MetronomeEngine.playing.collectAsState()
    val beat by MetronomeEngine.currentBeat.collectAsState()

    // 通知权限只影响通知栏显不显示，拒绝了也照样开始
    val askNotification = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
        MetronomeService.start(context)
    }
    LaunchedEffect(playing) {
        while (playing) {
            withFrameNanos { }
            MetronomeEngine.syncVisual()
        }
    }
    val taps = remember { ArrayDeque<Long>() }

    Column(
        Modifier.fillMaxSize().padding(horizontal = 24.dp, vertical = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            repeat(signature.beats) { i ->
                val color = when {
                    i != beat -> Palette.Dim
                    i == 0 -> Palette.Amber
                    else -> Palette.Green
                }
                Box(Modifier.size(22.dp).clip(CircleShape).background(color))
            }
        }

        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text("$bpm", color = Palette.Text, fontSize = 110.sp, fontWeight = FontWeight.Bold)
            Text("BPM", color = Palette.Dim, fontSize = 16.sp)
            Spacer(Modifier.height(16.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                RoundButton("−") { MetronomeEngine.setBpm(bpm - 1) }
                Slider(
                    value = bpm.toFloat(),
                    onValueChange = { MetronomeEngine.setBpm(it.roundToInt()) },
                    valueRange = MetronomeEngine.MIN_BPM.toFloat()..MetronomeEngine.MAX_BPM.toFloat(),
                    modifier = Modifier.weight(1f).padding(horizontal = 12.dp),
                )
                RoundButton("+") { MetronomeEngine.setBpm(bpm + 1) }
            }
        }

        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                SideButton(signature.label, "拍号") {
                    val all = TimeSignature.entries
                    MetronomeEngine.setSignature(all[(signature.ordinal + 1) % all.size])
                }
                Box(
                    Modifier
                        .size(96.dp)
                        .clip(CircleShape)
                        .background(Palette.Green)
                        .clickable {
                            when {
                                playing -> MetronomeService.stop(context)
                                Build.VERSION.SDK_INT >= 33 && ContextCompat.checkSelfPermission(
                                    context, Manifest.permission.POST_NOTIFICATIONS
                                ) != PackageManager.PERMISSION_GRANTED ->
                                    askNotification.launch(Manifest.permission.POST_NOTIFICATIONS)
                                else -> MetronomeService.start(context)
                            }
                        },
                    contentAlignment = Alignment.Center,
                ) {
                    Canvas(Modifier.size(32.dp)) {
                        if (playing) {
                            drawRect(Palette.Bg, Offset(size.width * 0.15f, size.height * 0.15f),
                                Size(size.width * 0.7f, size.height * 0.7f))
                        } else {
                            drawPath(
                                Path().apply {
                                    moveTo(size.width * 0.25f, size.height * 0.1f)
                                    lineTo(size.width * 0.95f, size.height * 0.5f)
                                    lineTo(size.width * 0.25f, size.height * 0.9f)
                                    close()
                                },
                                Palette.Bg,
                            )
                        }
                    }
                }
                SideButton("敲击", "测速") {
                    val now = SystemClock.elapsedRealtime()
                    // 30 BPM 时两下就隔 2 秒，断开的判定要留出余量
                    if (taps.isNotEmpty() && now - taps.last() > 3000) taps.clear()
                    taps.addLast(now)
                    if (taps.size > 5) taps.removeFirst()
                    if (taps.size >= 2) {
                        val average = (taps.last() - taps.first()).toDouble() / (taps.size - 1)
                        MetronomeEngine.setBpm((60000.0 / average).roundToInt())
                    }
                }
            }
            Spacer(Modifier.height(20.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("第一拍重音", color = Palette.Text, fontSize = 16.sp)
                Spacer(Modifier.width(12.dp))
                Switch(checked = accent, onCheckedChange = { MetronomeEngine.setAccent(it) })
            }
        }
    }
}

@Composable
private fun RoundButton(label: String, onClick: () -> Unit) {
    Box(
        Modifier.size(48.dp).clip(CircleShape).background(Palette.Surface).clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, color = Palette.Text, fontSize = 26.sp)
    }
}

@Composable
private fun SideButton(title: String, caption: String, onClick: () -> Unit) {
    Column(
        Modifier.size(84.dp).clip(CircleShape).background(Palette.Surface).clickable(onClick = onClick),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(title, color = Palette.Text, fontSize = 20.sp, fontWeight = FontWeight.Medium)
        Text(caption, color = Palette.Dim, fontSize = 12.sp)
    }
}
