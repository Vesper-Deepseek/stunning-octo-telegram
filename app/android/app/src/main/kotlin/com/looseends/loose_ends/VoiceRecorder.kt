package com.looseends.loose_ends

import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import java.io.File
import java.io.RandomAccessFile

class VoiceRecorder(private val outputDir: File) {
    companion object {
        private const val SAMPLE_RATE = 16_000
        private const val CHANNELS = 1
        private const val BITS_PER_SAMPLE = 16
        private const val MAX_DURATION_MS = 60_000L
    }

    @Volatile
    private var recording = false
    private var recorder: AudioRecord? = null
    private var worker: Thread? = null
    private var currentFile: File? = null

    @Synchronized
    fun start(): File {
        check(!recording) { "Voice recording is already active." }

        val minBuffer = AudioRecord.getMinBufferSize(
            SAMPLE_RATE,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT
        )
        if (minBuffer <= 0) {
            throw IllegalStateException("The microphone is not available on this device.")
        }

        val bufferSize = maxOf(minBuffer, 4096)
        val audioRecord = AudioRecord(
            MediaRecorder.AudioSource.MIC,
            SAMPLE_RATE,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
            bufferSize * 2
        )
        check(audioRecord.state == AudioRecord.STATE_INITIALIZED) {
            audioRecord.release()
            "Unable to initialize the microphone."
        }

        outputDir.mkdirs()
        val file = File.createTempFile("loose-ends-voice-", ".wav", outputDir)
        recorder = audioRecord
        currentFile = file
        recording = true

        worker = Thread({
            writeWav(audioRecord, file)
        }, "loose-ends-voice-recorder").also { it.start() }

        return file
    }

    @Synchronized
    fun stop(): File? {
        if (!recording) return null
        recording = false
        try {
            recorder?.stop()
        } catch (_: IllegalStateException) {
            // The recorder may already have stopped because the device ended capture.
        }
        recorder?.release()
        recorder = null

        val thread = worker
        if (thread != null && thread !== Thread.currentThread()) {
            try {
                thread.join(2_000)
            } catch (_: InterruptedException) {
                Thread.currentThread().interrupt()
            }
        }
        worker = null

        val result = currentFile
        currentFile = null
        return result?.takeIf { it.isFile && it.length() > 44L }
    }

    @Synchronized
    fun cancel() {
        recording = false
        try {
            recorder?.stop()
        } catch (_: IllegalStateException) {
        }
        recorder?.release()
        recorder = null
        worker?.interrupt()
        worker = null
        currentFile?.delete()
        currentFile = null
    }

    private fun writeWav(audioRecord: AudioRecord, file: File) {
        var dataBytes = 0L
        try {
            RandomAccessFile(file, "rw").use { out ->
                writeHeader(out, 0)
                val buffer = ShortArray(2048)
                val started = System.currentTimeMillis()
                audioRecord.startRecording()

                while (recording &&
                    System.currentTimeMillis() - started < MAX_DURATION_MS
                ) {
                    val read = audioRecord.read(buffer, 0, buffer.size)
                    if (read <= 0) continue
                    for (i in 0 until read) {
                        out.writeByte(buffer[i].toInt() and 0xff)
                        out.writeByte((buffer[i].toInt() shr 8) and 0xff)
                    }
                    dataBytes += read * 2L
                }

                writeHeader(out, dataBytes)
                out.fd.sync()
            }
        } catch (_: Exception) {
            file.delete()
        } finally {
            recording = false
        }
    }

    private fun writeHeader(out: RandomAccessFile, dataBytes: Long) {
        out.seek(0)
        out.writeBytes("RIFF")
        writeLeInt(out, (36L + dataBytes).toInt())
        out.writeBytes("WAVE")
        out.writeBytes("fmt ")
        writeLeInt(out, 16)
        writeLeShort(out, 1)
        writeLeShort(out, CHANNELS)
        writeLeInt(out, SAMPLE_RATE)
        val byteRate = SAMPLE_RATE * CHANNELS * BITS_PER_SAMPLE / 8
        writeLeInt(out, byteRate)
        val blockAlign = CHANNELS * BITS_PER_SAMPLE / 8
        writeLeShort(out, blockAlign)
        writeLeShort(out, BITS_PER_SAMPLE)
        out.writeBytes("data")
        writeLeInt(out, dataBytes.toInt())
    }

    private fun writeLeInt(out: RandomAccessFile, value: Int) {
        out.writeByte(value and 0xff)
        out.writeByte((value shr 8) and 0xff)
        out.writeByte((value shr 16) and 0xff)
        out.writeByte((value shr 24) and 0xff)
    }

    private fun writeLeShort(out: RandomAccessFile, value: Int) {
        out.writeByte(value and 0xff)
        out.writeByte((value shr 8) and 0xff)
    }
}
