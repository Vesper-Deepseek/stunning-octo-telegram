package com.looseends.loose_ends

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private val channelName = "com.looseends/core"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName)
            .setMethodCallHandler { call, result ->
                val bridge = NativeBridge.getInstance()
                when (call.method) {
                    "init" -> {
                        bridge.init(applicationContext.filesDir.absolutePath)
                        result.success(null)
                    }
                    "ingestText" -> {
                        val text = call.argument<String>("text") ?: ""
                        val today = java.time.LocalDate.now()
                        val json = bridge.ingestRules(
                            text,
                            today.year, today.monthValue, today.dayOfMonth
                        )
                        result.success(json)
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
                        result.success(json)
                    }
                    else -> result.notImplemented()
                }
            }
    }
}
