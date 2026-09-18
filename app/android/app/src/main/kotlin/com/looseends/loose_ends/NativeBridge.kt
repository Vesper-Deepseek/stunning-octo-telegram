package com.looseends.loose_ends

import org.json.JSONArray
import org.json.JSONObject

/**
 * JNI bridge to the Rust core (loose_ends_native cdylib).
 *
 * The native library is loaded once per process. The store is kept in a
 * long-lived native handle so SQLite connections aren't repeatedly opened.
 */
class NativeBridge private constructor() {
    @Volatile private var store: Long = 0

    fun init(dbDir: String): Boolean {
        if (store != 0L) return true
        synchronized(this) {
            if (store != 0L) return true
            val dbPath = "$dbDir/loose_ends.sqlite"
            store = try {
                looseEndsOpen(dbPath)
            } catch (e: UnsatisfiedLinkError) {
                android.util.Log.e("NativeBridge", "Native library unavailable", e)
                0L
            }
            return store != 0L
        }
    }

    fun ingestRules(text: String, year: Int, month: Int, day: Int): JSONArray? {
        val handle = store
        if (handle == 0L) return null
        val json = looseEndsIngestRules(handle, text, year, month, day) ?: return null
        return try {
            val obj = JSONObject(json)
            obj.optJSONArray("drafts") ?: JSONArray()
        } catch (e: Exception) {
            null
        }
    }

    fun confirmDraft(
        draftId: Long,
        description: String?,
        direction: String?,
        expectedDate: String?,
        party: String?
    ): Long {
        val handle = store
        if (handle == 0L) return 0
        return looseEndsConfirmDraft(
            handle, draftId, description, direction, expectedDate, party
        )
    }

    fun listOpen(direction: String, year: Int, month: Int, day: Int): JSONArray? {
        val handle = store
        if (handle == 0L) return null
        val json = looseEndsListOpen(handle, direction, year, month, day) ?: return null
        return try {
            JSONArray(json)
        } catch (e: Exception) {
            null
        }
    }

    companion object {
        init {
            try {
                System.loadLibrary("loose_ends_native")
            } catch (e: UnsatisfiedLinkError) {
                android.util.Log.e(
                    "NativeBridge",
                    "Failed to load native lib; app will run in stub mode",
                    e
                )
            }
        }

        @Volatile private var instance: NativeBridge? = null

        fun getInstance(): NativeBridge {
            return instance ?: synchronized(this) {
                instance ?: NativeBridge().also { instance = it }
            }
        }

        @JvmStatic private external fun looseEndsOpen(path: String): Long
        @JvmStatic private external fun looseEndsIngestRules(
            handle: Long, text: String, year: Int, month: Int, day: Int
        ): String?
        @JvmStatic private external fun looseEndsConfirmDraft(
            handle: Long, draftId: Long,
            description: String?, direction: String?,
            expectedDate: String?, party: String?
        ): Long
        @JvmStatic private external fun looseEndsListOpen(
            handle: Long, direction: String, year: Int, month: Int, day: Int
        ): String?
    }
}
