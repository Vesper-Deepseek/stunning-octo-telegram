import org.gradle.api.tasks.Copy
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application") version "9.0.1"
    kotlin("android") version "2.3.20"
}

android {
    namespace = "com.looseends.loose_ends"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
        isCoreLibraryDesugaringEnabled = true
    }

    defaultConfig {
        applicationId = "com.looseends.loose_ends"
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
        multiDexEnabled = true
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
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
            // Disable unit tests for release builds
            unitTestVariants.all {
                enabled = false
            }
        }
        debug {
            isMinifyEnabled = false
            isShrinkResources = false
            // Disable unit tests for debug builds
            unitTestVariants.all {
                enabled = false
            }
        }
    }

    // Completely disable all unit tests to avoid variant ambiguity
    testBuildType = "release"
    testOptions {
        unitTests {
            isIncludeAndroidResources = false
            all {
                it.enable = false
            }
        }
        animationsDisabled = true
    }

    // Disable androidTest to avoid configuration cache issues
    androidTests {
        enable = false
    }

    // Compress native .so files inside the APK to reduce universal download size.
    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

dependencies {
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.4")
    implementation("com.microsoft.onnxruntime:onnxruntime-android:1.21.1")
    implementation("org.opencv:opencv:4.10.0")
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
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
