package com.looseends.loose_ends

/**
 * Dedicated Rust and Whisper JNI boundary.
 *
 * Whisper is loaded from a separate shared library because whisper.cpp and
 * llama.cpp each embed GGML/GGUF symbols. Separate libraries avoid duplicate
 * symbols while keeping voice inference inside Rust.
 */
class VoiceNativeBridge private constructor() {
    fun transcribeWav(wavPath: String, modelPath: String): String? {
        if (wavPath.isBlank() || modelPath.isBlank()) return null
        return try {
            looseEndsTranscribeWav(wavPath, modelPath)
        } catch (e: UnsatisfiedLinkError) {
            android.util.Log.e("VoiceNativeBridge", "Whisper native library unavailable", e)
            null
        }
    }

    companion object {
        init {
            try {
                System.loadLibrary("loose_ends_voice_native")
            } catch (e: UnsatisfiedLinkError) {
                android.util.Log.e(
                    "VoiceNativeBridge",
                    "Failed to load Whisper native library; voice remains unavailable",
                    e
                )
            }
        }

        @Volatile private var instance: VoiceNativeBridge? = null

        fun getInstance(): VoiceNativeBridge {
            return instance ?: synchronized(this) {
                instance ?: VoiceNativeBridge().also { instance = it }
            }
        }

        @JvmStatic private external fun looseEndsTranscribeWav(
            wavPath: String,
            modelPath: String
        ): String?
    }
}
