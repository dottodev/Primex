plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

// Release signing material is supplied by the operator through the environment.
// No keystore, password or signing config is stored in the repository.
val releaseKeystore = System.getenv("PDX_KEYSTORE")
val releaseStorePassword = System.getenv("PDX_STORE_PASSWORD")
val releaseKeyAlias = System.getenv("PDX_KEY_ALIAS") ?: "primex"
val releaseKeyPassword = System.getenv("PDX_KEY_PASSWORD") ?: releaseStorePassword

android {
    namespace = "dev.primex.dt"
    // Miuix 0.9.x AARs declare minCompileSdk=37 (verified from the published
    // AAR metadata). compileSdk is a build-time API level; runtime behavior is
    // still governed by targetSdk 35 as the spec requires.
    compileSdk {
        version = release(37) {
            minorApiLevel = 0
        }
    }

    defaultConfig {
        applicationId = "dev.primex.dt"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    // No externalNativeBuild block on purpose: the Rust cdylib (libpdx_jni.so) is built
    // separately by the Cargo side and the resulting .so files are copied into
    // app/src/main/jniLibs/<abi>/ later. CommandBus loads "pdx_jni" at runtime.

    signingConfigs {
        if (!releaseKeystore.isNullOrBlank() && !releaseStorePassword.isNullOrBlank()) {
            create("release") {
                storeFile = file(releaseKeystore!!)
                storePassword = releaseStorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            if (signingConfigs.findByName("release") != null) {
                signingConfig = signingConfigs.getByName("release")
            }
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        resources {
            excludes += setOf(
                "META-INF/AL2.0",
                "META-INF/LGPL2.1",
                "META-INF/LICENSE*",
                "META-INF/NOTICE*",
            )
        }
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)

    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.foundation)

    implementation(libs.miuix.ui)
}
