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

/**
 * Downloads the Apache-2.0 PP-OCRv5 ONNX assets from the pinned release-assets
 * repository used by Photos for Proton. Every file is size/hash checked before use.
 */
object OcrModelManager {
    private data class Asset(
        val id: String,
        val fileName: String,
        val sizeBytes: Long,
        val sha256: String,
    )

    private const val BASE =
        "https://github.com/gitakoos/ocr-models/releases/latest/download/"

    private val assets = listOf(
        Asset(
            "det",
            "det.onnx",
            4_748_769L,
            "d7fe3ea74652890722c0f4d02458b7261d9f5ae6c92904d05707c9eb155c7924",
        ),
        Asset(
            "rec",
            "rec_latin.onnx",
            8_064_539L,
            "995b0f5f28d2073896a78c03b5b863eae6af3744bafa0245b8522beea6994927",
        ),
        Asset(
            "dict",
            "ppocrv5_latin_dict.txt",
            2_616L,
            "ccbcc45730b3fbbd9050c5bc74db6a99067141ef1035e3d14889a84a6b9b1aff",
        ),
    )

    fun status(context: Context): Map<String, Any?> {
        val values = assets.map { asset ->
            val file = File(modelDir(context), asset.fileName)
            mapOf(
                "id" to asset.id,
                "fileName" to asset.fileName,
                "sizeBytes" to asset.sizeBytes,
                "sha256" to asset.sha256,
                "ready" to isVerified(file, asset),
            )
        }
        return mapOf(
            "ready" to values.all { it["ready"] == true },
            "assets" to values,
        )
    }

    fun allReady(context: Context): Boolean = assets.all {
        isVerified(File(modelDir(context), it.fileName), it)
    }

    fun detFile(context: Context): File? =
        assetFile(context, "det")?.takeIf { isVerified(it, assets.first { a -> a.id == "det" }) }

    fun recFile(context: Context): File? =
        assetFile(context, "rec")?.takeIf { isVerified(it, assets.first { a -> a.id == "rec" }) }

    fun dictFile(context: Context): File? =
        assetFile(context, "dict")?.takeIf { isVerified(it, assets.first { a -> a.id == "dict" }) }

    fun downloadAll(
        context: Context,
        allowMobile: Boolean,
        onProgress: (Map<String, Any>) -> Unit,
        onFinished: (Boolean, String) -> Unit,
    ) {
        Thread {
            try {
                checkNetwork(context, allowMobile)
                val total = assets.sumOf { it.sizeBytes }
                val dir = modelDir(context)
                dir.mkdirs()
                val available = StatFs(dir.absolutePath).availableBytes
                if (available < total + 128L * 1024L * 1024L) {
                    throw OcrModelException("Not enough storage for OCR models.")
                }

                var completed = 0L
                for (asset in assets) {
                    downloadOne(context, asset) { downloaded, assetTotal, state ->
                        onProgress(
                            mapOf(
                                "assetId" to asset.id,
                                "state" to state,
                                "downloadedBytes" to downloaded,
                                "assetBytes" to assetTotal,
                                "overallBytes" to total,
                                "overallCompletedBytes" to completed + downloaded,
                                "percent" to (
                                    (completed + downloaded).toDouble() /
                                        total.toDouble() * 100.0
                                    ).coerceIn(0.0, 100.0),
                            ),
                        )
                    }
                    completed += asset.sizeBytes
                }
                onFinished(true, "OCR models downloaded and verified.")
            } catch (e: Exception) {
                onFinished(false, e.message ?: "OCR model download failed.")
            }
        }.start()
    }

    fun deleteAll(context: Context): Boolean {
        var ok = true
        assets.forEach { asset ->
            val file = File(modelDir(context), asset.fileName)
            if (file.exists() && !file.delete()) ok = false
            File(modelDir(context), asset.fileName + ".part").delete()
        }
        return ok
    }

    private fun downloadOne(
        context: Context,
        asset: Asset,
        onProgress: (Long, Long, String) -> Unit,
    ) {
        val dir = modelDir(context)
        dir.mkdirs()
        val target = File(dir, asset.fileName)
        if (isVerified(target, asset)) {
            onProgress(asset.sizeBytes, asset.sizeBytes, "ready")
            return
        }
        target.delete()
        val partial = File(dir, asset.fileName + ".part")
        partial.delete()

        var connection: HttpURLConnection? = null
        try {
            val url = BASE + asset.fileName
            if (!url.startsWith("https://github.com/gitakoos/ocr-models/", ignoreCase = true)) {
                throw OcrModelException("Blocked OCR model source.")
            }
            connection = URL(url).openConnection() as HttpURLConnection
            connection.connectTimeout = 20_000
            connection.readTimeout = 30_000
            connection.instanceFollowRedirects = true
            connection.requestMethod = "GET"
            val code = connection.responseCode
            if (code !in 200..299) {
                throw OcrModelException("OCR model download failed with HTTP $code.")
            }

            val total = connection.contentLengthLong.takeIf { it > 0 } ?: asset.sizeBytes
            var downloaded = 0L
            var lastReport = -1L
            BufferedInputStream(connection.inputStream, 1024 * 1024).use { input ->
                FileOutputStream(partial).use { output ->
                    val buffer = ByteArray(1024 * 1024)
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        output.write(buffer, 0, read)
                        downloaded += read
                        if (downloaded == total || downloaded - lastReport >= 512 * 1024) {
                            lastReport = downloaded
                            onProgress(downloaded, total, "downloading")
                        }
                    }
                    output.fd.sync()
                }
            }

            if (partial.length() != asset.sizeBytes ||
                !sha256(partial).equals(asset.sha256, ignoreCase = true)
            ) {
                partial.delete()
                throw OcrModelException(
                    "OCR model ${asset.fileName} failed size/hash verification; partial file deleted.",
                )
            }
            onProgress(asset.sizeBytes, asset.sizeBytes, "verified")
            if (!partial.renameTo(target)) {
                partial.delete()
                throw OcrModelException("Could not store verified OCR asset.")
            }
        } finally {
            connection?.disconnect()
        }
    }

    private fun assetFile(context: Context, id: String): File? =
        assets.firstOrNull { it.id == id }?.let { File(modelDir(context), it.fileName) }

    private fun isVerified(file: File, asset: Asset): Boolean =
        file.isFile &&
            file.length() == asset.sizeBytes &&
            sha256(file).equals(asset.sha256, ignoreCase = true)

    private fun checkNetwork(context: Context, allowMobile: Boolean) {
        val manager = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network = manager.activeNetwork
            ?: throw OcrModelException("No internet connection is available.")
        val caps = manager.getNetworkCapabilities(network)
            ?: throw OcrModelException("Could not determine the current network.")
        if (caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) return
        if (caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) && allowMobile) return
        if (caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)) {
            throw OcrModelException("Wi-Fi is required by default for OCR model download.")
        }
        throw OcrModelException("Connect over Wi-Fi or explicitly allow mobile data.")
    }

    private fun modelDir(context: Context): File =
        File(context.filesDir, "ocr-models")

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

    private class OcrModelException(message: String) : Exception(message)
}
