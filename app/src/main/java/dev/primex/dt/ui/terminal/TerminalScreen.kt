package dev.primex.dt.ui.terminal

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.primex.dt.jni.CommandBus
import org.json.JSONObject
import top.yukonga.miuix.kmp.basic.Text
import top.yukonga.miuix.kmp.basic.TextButton

/**
 * M0 terminal panel: local echo always works; the native bridge is attempted but
 * a missing/failing bridge never blocks typing. No Material 3 imports.
 */
@Composable
fun TerminalScreen(modifier: Modifier = Modifier) {
    val lines = remember { mutableStateListOf<String>() }
    var input by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    val scrollState = rememberScrollState()

    LaunchedEffect(lines.size) {
        scrollState.animateScrollTo(scrollState.maxValue)
    }

    fun submit() {
        val typed = input
        if (typed.isEmpty()) return
        lines.add("$PROMPT $typed")
        input = ""
        val payload = JSONObject().put("data", typed).toString()
        val result = runCatching { CommandBus.send("terminal.write", payload) }
        error = result.exceptionOrNull()?.message
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(TerminalBg)
            .padding(horizontal = 16.dp, vertical = 12.dp),
    ) {
        Column(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .verticalScroll(scrollState),
        ) {
            Text(text = PROMPT, color = PromptFg, fontFamily = FontFamily.Monospace, fontSize = 13.sp)
            Text(
                text = "PTY bridge not yet connected — M0 scaffold",
                color = DimFg,
                fontFamily = FontFamily.Monospace,
                fontSize = 13.sp,
            )
            lines.forEach { line ->
                Text(text = line, color = TextFg, fontFamily = FontFamily.Monospace, fontSize = 13.sp)
            }
        }

        Spacer(Modifier.height(8.dp))

        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            BasicTextField(
                value = input,
                onValueChange = { input = it },
                modifier = Modifier
                    .weight(1f)
                    .background(InputBg, RoundedCornerShape(8.dp))
                    .padding(horizontal = 10.dp, vertical = 8.dp),
                textStyle = TextStyle(
                    color = TextFg,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 13.sp,
                ),
                singleLine = true,
                maxLines = 1,
            )
            TextButton(text = "send", onClick = { submit() })
        }

        error?.let { message ->
            Spacer(Modifier.height(4.dp))
            Text(
                text = "bridge error: $message",
                color = ErrorFg,
                fontFamily = FontFamily.Monospace,
                fontSize = 12.sp,
            )
        }
    }
}

private const val PROMPT = "sh -l (uid: app)"
private val TerminalBg = Color(0xFF090C12)
private val InputBg = Color(0xFF161C26)
private val TextFg = Color(0xFFD6E2F0)
private val PromptFg = Color(0xFF7EE787)
private val DimFg = Color(0xFF7A889B)
private val ErrorFg = Color(0xFFFF8A80)
