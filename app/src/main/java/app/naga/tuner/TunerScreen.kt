package app.naga.tuner

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LifecycleResumeEffect
import app.naga.Palette
import kotlin.math.roundToInt

@Composable
fun TunerScreen() {
    val context = LocalContext.current
    fun micGranted() = ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO) ==
        PackageManager.PERMISSION_GRANTED
    var granted by remember { mutableStateOf(micGranted()) }
    // 拒过两次或勾了「不再询问」后系统不再弹框，只能去系统设置里开
    var blocked by remember { mutableStateOf(false) }
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
        granted = it
        val activity = context as? Activity
        val canAskAgain = activity != null &&
            ActivityCompat.shouldShowRequestPermissionRationale(activity, Manifest.permission.RECORD_AUDIO)
        blocked = !it && !canAskAgain
    }
    LaunchedEffect(Unit) {
        if (!granted) launcher.launch(Manifest.permission.RECORD_AUDIO)
    }
    // 在系统设置里打开权限再回来时不会走授权回调，只能回到前台时自己再查一次
    LifecycleResumeEffect(Unit) {
        granted = micGranted()
        onPauseOrDispose { }
    }

    if (!granted) {
        Column(
            Modifier.fillMaxSize().padding(32.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text("需要用麦克风听琴声才能调音", color = Palette.Text, fontSize = 18.sp)
            Spacer(Modifier.height(20.dp))
            Button(onClick = {
                if (blocked) {
                    context.startActivity(
                        Intent(
                            Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                            Uri.fromParts("package", context.packageName, null),
                        )
                    )
                } else {
                    launcher.launch(Manifest.permission.RECORD_AUDIO)
                }
            }) {
                Text(if (blocked) "去系统设置打开麦克风" else "允许使用麦克风")
            }
        }
        return
    }

    val engine = remember { TunerEngine() }
    LifecycleResumeEffect(engine) {
        engine.start()
        onPauseOrDispose { engine.stop() }
    }
    val state by engine.state.collectAsState()

    val color = when {
        !state.active -> Palette.Dim
        state.inTune -> Palette.Green
        else -> Palette.Amber
    }
    // 没声音时保留上一次的弦名（变灰），避免界面闪空
    val shown = state.locked ?: state.stringIndex

    Column(
        Modifier.fillMaxSize().padding(horizontal = 24.dp, vertical = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                text = if (shown >= 0) STRING_NAMES[shown] else "–",
                color = color,
                fontSize = 120.sp,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = if (shown >= 0) "${6 - shown} 弦" else "拨一根弦",
                color = Palette.Dim,
                fontSize = 18.sp,
            )
        }

        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text("准", color = Palette.Text, fontSize = 13.sp)
            Meter(cents = state.cents, active = state.active, color = color)
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text("偏低", color = Palette.Dim, fontSize = 13.sp)
                Text(
                    text = if (state.active) "%+d".format(state.cents.roundToInt()) else "",
                    color = color,
                    fontSize = 13.sp,
                )
                Text("偏高", color = Palette.Dim, fontSize = 13.sp)
            }
            Spacer(Modifier.height(20.dp))
            Text(
                text = when {
                    !state.active -> if (state.locked != null) "已锁定这根弦，再点一次取消" else "拨一根弦"
                    state.inTune -> "准了"
                    state.cents < 0 -> "拧紧一点"
                    else -> "放松一点"
                },
                color = color,
                fontSize = 22.sp,
                fontWeight = FontWeight.Medium,
            )
        }

        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            STRING_NAMES.forEachIndexed { index, name ->
                StringDot(
                    name = name,
                    number = 6 - index,
                    current = state.active && state.stringIndex == index,
                    locked = state.locked == index,
                    tuned = index in state.tuned,
                    onClick = { engine.toggleLock(index) },
                )
            }
        }
    }
}

@Composable
private fun Meter(cents: Float, active: Boolean, color: Color) {
    val position by animateFloatAsState(
        targetValue = if (active) cents.coerceIn(-50f, 50f) else 0f,
        animationSpec = tween(120),
        label = "needle",
    )
    Canvas(Modifier.fillMaxWidth().height(130.dp)) {
        val centerX = size.width / 2
        val half = size.width / 2 - 8.dp.toPx()
        val base = size.height - 10.dp.toPx()
        for (i in -10..10) {
            val x = centerX + half * i / 10f
            val h = when {
                i == 0 -> 44.dp
                i % 5 == 0 -> 28.dp
                else -> 16.dp
            }.toPx()
            drawLine(
                color = if (i == 0) Palette.Text else Palette.Dim,
                start = Offset(x, base),
                end = Offset(x, base - h),
                strokeWidth = 2.dp.toPx(),
                cap = StrokeCap.Round,
            )
        }
        val x = centerX + half * position / 50f
        drawLine(
            color = color,
            start = Offset(x, base),
            end = Offset(x, 6.dp.toPx()),
            strokeWidth = 6.dp.toPx(),
            cap = StrokeCap.Round,
        )
    }
}

@Composable
private fun StringDot(
    name: String,
    number: Int,
    current: Boolean,
    locked: Boolean,
    tuned: Boolean,
    onClick: () -> Unit,
) {
    val ring = when {
        locked -> Palette.Text
        current -> Palette.Amber
        tuned -> Palette.Green
        else -> Palette.Dim
    }
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Box(
            Modifier
                .size(46.dp)
                .clip(CircleShape)
                .background(if (current || locked) Palette.Surface else Color.Transparent)
                .border(if (locked) 3.dp else 2.dp, ring, CircleShape)
                .clickable(onClick = onClick),
            contentAlignment = Alignment.Center,
        ) {
            Text(name, color = if (tuned) Palette.Green else Palette.Text, fontSize = 18.sp)
        }
        Spacer(Modifier.height(6.dp))
        Box(Modifier.height(18.dp), contentAlignment = Alignment.Center) {
            if (tuned) {
                Canvas(Modifier.size(14.dp)) {
                    val path = Path().apply {
                        moveTo(size.width * 0.1f, size.height * 0.55f)
                        lineTo(size.width * 0.4f, size.height * 0.85f)
                        lineTo(size.width * 0.9f, size.height * 0.2f)
                    }
                    drawPath(
                        path, Palette.Green,
                        style = Stroke(2.dp.toPx(), cap = StrokeCap.Round, join = StrokeJoin.Round),
                    )
                }
            } else {
                Text("$number", color = Palette.Dim, fontSize = 12.sp)
            }
        }
    }
}
