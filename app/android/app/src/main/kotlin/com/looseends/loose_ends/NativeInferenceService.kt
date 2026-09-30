package com.looseends.loose_ends

import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.os.IBinder
import android.os.ResultReceiver
import android.util.Log
import java.io.File

/**
 * Runs Whisper/OCR in a dedicated app process.
 *
 * Native C/C++ failures such as SIGABRT, SIGSEGV and abort() cannot be caught
 * by Kotlin try/catch. Keeping the native runtime in :inference means a
 * broken native model/runtime can kill only this worker process; the Flutter
 * UI process remains alive and receives a timeout/error instead.
 */
class NativeInferenceService : Service() {
    companion object {
        const val ACTION_RUN = "com.looseends.RUN_NATIVE_INFERENCE"
        const val EXTRA_KIND = "kind"
        const val EXTRA_PATH = "path"
        const val EXTRA_MODEL = "model"
        const val EXTRA_RECEIVER = "receiver"

        const val KIND_VOICE = "voice"
        const val KIND_OCR = "ocr"

        const val RESULT_OK = 0
        const val RESULT_ERROR = 1
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent == null) {
            stopSelf(startId)
            return START_NOT_STICKY
        }

        val kind = intent.getStringExtra(EXTRA_KIND)
        val path = intent.getStringExtra(EXTRA_PATH)
        val model = intent.getStringExtra(EXTRA_MODEL)
        val receiver = receiverFrom(intent)

        if (kind.isNullOrBlank() || path.isNullOrBlank() || receiver == null) {
            stopSelf(startId)
            return START_NOT_STICKY
        }

        Thread({
            try {
                when (kind) {
                    KIND_VOICE -> runVoice(path, model, receiver)
                    KIND_OCR -> runOcr(path, receiver)
                    else -> sendError(receiver, "Unknown native inference request.")
                }
            } catch (t: Throwable) {
                Log.e("NativeInferenceService", "Native inference worker failed", t)
                sendError(receiver, t.message ?: "Native inference failed.")
            } finally {
                stopSelf(startId)
            }
        }, "loose-ends-native-inference").apply {
            isDaemon = true
            start()
        }

        return START_NOT_STICKY
    }

    private fun runVoice(
        wavPath: String,
        modelPath: String?,
        receiver: ResultReceiver,
    ) {
        if (modelPath.isNullOrBlank()) {
            sendError(receiver, "Whisper model is unavailable.")
            return
        }

        if (!VoiceNativeBridge.isAvailable()) {
            sendError(receiver, "Whisper native library is unavailable in this build.")
            return
        }

        val text = VoiceNativeBridge.getInstance().transcribeWav(wavPath, modelPath)
        if (text == null) {
            sendError(receiver, "Offline Whisper transcription failed.")
        } else {
            receiver.send(RESULT_OK, Bundle().apply { putString("text", text) })
        }
    }

    private fun runOcr(imagePath: String, receiver: ResultReceiver) {
        val image = File(imagePath)
        if (!image.isFile) {
            sendError(receiver, "OCR image is unavailable.")
            return
        }

        val text = OfflineOcrEngine(applicationContext).recognize(image)
        receiver.send(RESULT_OK, Bundle().apply { putString("text", text) })
    }

    private fun sendError(receiver: ResultReceiver, message: String) {
        receiver.send(RESULT_ERROR, Bundle().apply { putString("message", message) })
    }

    @Suppress("DEPRECATION")
    private fun receiverFrom(intent: Intent): ResultReceiver? =
        if (Build.VERSION.SDK_INT >= 33) {
            intent.getParcelableExtra(EXTRA_RECEIVER, ResultReceiver::class.java)
        } else {
            intent.getParcelableExtra(EXTRA_RECEIVER)
        }
}
