package com.looseends.loose_ends

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.Uri
import android.os.StatFs
import java.io.BufferedInputStream
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.atomic.AtomicBoolean

object ModelDownloadManager {
    data class ModelSpec(
        val id: String,
        val name: String,
        val fileName: String,
        val sizeBytes: Long,
        val sha256: String,
        val url: String,
        val description: String
    )

    private val catalog = listOf(
        ModelSpec(
            id = "qwen_1_5b_q4_k_m",
            name = "Qwen 2.5 1.5B Instruct",
            fileName = "qwen2.5-1.5b-instruct-q4_k_m.gguf",
            sizeBytes = 1_117_320_736L,
            sha256 = "6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e",
            url = "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf?download=true",
            description = "Lighter and faster. Recommended for quick everyday extraction on mid-range phones."
        ),
        ModelSpec(
            id = "minicpm5_2b_q4_k_m",
            name = "MiniCPM5 2B",
            fileName = "MiniCPM5-2B-Q4_K_M.gguf",
            sizeBytes = 1_561_318_368L,
            sha256 = "ec2d5801640099e97d8d7e8003ad4d81f336e757811f03a26173dddf386602fd",
            url = "https://huggingface.co/openbmb/MiniCPM5-2B-GGUF/resolve/main/MiniCPM5-2B-Q4_K_M.gguf?download=true",
            description = "Larger and more capable for complex or multi-step commitment extraction."
        )
    )

    private const val PREFS = "loose_ends_models"
    private const val SELECTED_MODEL = "selected_model"
    private const val CUSTOM_ID = "custom_import"
    private const val EXTRA_HEADROOM = 128L * 1024L * 1024L

    private val cancelled = AtomicBoolean(false)
    @Volatile private var active = false

    fun catalog(): List<Map<String, Any>> = catalog.map {
        mapOf(
            "id" to it.id,
            "name" to it.name,
            "fileName" to it.fileName,
            "sizeBytes" to it.sizeBytes,
            "sha256" to it.sha256,
            "description" to it.description
        )
    }

    fun status(context: Context): Map<String, Any?> {
        val root = modelDir(context)
        val prefs = prefs(context)
        val selected = prefs.getString(SELECTED_MODEL, null)
        val models = catalog.map { spec ->
            val file = File(root, spec.fileName)
            val downloaded = file.isFile && file.length() == spec.sizeBytes &&
                sha256(file).equals(spec.sha256, ignoreCase = true)
            mapOf(
                "id" to spec.id,
                "name" to spec.name,
                "fileName" to spec.fileName,
                "sizeBytes" to spec.sizeBytes,
                "sha256" to spec.sha256,
                "description" to spec.description,
                "downloaded" to downloaded,
                "ready" to downloaded && selected == spec.id,
                "selected" to selected == spec.id
            )
        }
        val custom = File(root, "custom-import.gguf")
        return mapOf(
            "models" to models,
            "selectedModelId" to selected,
            "custom" to if (custom.isFile) mapOf(
                "id" to CUSTOM_ID,
                "name" to custom.name,
                "sizeBytes" to custom.length(),
                "sha256" to sha256(custom),
                "ready" to selected == CUSTOM_ID,
                "selected" to selected == CUSTOM_ID,
                "verified" to false
            ) else null,
            "activeDownload" to active
        )
    }

    fun selectedModelFile(context: Context): File? {
        val selected = prefs(context).getString(SELECTED_MODEL, null) ?: return null
        return when (selected) {
            CUSTOM_ID -> File(modelDir(context), "custom-import.gguf").takeIf { it.isFile }
            else -> catalog.firstOrNull { it.id == selected }?.let {
                File(modelDir(context), it.fileName)
            }?.takeIf { it.isFile }
        }
    }

