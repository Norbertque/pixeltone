import java.util.Properties

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.compose.compiler)
    alias(libs.plugins.roborazzi)
}

// Release signing: CI passes the key in the environment; on claude-dev it lives in
// ~/.config/pixeltone/, outside the repo. Without either, release builds come out unsigned.
val localSigning = Properties().apply {
    val file = File(System.getProperty("user.home"), ".config/pixeltone/android-signing.properties")
    if (file.isFile) file.inputStream().use { load(it) }
}

android {
    namespace = "io.github.norbertque.pixeltone"
    compileSdk = 37

    defaultConfig {
        applicationId = "io.github.norbertque.pixeltone"
        minSdk = 26
        targetSdk = 37
        // CI numbers every build (its run number); the updater compares version codes.
        versionCode = (findProperty("versionCode") as String?)?.toInt() ?: 1
        versionName = (findProperty("versionName") as String?) ?: "0.1.0-dev"
    }

    signingConfigs {
        create("release") {
            val store = System.getenv("PIXELTONE_KEYSTORE") ?: localSigning.getProperty("storeFile")
            if (store != null) {
                storeFile = file(store)
                storePassword = System.getenv("PIXELTONE_KEYSTORE_PASSWORD")
                    ?: localSigning.getProperty("passwordFile")?.let { File(it).readText().trim() }
                keyAlias = System.getenv("PIXELTONE_KEY_ALIAS") ?: localSigning.getProperty("alias")
                keyPassword = storePassword
            }
        }
    }

    buildTypes {
        debug {
            // Development builds install beside a released Pixeltone and never update themselves.
            applicationIdSuffix = ".debug"
            buildConfigField("String", "UPDATE_FEED", "\"\"")
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.getByName("release").takeIf { it.storeFile != null }
            buildConfigField(
                "String",
                "UPDATE_FEED",
                "\"https://github.com/Norbertque/pixeltone/releases/download/android-dev/android-latest.json\"",
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    testOptions {
        // Robolectric screenshot tests (Roborazzi) render with the app's resources.
        unitTests.isIncludeAndroidResources = true
        // Robolectric's Android 16+ sets up shared memory through JDK internals.
        unitTests.all {
            it.jvmArgs("--add-exports=java.base/jdk.internal.access=ALL-UNNAMED", "--add-opens=java.base/java.io=ALL-UNNAMED")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.navigation.compose)
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.ui)
    implementation(libs.compose.ui.tooling.preview)
    implementation(libs.compose.material3)
    implementation(libs.compose.material.icons.extended)
    implementation(libs.media3.exoplayer)
    implementation(libs.media3.session)
    implementation(libs.coil.compose)
    implementation(libs.kotlinx.coroutines.android)
    debugImplementation(libs.compose.ui.tooling)
    debugImplementation(libs.compose.ui.test.manifest)

    testImplementation(libs.junit)
    testImplementation(platform(libs.compose.bom))
    testImplementation(libs.compose.ui.test.junit4)
    testImplementation(libs.robolectric)
    testImplementation(libs.roborazzi)
    testImplementation(libs.roborazzi.compose)
}
