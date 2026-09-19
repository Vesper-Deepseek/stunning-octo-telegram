package com.looseends.loose_ends

import android.Manifest
import android.app.Activity
import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject

class MainActivity : FlutterActivity() {
    private val channelName = "com.looseends/core"
    private val progressChannelName = "com.looseends/model_progress"
    private val modelPickRequestCode = 4242
    private val customModelPickRequestCode = 4244
    private val screenshotPickRequestCode = 4246
    private var pendingModelResult: MethodChannel.Result? = null
    private var pendingCustomModelResult: MethodChannel.Result? = null
    private var pendingScreenshotResult: MethodChannel.Result? = null
    private var progressSink: EventChannel.EventSink? = null
    private val offlineOcr by lazy { OfflineOcrEngine(applicationContext) }
    private val voiceRecorder by lazy { VoiceRecorder(cacheDir) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 4243)
        }
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        EventChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            progressChannelName
        ).setStreamHandler(object : EventChannel.StreamHandler {
            override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
                progressSink = events
            }

            override fun onCancel(arguments: Any?) {
                progressSink = null
            }
        })

        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            channelName
        ).setMethodCallHandler { call, result ->
            val bridge = NativeBridge.getInstance()
            when (call.method) {
                "init" -> {
                    result.success(bridge.init(applicationContext.filesDir.absolutePath))
                }
                "ingestText" -> {
                    val text = call.argument<String>("text") ?: ""
                    val today = java.time.LocalDate.now()
                    val model = ModelDownloadManager.selectedModelFile(this)
                    val json = bridge.extractText(
                        text,
                        if (model?.isFile == true) model.absolutePath else null,
                        call.argument<String>("sourceType") ?: "text",
                        today.year,
                        today.monthValue,
                        today.dayOfMonth
                    )
                    result.success(jsonArrayToList(json))
                }
                "pickModel" -> pickCustomModel(result)
                "modelStatus" -> result.success(ModelDownloadManager.status(this))
                "models" -> result.success(ModelDownloadManager.catalog())
                "startModelDownload" -> {
                    val modelId = call.argument<String>("modelId")
                        ?: return@setMethodCallHandler result.error("bad_args", "modelId required", null)
                    val allowMobile = call.argument<Boolean>("allowMobile") ?: false
                    ModelDownloadManager.startDownload(
                        context = this,
                        modelId = modelId,
                        allowMobile = allowMobile,
                        onProgress = { progressSink?.success(it) },
                        onFinished = { ok, message ->
                            runOnUiThread { result.success(mapOf("ok" to ok, "message" to message)) }
                        }
                    )
                }
                "cancelModelDownload" -> {
                    ModelDownloadManager.cancelDownload()
                    result.success(true)
                }
                "ocrModelStatus" -> result.success(OcrModelManager.status(this))
                "startOcrModelDownload" -> {
                    val allowMobile = call.argument<Boolean>("allowMobile") ?: false
                    OcrModelManager.downloadAll(
                        context = this,
                        allowMobile = allowMobile,
                        onProgress = { progressSink?.success(it) },
                        onFinished = { ok, message ->
                            runOnUiThread { result.success(mapOf("ok" to ok, "message" to message)) }
                        }
                    )
                }
                "deleteOcrModels" -> result.success(OcrModelManager.deleteAll(this))
                "pickScreenshot" -> pickScreenshot(result)
                "voiceModelStatus" -> result.success(VoiceModelManager.status(this))
                "startVoiceModelDownload" -> {
                    val allowMobile = call.argument<Boolean>("allowMobile") ?: false
                    VoiceModelManager.startDownload(
                        context = this,
                        allowMobile = allowMobile,
                        onProgress = { progressSink?.success(it) },
                        onFinished = { ok, message ->
                            runOnUiThread { result.success(mapOf("ok" to ok, "message" to message)) }
                        }
                    )
                }
                "cancelVoiceModelDownload" -> {
                    VoiceModelManager.cancelDownload()
                    result.success(true)
                }
                "deleteVoiceModel" -> result.success(VoiceModelManager.deleteModel(this))
                "startVoiceRecording" -> startVoiceRecording(result)
                "stopVoiceRecording" -> {
                    val file = voiceRecorder.stop()
                    result.success(file?.absolutePath)
                }
                "transcribeVoice" -> transcribeVoice(call.argument<String>("wavPath"), result)
                "deleteModel" -> {
                    val modelId = call.argument<String>("modelId")
                        ?: return@setMethodCallHandler result.error("bad_args", "modelId required", null)
                    result.success(ModelDownloadManager.deleteModel(this, modelId))
                }
                "selectModel" -> {
                    val modelId = call.argument<String>("modelId")
                        ?: return@setMethodCallHandler result.error("bad_args", "modelId required", null)
                    result.success(ModelDownloadManager.selectModel(this, modelId))
                }
                "pickCustomGguf" -> pickCustomModel(result)
                "shouldShowOnboarding" -> {
                    val prefs = getSharedPreferences("loose_ends_app", Context.MODE_PRIVATE)
                    result.success(!prefs.getBoolean("onboarding_complete", false))
                }
                "markOnboardingComplete" -> {
                    getSharedPreferences("loose_ends_app", Context.MODE_PRIVATE)
                        .edit()
                        .putBoolean("onboarding_complete", true)
                        .apply()
                    result.success(true)
                }
                "confirmDraft" -> {
                    val draftId = call.argument<Number>("draftId")?.toLong()
                        ?: return@setMethodCallHandler result.error("bad_args", "draftId required", null)
                    result.success(
                        bridge.confirmDraft(
                            draftId,
                            call.argument<String>("description"),
                            call.argument<String>("direction"),
                            call.argument<String>("expected_date"),
                            call.argument<String>("party")
                        )
                    )
                }
                "listOpen" -> {
                    val dir = call.argument<String>("direction") ?: "user_owes"
                    val today = java.time.LocalDate.now()
                    result.success(
                        jsonArrayToList(
                            bridge.listOpen(dir, today.year, today.monthValue, today.dayOfMonth)
                        )
                    )
                }
                "listDrafts" -> result.success(jsonArrayToList(bridge.listDrafts()))
                "createCommitment" -> {
                    val description = call.argument<String>("description") ?: ""
                    val direction = call.argument<String>("direction") ?: "unclear"
                    if (direction == "unclear") {
                        return@setMethodCallHandler result.error(
                            "bad_args",
                            "firm direction required",
                            null
                        )
                    }
                    result.success(
                        bridge.createCommitment(
                            description,
                            direction,
                            call.argument<String>("expected_date"),
                            call.argument<String>("party")
                        )
                    )
                }
                "resolveCommitment" -> {
                    val id = call.argument<Number>("id")?.toLong()
                        ?: return@setMethodCallHandler result.error("bad_args", "id required", null)
                    result.success(bridge.resolveCommitment(id, call.argument<String>("note")))
                    ModelDownloadManager.cancelReminder(this, id.toInt())
                }
                "snoozeCommitment" -> {
                    val id = call.argument<Number>("id")?.toLong()
                        ?: return@setMethodCallHandler result.error("bad_args", "id required", null)
                    result.success(bridge.snoozeCommitment(id))
                }
                "scheduleReminder" -> {
                    val id = call.argument<Number>("id")?.toInt()
                        ?: return@setMethodCallHandler result.error("bad_args", "id required", null)
                    val triggerAt = call.argument<Number>("triggerAtMillis")?.toLong()
                        ?: return@setMethodCallHandler result.error("bad_args", "triggerAtMillis required", null)
                    scheduleReminder(
                        id,
                        call.argument<String>("title") ?: "Loose Ends reminder",
                        call.argument<String>("text") ?: "A commitment needs your attention.",
                        triggerAt
                    )
                    result.success(true)
                }
                "cancelReminder" -> {
                    val id = call.argument<Number>("id")?.toInt()
                        ?: return@setMethodCallHandler result.error("bad_args", "id required", null)
                    cancelReminder(id)
                    result.success(true)
                }
                else -> result.notImplemented()
            }
        }
    }

    private fun startVoiceRecording(result: MethodChannel.Result) {
        if (android.os.Build.VERSION.SDK_INT >= 23 &&
            checkSelfPermission(Manifest.permission.RECORD_AUDIO) != android.content.pm.PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 4245)
            result.error("permission_required", "Microphone permission is required. Tap the record button again after granting it.", null)
            return
        }

        Thread {
            try {
                voiceRecorder.start()
                runOnUiThread { result.success(true) }
            } catch (e: Exception) {
                runOnUiThread { result.error("recording_failed", e.message, null) }
            }
        }.start()
    }

    private fun transcribeVoice(wavPath: String?, result: MethodChannel.Result) {
        if (wavPath.isNullOrBlank()) {
            result.error("bad_args", "wavPath required", null)
            return
        }
        val wav = java.io.File(wavPath)
        val model = VoiceModelManager.selectedModelFile(this)
        if (!wav.isFile) {
            result.error("recording_missing", "The recorded audio file is unavailable.", null)
            return
        }
        if (model == null) {
            wav.delete()
            result.error("voice_model_missing", "Download the Whisper voice model before recording.", null)
            return
        }

        Thread {
            try {
                val text = VoiceNative.transcribeWav(
                    wav.absolutePath,
                    model.absolutePath
                )
                wav.delete()
                if (text == null) {
                    runOnUiThread {
                        result.error("transcription_failed", "Offline Whisper transcription failed.", null)
                    }
                } else {
                    runOnUiThread { result.success(text) }
                }
            } catch (e: Exception) {
                wav.delete()
                runOnUiThread { result.error("transcription_failed", e.message, null) }
            }
        }.start()
    }

    override fun onDestroy() {
        voiceRecorder.cancel()
        offlineOcr.release()
        super.onDestroy()
    }

    private fun pickScreenshot(result: MethodChannel.Result) {
        if (pendingScreenshotResult != null) {
            result.error("busy", "image picker already open", null)
            return
        }
        if (!OcrModelManager.allReady(this)) {
            result.error(
                "ocr_models_missing",
                "Download and verify the OCR models before selecting a screenshot.",
                null,
            )
            return
        }
        pendingScreenshotResult = result
        startActivityForResult(
            Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "image/*"
            },
            screenshotPickRequestCode,
        )
    }

    private fun pickCustomModel(result: MethodChannel.Result) {
        if (pendingCustomModelResult != null) {
            result.error("busy", "model picker already open", null)
            return
        }
        pendingCustomModelResult = result
        startActivityForResult(
            Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
            },
            customModelPickRequestCode
        )
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        when (requestCode) {
            screenshotPickRequestCode -> {
                val result = pendingScreenshotResult
                pendingScreenshotResult = null
                if (result == null) return
                val uri = data?.data
                if (resultCode != Activity.RESULT_OK || uri == null) {
                    result.success(null)
                    return
                }
                Thread {
                    val image = java.io.File.createTempFile("loose-ends-ocr-", ".image", cacheDir)
                    try {
                        contentResolver.openInputStream(uri).use { input ->
                            if (input == null) throw IllegalStateException("Could not read the selected image.")
                            java.io.FileOutputStream(image).use { output -> input.copyTo(output, 1024 * 1024) }
                        }
                        val text = offlineOcr.recognize(image)
                        image.delete()
                        runOnUiThread { result.success(text) }
                    } catch (e: Exception) {
                        image.delete()
                        runOnUiThread { result.error("ocr_failed", e.message, null) }
                    }
                }.start()
            }
            customModelPickRequestCode -> {
                val result = pendingCustomModelResult
                pendingCustomModelResult = null
                if (result == null) return
                val uri = data?.data
                if (resultCode != Activity.RESULT_OK || uri == null) {
                    result.success(null)
                    return
                }
                Thread {
                    try {
                        val imported = ModelDownloadManager.importCustomModel(this, uri)
                        runOnUiThread { result.success(imported) }
                    } catch (e: Exception) {
                        runOnUiThread {
                            result.error("model_import_failed", e.message, null)
                        }
                    }
                }.start()
            }
            else -> super.onActivityResult(requestCode, resultCode, data)
        }
    }

    private fun scheduleReminder(id: Int, title: String, text: String, triggerAtMillis: Long) {
        val alarmManager = getSystemService(Context.ALARM_SERVICE) as AlarmManager
        val intent = Intent(this, ReminderReceiver::class.java).apply {
            putExtra("commitment_id", id)
            putExtra("title", title)
            putExtra("text", text)
        }
        val flags = PendingIntent.FLAG_UPDATE_CURRENT or
            (if (android.os.Build.VERSION.SDK_INT >= 23) PendingIntent.FLAG_IMMUTABLE else 0)
        val pending = PendingIntent.getBroadcast(this, id, intent, flags)
        alarmManager.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, triggerAtMillis, pending)
    }

    private fun cancelReminder(id: Int) {
        ModelDownloadManager.cancelReminder(this, id)
    }

    private fun jsonArrayToList(array: JSONArray?): List<Map<String, Any?>>? {
        if (array == null) return null
        return buildList {
            for (i in 0 until array.length()) {
                val obj = array.optJSONObject(i) ?: continue
                val map = mutableMapOf<String, Any?>()
                val keys = obj.keys()
                while (keys.hasNext()) {
                    val key = keys.next()
                    val value = obj.opt(key)
                    map[key] = if (value === JSONObject.NULL) null else value
                }
                add(map)
            }
        }
    }
}
