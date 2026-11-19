package dev.primex.dt.ui.theme

import androidx.compose.runtime.Composable
import top.yukonga.miuix.kmp.theme.Colors
import top.yukonga.miuix.kmp.theme.darkColorScheme
import top.yukonga.miuix.kmp.theme.lightColorScheme

/**
 * Miuix color scheme selection for PrimeX D.T.
 *
 * Verified against Miuix 0.9.4 source: MiuixTheme, lightColorScheme and darkColorScheme
 * live in package `top.yukonga.miuix.kmp.theme`. No Material 3 is used anywhere.
 */
@Composable
fun primeXColors(darkTheme: Boolean): Colors =
    if (darkTheme) darkColorScheme() else lightColorScheme()
