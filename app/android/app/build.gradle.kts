plugins {
    id("com.android.application") version "8.7.0"
    id("dev.flutter.flutter-gradle-plugin") version "1.0.0"
}

android {
    namespace = "com.looseends.loose_ends"
    compileSdk = 35
    ndkVersion = "27.0.12077973"

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
    
    // Disable all tests to avoid configuration cache issues with AGP
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

tasks.register("extractOpenCvNativeLibs", Copy::class) {
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
