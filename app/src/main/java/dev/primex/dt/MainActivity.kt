package dev.primex.dt

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.primex.dt.jni.CommandBus
import dev.primex.dt.ui.terminal.TerminalScreen
import dev.primex.dt.ui.theme.primeXColors
import top.yukonga.miuix.kmp.basic.Text
import top.yukonga.miuix.kmp.theme.MiuixTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            MiuixTheme(colors = primeXColors(isSystemInDarkTheme())) {
                PrimeXRoot()
            }
        }
    }
}

@Composable
private fun PrimeXRoot() {
    // Native library may not be present yet in M0; degrade gracefully.
    val apiVersion = remember { runCatching { CommandBus.apiVersion() }.getOrNull() }

    Column(modifier = Modifier.fillMaxSize().background(ShellBg)) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.statusBars)
                .padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(text = "PrimeX D.T — M0", fontWeight = FontWeight.SemiBold, fontSize = 18.sp)
        }

        TerminalScreen(modifier = Modifier.weight(1f))

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.navigationBars)
                .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "apiVersion: " + (apiVersion?.toString() ?: "unavailable"),
                color = StatusFg,
                fontSize = 12.sp,
            )
        }
    }
}

private val ShellBg = Color(0xFF101319)
private val StatusFg = Color(0xFF9AA4B2)