    fun startDownload(
        context: Context,
        modelId: String,
        allowMobile: Boolean,
        onProgress: (Map<String, Any>) -> Unit,
        onFinished: (Boolean, String) -> Unit
    ) {
        synchronized(this) {
            if (active) {
                onFinished(false, "A model download is already running.")
                return
            }
            active = true
            cancelled.set(false)
        }

        Thread {
            var connection: HttpURLConnection? = null
            var partial: File? = null
            try {
                val spec = catalog.firstOrNull { it.id == modelId }
                    ?: throw ModelDownloadException("Unknown model.")
                requireHttps(spec.url)
                checkNetwork(context, allowMobile)

                val dir = modelDir(context)
                dir.mkdirs()
                val stat = StatFs(dir.absolutePath)
                val required = spec.sizeBytes + EXTRA_HEADROOM
                if (stat.availableBytes < required) {
                    throw ModelDownloadException(
                        "Not enough storage. Need at least \${formatBytes(required)} free."
                    )
                }

                val target = File(dir, spec.fileName)
                partial = File(dir, spec.fileName + ".part")

                if (target.isFile && target.length() == spec.sizeBytes) {
                    if (sha256(target).equals(spec.sha256, ignoreCase = true)) {
                        prefs(context).edit().putString(SELECTED_MODEL, spec.id).apply()
                        onProgress(progress(spec, spec.sizeBytes, spec.sizeBytes, "ready"))
                        onFinished(true, "Model is already downloaded and verified.")
                        return@Thread
                    }
                    target.delete()
                }

                partial.delete()

                connection = URL(spec.url).openConnection() as HttpURLConnection
                connection.connectTimeout = 20_000
                connection.readTimeout = 30_000
                connection.instanceFollowRedirects = true
                connection.requestMethod = "GET"
                connection.setRequestProperty("Accept", "application/octet-stream")

                val code = connection.responseCode
                if (code !in 200..299) {
                    throw ModelDownloadException("Model download failed with HTTP $code.")
                }

                val total = connection.contentLengthLong.takeIf { it > 0 } ?: spec.sizeBytes
                var downloaded = 0L
                var lastReport = -1L

                BufferedInputStream(connection.inputStream, 1024 * 1024).use { input ->
                    FileOutputStream(partial).use { output ->
                        val buffer = ByteArray(1024 * 1024)
                        while (true) {
                            if (cancelled.get()) throw ModelCancelledException()
                            val read = input.read(buffer)
                            if (read < 0) break
                            output.write(buffer, 0, read)
                            downloaded += read
                            if (downloaded == total ||
                                downloaded - lastReport >= 512 * 1024
                            ) {
                                lastReport = downloaded
                                onProgress(progress(spec, downloaded, total, "downloading"))
                            }
                        }
                        output.fd.sync()
                    }
                }

                if (cancelled.get()) throw ModelCancelledException()

                if (partial.length() != spec.sizeBytes) {
                    partial.delete()
                    throw ModelDownloadException(
                        "Downloaded size does not match the published model size."
                    )
                }

                onProgress(progress(spec, partial.length(), spec.sizeBytes, "verifying"))
                val actualHash = sha256(partial)
                if (!actualHash.equals(spec.sha256, ignoreCase = true)) {
                    partial.delete()
                    throw ModelDownloadException(
                        "Hash verification failed. The partial file was deleted."
                    )
                }

                if (target.exists()) target.delete()
                if (!partial.renameTo(target)) {
                    throw ModelDownloadException(
                        "Could not move the verified model into local storage."
                    )
                }

                prefs(context).edit().putString(SELECTED_MODEL, spec.id).apply()
                onProgress(progress(spec, spec.sizeBytes, spec.sizeBytes, "ready"))
                onFinished(true, "Downloaded and SHA-256 verified \${spec.name}.")
            } catch (e: ModelCancelledException) {
                partial?.delete()
                onFinished(false, "Download cancelled. Partial file deleted.")
            } catch (e: Exception) {
                partial?.delete()
                onFinished(false, e.message ?: "Model download failed.")
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

    fun selectModel(context: Context, modelId: String): Boolean {
        if (modelId == CUSTOM_ID) {
            val file = File(modelDir(context), "custom-import.gguf")
            return file.isFile && prefs(context).edit().putString(SELECTED_MODEL, modelId).commit()
        }

        val spec = catalog.firstOrNull { it.id == modelId } ?: return false
        val file = File(modelDir(context), spec.fileName)
        if (!file.isFile || file.length() != spec.sizeBytes) return false
        if (!sha256(file).equals(spec.sha256, ignoreCase = true)) {
            file.delete()
            return false
        }
        return prefs(context).edit().putString(SELECTED_MODEL, spec.id).commit()
    }

    fun deleteModel(context: Context, modelId: String): Boolean {
        val file = when (modelId) {
            CUSTOM_ID -> File(modelDir(context), "custom-import.gguf")
            else -> catalog.firstOrNull { it.id == modelId }?.let {
                File(modelDir(context), it.fileName)
            }
        } ?: return false

        val deleted = !file.exists() || file.delete()
        if (deleted && prefs(context).getString(SELECTED_MODEL, null) == modelId) {
            prefs(context).edit().remove(SELECTED_MODEL).apply()
        }
        return deleted
    }

    @Throws(Exception::class)
    fun importCustomModel(context: Context, uri: Uri): Map<String, Any?> {
        val dir = modelDir(context)
        dir.mkdirs()
        val destination = File(dir, "custom-import.gguf")
        val partial = File(dir, "custom-import.gguf.part")
        partial.delete()

        context.contentResolver.openInputStream(uri).use { input ->
            if (input == null) throw ModelDownloadException("Could not read the selected file.")
            BufferedInputStream(input, 1024 * 1024).use { source ->
                FileOutputStream(partial).use { output ->
                    val header = ByteArray(4)
                    var headerRead = 0
                    while (headerRead < 4) {
                        val n = source.read(header, headerRead, 4 - headerRead)
                        if (n < 0) break
                        headerRead += n
                    }
                    if (headerRead != 4 ||
                        String(header, Charsets.US_ASCII) != "GGUF"
                    ) {
                        throw ModelDownloadException("Selected file is not a GGUF model.")
                    }
                    output.write(header)
                    source.copyTo(output, 1024 * 1024)
                    output.fd.sync()
                }
            }
        }

        if (partial.length() < 5) {
            partial.delete()
            throw ModelDownloadException("Selected GGUF file is empty or incomplete.")
        }

        val hash = sha256(partial)
        val matched = catalog.firstOrNull { it.sha256.equals(hash, ignoreCase = true) }
        if (matched != null) {
            val target = File(dir, matched.fileName)
            if (target.exists()) target.delete()
            if (!partial.renameTo(target)) {
                throw ModelDownloadException("Could not store imported model.")
            }
            prefs(context).edit().putString(SELECTED_MODEL, matched.id).apply()
            return mapOf(
                "id" to matched.id,
                "name" to matched.name,
                "sha256" to hash,
                "verified" to true
            )
        }

        if (destination.exists()) destination.delete()
        if (!partial.renameTo(destination)) {
            throw ModelDownloadException("Could not store imported model.")
        }
        prefs(context).edit().putString(SELECTED_MODEL, CUSTOM_ID).apply()
        return mapOf(
            "id" to CUSTOM_ID,
            "name" to destination.name,
            "sha256" to hash,
            "verified" to false
        )
    }

    fun cancelReminder(context: Context, id: Int) {
        val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as android.app.AlarmManager
        val intent = android.content.Intent(context, ReminderReceiver::class.java)
        val flags = android.app.PendingIntent.FLAG_UPDATE_CURRENT or
            (if (android.os.Build.VERSION.SDK_INT >= 23) android.app.PendingIntent.FLAG_IMMUTABLE else 0)
        val pending = android.app.PendingIntent.getBroadcast(context, id, intent, flags)
        alarmManager.cancel(pending)
        pending.cancel()
    }

    private fun checkNetwork(context: Context, allowMobile: Boolean) {
        val manager = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network = manager.activeNetwork
            ?: throw ModelDownloadException("No internet connection is available.")
        val caps = manager.getNetworkCapabilities(network)
            ?: throw ModelDownloadException("Could not determine the current network.")
        val wifi = caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)
        val mobile = caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)
        if (wifi) return
        if (mobile && allowMobile) return
        if (mobile) {
            throw ModelDownloadException(
                "Wi-Fi is required by default. Enable the mobile-data override to continue."
            )
        }
        throw ModelDownloadException("Connect over Wi-Fi or explicitly allow mobile data.")
    }

    private fun requireHttps(url: String) {
        if (!url.startsWith("https://huggingface.co/", ignoreCase = true)) {
            throw ModelDownloadException(
                "Blocked: only the fixed official Hugging Face model source is allowed."
            )
        }
    }

    private fun progress(
        spec: ModelSpec,
        downloaded: Long,
        total: Long,
        state: String
    ): Map<String, Any> = mapOf(
        "modelId" to spec.id,
        "state" to state,
        "downloadedBytes" to downloaded,
        "totalBytes" to total,
        "percent" to if (total > 0) {
            (downloaded.toDouble() / total.toDouble() * 100.0).coerceIn(0.0, 100.0)
        } else 0.0
    )

    private fun modelDir(context: Context): File =
        File(context.filesDir, "models")

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
        val gb = bytes / (1024.0 * 1024.0 * 1024.0)
        if (gb >= 1.0) return "%.2f GB".format(gb)
        return "%.0f MB".format(bytes / (1024.0 * 1024.0))
    }

    private class ModelDownloadException(message: String) : Exception(message)
    private class ModelCancelledException : Exception()
}
