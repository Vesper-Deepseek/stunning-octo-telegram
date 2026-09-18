package com.looseends.loose_ends

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import android.app.Activity
import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import java.io.File
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject

class MainActivity : FlutterActivity() {
    private val channelName = "com.looseends/core"
    private val modelPickRequestCode = 4242
    private var pendingModelResult: MethodChannel.Result? = null

    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            requestPermissions(
                arrayOf(android.Manifest.permission.POST_NOTIFICATIONS),
                4243
            )
        }
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName)
            .setMethodCallHandler { call, result ->
                val bridge = NativeBridge.getInstance()
                when (call.method) {
                    "init" -> {
                        result.success(bridge.init(applicationContext.filesDir.absolutePath))
                    }
                    "ingestText" -> {
                        val text = call.argument<String>("text") ?: ""
                        val today = java.time.LocalDate.now()
                        val model = modelFile()
                        val json = bridge.extractText(
                            text,
                            if (model.isFile) model.absolutePath else null,
                            today.year, today.monthValue, today.dayOfMonth
                        )
                        result.success(jsonArrayToList(json))
                    }
                    "pickModel" -> {
                        if (pendingModelResult != null) {
                            return@setMethodCallHandler result.error("busy", "model picker already open", null)
                        }
                        pendingModelResult = result
                        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                            addCategory(Intent.CATEGORY_OPENABLE)
                            type = "*/*"
                        }
                        startActivityForResult(intent, modelPickRequestCode)
                    }
                    "modelStatus" -> {
                        val file = modelFile()
                        result.success(
                            mapOf(
                                "configured" to file.isFile,
                                "path" to if (file.isFile) file.absolutePath else null,
                                "name" to if (file.isFile) file.name else null,
                                "sizeBytes" to if (file.isFile) file.length() else 0L
                            )
                        )
                    }
                    "confirmDraft" -> {
                        val draftId = (call.argument<Number>("draftId") as? Number)?.toLong()
                            ?: return@setMethodCallHandler result.error("bad_args", "draftId required", null)
                        val desc = call.argument<String>("description")
                        val dir = call.argument<String>("direction")
                        val date = call.argument<String>("expected_date")
                        val party = call.argument<String>("party")
                        val id = bridge.confirmDraft(draftId, desc, dir, date, party)
                        result.success(id)
                    }
                    "listOpen" -> {
                        val dir = call.argument<String>("direction") ?: "user_owes"
                        val today = java.time.LocalDate.now()
                        val json = bridge.listOpen(
                            dir,
                            today.year, today.monthValue, today.dayOfMonth
                        )
                        result.success(jsonArrayToList(json))
                    }
                    "listDrafts" -> {
                        result.success(jsonArrayToList(bridge.listDrafts()))
                    }
                    "createCommitment" -> {
                        val description = call.argument<String>("description") ?: ""
                        val direction = call.argument<String>("direction") ?: "unclear"
                        if (direction == "unclear") {
                            return@setMethodCallHandler result.error("bad_args", "firm direction required", null)
                        }
                        val id = bridge.createCommitment(
                            description,
                            direction,
                            call.argument<String>("expected_date"),
                            call.argument<String>("party")
                        )
                        result.success(id)
                    }
                    "resolveCommitment" -> {
                        val id = (call.argument<Number>("id"))?.toLong()
                            ?: return@setMethodCallHandler result.error("bad_args", "id required", null)
                        result.success(bridge.resolveCommitment(id, call.argument<String>("note")))
                    }
                    "snoozeCommitment" -> {
                        val id = (call.argument<Number>("id"))?.toLong()
                            ?: return@setMethodCallHandler result.error("bad_args", "id required", null)
                        result.success(bridge.snoozeCommitment(id))
                    }
                    "scheduleReminder" -> {
                        val id = (call.argument<Number>("id"))?.toInt()
                            ?: return@setMethodCallHandler result.error("bad_args", "id required", null)
                        val triggerAt = (call.argument<Number>("triggerAtMillis"))?.toLong()
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
                        val id = (call.argument<Number>("id"))?.toInt()
                            ?: return@setMethodCallHandler result.error("bad_args", "id required", null)
                        cancelReminder(id)
                        result.success(true)
                    }
                    else -> result.notImplemented()
                }
            }
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


    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode != modelPickRequestCode) {
            super.onActivityResult(requestCode, resultCode, data)
            return
        }

        val result = pendingModelResult
        pendingModelResult = null
        if (result == null) return

        val uri = data?.data
        if (resultCode != Activity.RESULT_OK || uri == null) {
            result.success(null)
            return
        }

        Thread {
            try {
                val destination = modelFile()
                destination.parentFile?.mkdirs()
                contentResolver.openInputStream(uri)?.use { input ->
                    destination.outputStream().use { output ->
                        input.copyTo(output, 1024 * 1024)
                    }
                } ?: throw IllegalStateException("Cannot read selected model")
                runOnUiThread { result.success(destination.absolutePath) }
            } catch (e: Exception) {
                runOnUiThread {
                    result.error("model_import_failed", e.message, null)
                }
            }
        }.start()
    }

    private fun modelFile(): File =
        File(filesDir, "models/qwen2.5-1.5b-instruct-q4_k_m.gguf")


    private fun scheduleReminder(
        id: Int,
        title: String,
        text: String,
        triggerAtMillis: Long
    ) {
        val alarmManager = getSystemService(Context.ALARM_SERVICE) as AlarmManager
        val intent = Intent(this, ReminderReceiver::class.java).apply {
            putExtra("commitment_id", id)
            putExtra("title", title)
            putExtra("text", text)
        }
        val flags = PendingIntent.FLAG_UPDATE_CURRENT or
            (if (android.os.Build.VERSION.SDK_INT >= 23) PendingIntent.FLAG_IMMUTABLE else 0)
        val pending = PendingIntent.getBroadcast(this, id, intent, flags)
        alarmManager.setAndAllowWhileIdle(
            AlarmManager.RTC_WAKEUP,
            triggerAtMillis,
            pending
        )
    }

    private fun cancelReminder(id: Int) {
        val alarmManager = getSystemService(Context.ALARM_SERVICE) as AlarmManager
        val intent = Intent(this, ReminderReceiver::class.java)
        val flags = PendingIntent.FLAG_UPDATE_CURRENT or
            (if (android.os.Build.VERSION.SDK_INT >= 23) PendingIntent.FLAG_IMMUTABLE else 0)
        val pending = PendingIntent.getBroadcast(this, id, intent, flags)
        alarmManager.cancel(pending)
        pending.cancel()
    }
