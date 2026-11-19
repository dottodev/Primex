package dev.primex.dt.jni

import org.json.JSONObject

/**
 * Single entry point to the native command bus.
 *
 * The Rust side exports: Java_dev_primex_dt_jni_CommandBus_nativeSend
 */
object CommandBus {
    init {
        System.loadLibrary("pdx_jni")
    }

    private external fun nativeSend(command: String, payload: String): String

    /** Returns the raw JSON string from native; the caller decodes it. */
    fun send(command: String, payload: String): String = nativeSend(command, payload)

    /** Convenience probe for the status line. Returns 0 when native is absent or JSON is bad. */
    fun apiVersion(): Int = try {
        val obj = JSONObject(send("api.version", "{}"))
        obj.optInt("version", obj.optInt("apiVersion", 0))
    } catch (t: Throwable) {
        0
    }
}

/** Typed error families forwarded by the native layer (kept simple in M0). */
sealed class CommandBusException(message: String) : RuntimeException(message) {
    class BadInput(message: String) : CommandBusException(message)
    class Unsupported(message: String) : CommandBusException(message)
    class Verification(message: String) : CommandBusException(message)
    class Capability(message: String) : CommandBusException(message)
    class Cancelled(message: String) : CommandBusException(message)
    class Internal(message: String) : CommandBusException(message)
}
