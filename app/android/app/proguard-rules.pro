# ProGuard/R8 rules for Loose Ends Android app

# Keep native library entry points (JNI methods)
-keepclassmembers class com.looseends.loose_ends.NativeBridge {
    private static native <methods>;
}

-keepclassmembers class com.looseends.loose_ends.VoiceNative {
    private static native <methods>;
}

-keepclassmembers class com.looseends.loose_ends.VoiceNativeBridge {
    private static native <methods>;
}

# Keep NativeBridge singleton instance
-keep class com.looseends.loose_ends.NativeBridge {
    public static com.looseends.loose_ends.NativeBridge getInstance();
    private static volatile com.looseends.loose_ends.NativeBridge instance;
}

# Keep VoiceNative object
-keep class com.looseends.loose_ends.VoiceNative {
    public static void transcribeWav(java.lang.String, java.lang.String);
}

# Keep VoiceNativeBridge singleton
-keep class com.looseends.loose_ends.VoiceNativeBridge {
    public static com.looseends.loose_ends.VoiceNativeBridge getInstance();
    private static volatile com.looseends.loose_ends.VoiceNativeBridge instance;
}

# Keep Kotlin metadata for reflection
-keepattributes *Annotation*
-keepattributes RuntimeVisibleAnnotations

# Keep data classes used for JSON serialization
-keepclassmembers class * {
    @com.google.gson.annotations.SerializedName <fields>;
}

# Keep ONNX Runtime classes
-keep class com.microsoft.onnxruntime.** { *; }
-dontwarn com.microsoft.onnxruntime.**

# Keep OpenCV classes
-keep class org.opencv.** { *; }
-dontwarn org.opencv.**

# Keep SQLite/Rusqlite native bindings (if any reflection is used)
-keep class * implements org.sqlite.** { *; }

# Keep Flutter embedding classes
-keep class io.flutter.** { *; }
-dontwarn io.flutter.**

# Keep generated plugin registrant
-keep class io.flutter.plugins.GeneratedPluginRegistrant { *; }

# Keep ReminderReceiver
-keep class com.looseends.loose_ends.ReminderReceiver { *; }

# Keep model managers
-keep class com.looseends.loose_ends.*ModelManager { *; }
-keep class com.looseends.loose_ends.VoiceRecorder { *; }
-keep class com.looseends.loose_ends.OfflineOcrEngine { *; }

# Preserve native method names
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep enum fields
-keepclassmembers enum * {
    public static **[] values();
    public static ** valueOf(java.lang.String);
}

# Keep parcelable implementations
-keep class * implements android.os.Parcelable {
    public static final android.os.Parcelable$Creator *;
}
