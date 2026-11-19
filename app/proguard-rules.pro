# JNI entry point: Rust exports Java_dev_primex_dt_jni_CommandBus_nativeSend.
# Keep the owning class and all native methods intact under R8.
-keep class dev.primex.dt.jni.CommandBus { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}

# Typed command errors are matched by simple name on the Kotlin side.
-keep class dev.primex.dt.jni.CommandBusException { *; }
-keep class dev.primex.dt.jni.CommandBusException$** { *; }
