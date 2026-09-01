plugins {
    // AGP 9+ trae soporte de Kotlin integrado (built-in Kotlin) — no se aplica
    // "org.jetbrains.kotlin.android" aparte, generaba choque de extensión "kotlin".
    id("com.android.application") version "9.2.0" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.2.10" apply false
}
