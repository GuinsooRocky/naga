package app.naga

import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import app.naga.metronome.MetronomeEngine
import app.naga.metronome.MetronomeScreen
import app.naga.tuner.TunerScreen

object Palette {
    val Bg = Color(0xFF101214)
    val Surface = Color(0xFF1B1E22)
    val Text = Color(0xFFE8EAED)
    val Dim = Color(0xFF5A6068)
    val Green = Color(0xFF3DDC84)
    val Amber = Color(0xFFFFB74D)
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        MetronomeEngine.load(applicationContext)
        setContent { NagaApp() }
    }
}

@Composable
private fun NagaApp() {
    var tab by rememberSaveable { mutableIntStateOf(0) }
    val colors = darkColorScheme(
        primary = Palette.Green,
        onPrimary = Palette.Bg,
        background = Palette.Bg,
        surface = Palette.Surface,
        onBackground = Palette.Text,
        onSurface = Palette.Text,
    )
    MaterialTheme(colorScheme = colors) {
        Scaffold(
            containerColor = Palette.Bg,
            bottomBar = {
                NavigationBar(containerColor = Palette.Surface) {
                    NavigationBarItem(
                        selected = tab == 0,
                        onClick = { tab = 0 },
                        icon = { Icon(painterResource(R.drawable.ic_tuner), contentDescription = null) },
                        label = { Text("调音") },
                    )
                    NavigationBarItem(
                        selected = tab == 1,
                        onClick = { tab = 1 },
                        icon = { Icon(painterResource(R.drawable.ic_metronome), contentDescription = null) },
                        label = { Text("节拍") },
                    )
                }
            },
        ) { padding ->
            Box(Modifier.padding(padding).fillMaxSize()) {
                if (tab == 0) TunerScreen() else MetronomeScreen()
            }
        }
    }
}
