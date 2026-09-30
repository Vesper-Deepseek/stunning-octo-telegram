package com.looseends.loose_ends

import android.app.ActivityManager
import android.content.Context
import android.os.Debug
import android.util.Log

/**
 * Centralized Out-Of-Memory pre-flight guard for on-device inference.
 *
 * Both crash paths this protects against are *uncatchable* once they happen:
 *  - whisper.cpp / ggml call `operator new` and abort the process with
 *    SIGABRT (or GGML_ABORT on corrupt weights) when allocation fails — no
 *    Java/Kotlin exception is ever thrown, so try/catch cannot help;
 *  - ONNX Runtime's native session creation similarly dies inside C++ when
 *    device memory is exhausted.
 * The only reliable mitigation is to refuse to start an inference whose
 * memory footprint the kernel cannot back, and return a recoverable error
 * instead of letting the OS force-close the app.
 */
object MemoryGuard {
    private const val TAG = "MemoryGuard"

    /** Kernel-reported free memory in MB (`MemAvailable`), or null if unknown. */
    fun availableMemoryMb(context: Context): Long? = try {
        val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
        val info = ActivityManager.MemoryInfo()
        am?.getMemoryInfo(info)
        // ActivityManager exposes available system memory directly. MemoryInfo
        // has no memInfo map; use availMem and the low-memory signal instead.
        when {
            info.lowMemory -> 0L
            info.availMem > 0 -> info.availMem / (1024L * 1024L)
            else -> null
        }
    } catch (t: Throwable) {
        Log.w(TAG, "Could not read system memory status", t)
        null
    }

    /**
     * Returns true when we should refuse to start a native model load that is
     * estimated to need [estimatedModelPeakMb] (weights + activations).
     *
     * Fails open (returns false = proceed) when memory cannot be queried, so
     * devices with restricted procfs are not locked out of all inference.
     */
    fun wouldExceedMemoryBudget(context: Context, estimatedModelPeakMb: Long): Boolean {
        val availMb = availableMemoryMb(context) ?: return false
        // Keep a hard floor for the Android framework itself; below this the
        // low-memory killer will take us down mid-inference regardless.
        val headroomMb = availMb - SYSTEM_RESERVED_MB
        val overBudget = estimatedModelPeakMb > headroomMb
        if (overBudget) {
            Log.e(
                TAG,
                "Refusing inference: model needs ~${estimatedModelPeakMb}MB, " +
                    "only ${availMb}MB available (${headroomMb}MB usable after reserving " +
                    "${SYSTEM_RESERVED_MB}MB for the system)",
            )
        }
        return overBudget
    }

    /**
     * Recommended intra-op thread count for native runtimes (ONNX, ggml).
     *
     * Hardcoding threads (previously 4/6) oversubscribes small ARM cores and
     * multiplies per-thread activation buffers, which was a direct contributor
     * to OOM-aborts and main-event-loop starvation ("hangs indefinitely").
     */
    fun recommendedThreads(): Int =
        Runtime.getRuntime().availableProcessors().coerceIn(1, MAX_THREADS)

    /**
     * Approximate peak RSS Whisper/ggml needs for a GGML model, derived from
     * the weight size stored in the file header plus fixed overhead for the
     * mel spectrogram, KV cache and activation buffers.
     */
    fun estimatedWhisperPeakMb(modelFileBytes: Long): Long =
        (modelFileBytes / (1024L * 1024L)) * WEIGHT_MULTIPLIER + WHISPER_OVERHEAD_MB

    /** Approximate peak native footprint for an ONNX OCR pair. */
    fun estimatedOcrPeakMb(detBytes: Long, recBytes: Long): Long =
        ((detBytes + recBytes) / (1024L * 1024L)) * OCR_MULTIPLIER + OCR_OVERHEAD_MB

    private const val SYSTEM_RESERVED_MB = 256L
    private const val MAX_THREADS = 4
    private const val WEIGHT_MULTIPLIER = 4L
    private const val WHISPER_OVERHEAD_MB = 160L
    private const val OCR_MULTIPLIER = 3L
    private const val OCR_OVERHEAD_MB = 96L

    /** Unused helper kept for diagnostics: current process RSS in KB. */
    @Suppress("unused")
    private fun currentRssKb(): Long = try {
        val mi = Debug.MemoryInfo()
        Debug.getMemoryInfo(mi)
        mi.totalPss.toLong()
    } catch (t: Throwable) {
        0L
    }
}
