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
        // catch(Throwable), not just Exception/UnsatisfiedLinkError: a failed
        // System.loadLibrary in the companion init leaves every external call
        // throwing UnsatisfiedLinkError at invocation time, and native bridge
        // failures must degrade to "voice unavailable" instead of crashing.
        return try {
            looseEndsTranscribeWav(wavPath, modelPath)
        } catch (t: Throwable) {
            android.util.Log.e("VoiceNativeBridge", "Whisper native call failed", t)
            null
        }
    }

    companion object {
        @Volatile
        private var libraryLoaded = false

        init {
            try {
                System.loadLibrary("loose_ends_voice")
                libraryLoaded = true
            } catch (e: UnsatisfiedLinkError) {
                android.util.Log.e(
                    "VoiceNativeBridge",
                    "Failed to load Whisper native library (libloose_ends_voice.so); voice remains unavailable",
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

        /** True when libloose_ends_voice.so was loaded successfully. */
        @JvmStatic
        fun isAvailable(): Boolean = libraryLoaded
    }
}
