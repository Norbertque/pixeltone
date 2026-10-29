plugins {
    alias(libs.plugins.android.library)
}

// The USB Audio Class driver behind exclusive USB output: descriptor parsing and format
// negotiation in Kotlin, the isochronous stream in C++ over libusb (docs/android-usb.md).
android {
    namespace = "io.github.norbertque.pixeltone.usbaudio"
    compileSdk = 37
    // r28 LTS: 16 KB page alignment by default, required for targetSdk 37 devices.
    ndkVersion = "28.2.13676358"

    defaultConfig {
        minSdk = 26

        ndk {
            // Phones and tablets, and x86_64 Chromebooks (which take USB DACs too).
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)
    }
}

dependencies {
    implementation(libs.kotlinx.coroutines.android)

    testImplementation(libs.junit)
    testImplementation(libs.truth)
}
