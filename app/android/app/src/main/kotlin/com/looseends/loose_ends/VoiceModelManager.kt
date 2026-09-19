package com.looseends.loose_ends

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.StatFs
import java.io.BufferedInputStream
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Small official Whisper model manager.
 *
 * The model is the MIT-licensed whisper.cpp tiny.en Q5_1 model. The model stays
 * in the app-private files directory and is never bundled into the APK.
 */
object VoiceModelManager {
    private const val MODEL_ID = "whisper_tiny_en_q5_1"
    private const val MODEL_NAME = "Whisper tiny.en Q5_1"
    private const val FILE_NAME = "ggml-tiny.en-q5_1.bin"
    private const val SIZE_BYTES = 32_166_155L
    private const val SHA256 =
        "c77c5766f1cef09b6b7d47f21b546cbddd4157886b3b5d6d4f709e91e66c7c2b"
    private const val URL =
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en-q5_1.bin?download=true"
    private const val PREFS = "loose_ends_voice_model"
    private const val SELECTED = "selected_model"
    private const val EXTRA_HEADROOM = 64L * 1024L * 1024L

    private val cancelled = AtomicBoolean(false)
    @Volatile private var active = false

    fun status(context: Context): Map<String, Any?> {
        val file = modelFile(context)
        val downloaded = file.isFile &&
            file.length() == SIZE_BYTES &&
            sha256(file).equals(SHA256, ignoreCase = true)
        val selected = prefs(context).getString(SELECTED, null) == MODEL_ID
        return mapOf(
            "id" to MODEL_ID,
            "name" to MODEL_NAME,
            "fileName" to FILE_NAME,
            "sizeBytes" to SIZE_BYTES,
            "sha256" to SHA256,
            "downloaded" to downloaded,
            "selected" to selected,
            "ready" to (downloaded && selected),
            "activeDownload" to active
        )
    }

    fun selectedModelFile(context: Context): File? {
        val selected = prefs(context).getString(SELECTED, null)
        if (selected != MODEL_ID) return null
        return modelFile(context).takeIf { it.isFile }
    }

    fun startDownload(
        context: Context,
        allowMobile: Boolean,
        onProgress: (Map<String, Any>) -> Unit,
        onFinished: (Boolean, String) -> Unit
    ) {
        synchronized(this) {
            if (active) {
                onFinished(false, "A voice model download is already running.")
                return
            }
            active = true
            cancelled.set(false)
        }

        Thread {
            var connection: HttpURLConnection? = null
            var partial: File? = null
            try {
                requireHttps(URL)
                checkNetwork(context, allowMobile)

                val dir = modelDir(context)
                dir.mkdirs()
                val available = StatFs(dir.absolutePath).availableBytes
                val required = SIZE_BYTES + EXTRA_HEADROOM
                if (available < required) {
                    throw VoiceModelException(
                        "Not enough storage. Need at least ${formatBytes(required)} free."
                    )
                }

                val target = File(dir, FILE_NAME)
                partial = File(dir, "$FILE_NAME.part")

                if (target.isFile && target.length() == SIZE_BYTES) {
                    if (sha256(target).equals(SHA256, ignoreCase = true)) {
                        prefs(context).edit().putString(SELECTED, MODEL_ID).apply()
                        onProgress(progress(SIZE_BYTES, SIZE_BYTES, "ready"))
                        onFinished(true, "Whisper model is already downloaded and verified.")
                        return@Thread
                    }
                    target.delete()
                }

                partial.delete()
                connection = URL(URL).openConnection() as HttpURLConnection
                connection.connectTimeout = 20_000
                connection.readTimeout = 30_000
                connection.instanceFollowRedirects = true
                connection.requestMethod = "GET"
                connection.setRequestProperty("Accept", "application/octet-stream")

                val code = connection.responseCode
                if (code !in 200..299) {
                    throw VoiceModelException("Voice model download failed with HTTP $code.")
                }

                val total = connection.contentLengthLong.takeIf { it > 0 } ?: SIZE_BYTES
                var downloaded = 0L
                var lastReport = -1L

                BufferedInputStream(connection.inputStream, 1024 * 1024).use { input ->
                    FileOutputStream(partial).use { output ->
                        val buffer = ByteArray(1024 * 1024)
                        while (true) {
                            if (cancelled.get()) throw VoiceModelCancelledException()
                            val read = input.read(buffer)
                            if (read < 0) break
                            output.write(buffer, 0, read)
                            downloaded += read
                            if (downloaded == total || downloaded - lastReport >= 512 * 1024) {
                                lastReport = downloaded
                                onProgress(progress(downloaded, total, "downloading"))
                            }
                        }
                        output.fd.sync()
                    }
                }

                if (cancelled.get()) throw VoiceModelCancelledException()

                if (partial.length() != SIZE_BYTES) {
                    partial.delete()
                    throw VoiceModelException("Downloaded voice model size does not match the published size.")
                }

                onProgress(progress(partial.length(), SIZE_BYTES, "verifying"))
                val actualHash = sha256(partial)
                if (!actualHash.equals(SHA256, ignoreCase = true)) {
                    partial.delete()
                    throw VoiceModelException("Voice model SHA-256 verification failed. Partial file was deleted.")
                }

                if (target.exists()) target.delete()
                if (!partial.renameTo(target)) {
                    throw VoiceModelException("Could not move the verified voice model into local storage.")
                }

                prefs(context).edit().putString(SELECTED, MODEL_ID).apply()
                onProgress(progress(SIZE_BYTES, SIZE_BYTES, "ready"))
                onFinished(true, "Downloaded and SHA-256 verified $MODEL_NAME.")
            } catch (e: VoiceModelCancelledException) {
                partial?.delete()
                onFinished(false, "Voice model download cancelled. Partial file deleted.")
            } catch (e: Exception) {
                partial?.delete()
                onFinished(false, e.message ?: "Voice model download failed.")
            } finally {
                connection?.disconnect()
                synchronized(this) {
                    active = false
                    cancelled.set(false)
                }
            }
        }.start()
    }

