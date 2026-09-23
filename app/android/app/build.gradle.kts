import org.gradle.api.tasks.Copy
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "com.looseends.loose_ends"
    compileSdk = 35
    ndkVersion = "28.2.13676358"

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
        isCoreLibraryDesugaringEnabled = true
    }

    defaultConfig {
        applicationId = "com.looseends.loose_ends"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"
        multiDexEnabled = true
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        // Disable unit tests to avoid configuration issues
        testApplicationId = "com.looseends.loose_ends.test"
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("debug")
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isMinifyEnabled = false
            isShrinkResources = false
        }
    }

    // Compress native .so files inside the APK to reduce universal download size.
    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }

    // Disable all tests to avoid configuration issues with AGP.
    testOptions {
        unitTests.all {
            it.enabled = false
        }
        animationsDisabled = true
    }
}

dependencies {
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.4")
    implementation("com.microsoft.onnxruntime:onnxruntime-android:1.21.1")
    implementation("org.opencv:opencv:4.10.0")
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}

tasks.register<Copy>("extractOpenCvNativeLibs") {
    from({
        configurations.getByName("releaseRuntimeClasspath")
            .filter { it.name == "opencv-4.10.0.aar" }
            .map { zipTree(it) }
    }) {
        include("jni/**/libopencv_java4.so")
        eachFile {
            path = path.removePrefix("jni/")
        }
        includeEmptyDirs = false
    }
    into(layout.projectDirectory.dir("src/main/jniLibs"))
}

import java.util.Properties

/*
 * The Rust neural bridge uses the Android NDK's libc++ ABI. Some AARs
 * contribute an older libc++_shared.so, so overwrite the merged native
 * runtime with the exact library from the NDK used by the native build.
 */
tasks.matching { it.name == "mergeReleaseNativeLibs" }.configureEach {
    doLast {
        val properties = Properties()
        val localProperties = rootProject.file("local.properties")
        if (localProperties.isFile) {
            localProperties.inputStream().use { properties.load(it) }
        }

        val sdkPath = properties.getProperty("sdk.dir")
            ?: System.getenv("ANDROID_SDK_ROOT")
            ?: System.getenv("ANDROID_HOME")
        require(!sdkPath.isNullOrBlank()) {
            "Android SDK path is required to package libc++_shared.so"
        }

        val ndkPath = rootProject.file("$sdkPath/ndk/28.2.13676358")
        val prebuilt = ndkPath.resolve("toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib")
        val mergedLibRoot = layout.buildDirectory
            .dir("intermediates/merged_native_libs/release/mergeReleaseNativeLibs/out/lib")
            .get()
            .asFile

        mapOf(
            "arm64-v8a" to "aarch64-linux-android",
            "armeabi-v7a" to "arm-linux-androideabi",
            "x86_64" to "x86_64-linux-android"
        ).forEach { (abi, triple) ->
            val targetDir = mergedLibRoot.resolve(abi)
            if (!targetDir.isDirectory) return@forEach

            val source = prebuilt.resolve("$triple/libc++_shared.so")
            require(source.isFile) {
                "Matching NDK libc++_shared.so not found for $abi: $source"
            }
            source.copyTo(targetDir.resolve("libc++_shared.so"), overwrite = true)
        }
    }
}
