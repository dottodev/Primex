package dev.primex.dt

import android.app.Application

/**
 * M0 application: offline-first. No sockets, no network stack, no background workers.
 * The native JNI library is loaded lazily by [dev.primex.dt.jni.CommandBus] on first use.
 */
class PrimeXApp : Application() {
    override fun onCreate() {
        super.onCreate()
    }
}