    fun cancelDownload() {
        cancelled.set(true)
    }

    fun deleteModel(context: Context): Boolean {
        val file = modelFile(context)
        val deleted = !file.exists() || file.delete()
        if (deleted) prefs(context).edit().remove(SELECTED).apply()
        return deleted
    }

    private fun checkNetwork(context: Context, allowMobile: Boolean) {
        val manager = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network = manager.activeNetwork
            ?: throw VoiceModelException("No internet connection is available.")
        val caps = manager.getNetworkCapabilities(network)
            ?: throw VoiceModelException("Could not determine the current network.")
        val wifi = caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)
        val mobile = caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)
        if (wifi) return
        if (mobile && allowMobile) return
        if (mobile) {
            throw VoiceModelException(
                "Wi-Fi is required by default. Enable the mobile-data override to continue."
            )
        }
        throw VoiceModelException("Connect over Wi-Fi or explicitly allow mobile data.")
    }

    private fun requireHttps(url: String) {
        if (!url.startsWith("https://huggingface.co/ggerganov/whisper.cpp/", ignoreCase = true)) {
            throw VoiceModelException("Blocked: only the fixed official Whisper model source is allowed.")
        }
    }

    private fun progress(downloaded: Long, total: Long, state: String): Map<String, Any> = mapOf(
        "modelId" to MODEL_ID,
        "state" to state,
        "downloadedBytes" to downloaded,
        "totalBytes" to total,
        "percent" to if (total > 0) {
            (downloaded.toDouble() / total.toDouble() * 100.0).coerceIn(0.0, 100.0)
        } else 0.0
    )

    private fun modelDir(context: Context): File =
        File(context.filesDir, "models")

    private fun modelFile(context: Context): File =
        File(modelDir(context), FILE_NAME)

    private fun prefs(context: Context) =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private fun sha256(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        FileInputStream(file).use { input ->
            val buffer = ByteArray(1024 * 1024)
            while (true) {
                val n = input.read(buffer)
                if (n < 0) break
                digest.update(buffer, 0, n)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }

    private fun formatBytes(bytes: Long): String {
        val mb = bytes / (1024.0 * 1024.0)
        return if (mb >= 1024.0) "%.2f GB".format(mb / 1024.0) else "%.0f MB".format(mb)
    }

    private class VoiceModelException(message: String) : Exception(message)
    private class VoiceModelCancelledException : Exception()
}
