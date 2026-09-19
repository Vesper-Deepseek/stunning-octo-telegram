package com.looseends.loose_ends

/**
 * Isolated native Whisper bridge.
 *
 * Whisper is kept in a separate native library because whispercpp vendors its
 * own ggml while llama-cpp-2 also vendors ggml. Keeping them in separate
 * shared objects avoids duplicate ggml/gguf symbols in the Android linker.
 */
object VoiceNative {
    init {
        try {
            System.loadLibrary("loose_ends_voice")
        } catch (e: UnsatisfiedLinkError) {
            android.util.Log.e(
                "VoiceNative",
                "Whisper native library unavailable; voice remains disabled until installed",
                e
            )
        }
    }

    fun transcribeWav(wavPath: String, modelPath: String): String? {
        if (wavPath.isBlank() || modelPath.isBlank()) return null
        return try {
            looseEndsTranscribeWav(wavPath, modelPath)
        } catch (e: UnsatisfiedLinkError) {
            android.util.Log.e("VoiceNative", "Whisper native method unavailable", e)
            null
        }
    }

    @JvmStatic
    private external fun looseEndsTranscribeWav(
        wavPath: String,
        modelPath: String
    ): String?
}
