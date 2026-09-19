package com.looseends.loose_ends

import android.content.Context
import android.graphics.BitmapFactory
import android.graphics.PointF
import ai.onnxruntime.OnnxTensor
import ai.onnxruntime.OrtEnvironment
import ai.onnxruntime.OrtSession
import org.opencv.android.Utils
import org.opencv.core.Core
import org.opencv.core.CvType
import org.opencv.core.Mat
import org.opencv.core.MatOfPoint
import org.opencv.core.MatOfPoint2f
import org.opencv.core.Point
import org.opencv.core.Scalar
import org.opencv.core.Size
import org.opencv.imgproc.Imgproc
import java.io.File
import java.nio.FloatBuffer
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.cos
import kotlin.math.floor
import kotlin.math.hypot
import kotlin.math.max
import kotlin.math.min

/**
 * Minimal on-device PP-OCRv5 pipeline adapted from the Apache-2.0 PaddleOCR
 * Android deployment implementation.
 */
class OfflineOcrEngine(private val context: Context) {
    private var env: OrtEnvironment? = null
    private var detSession: OrtSession? = null
    private var recSession: OrtSession? = null
    private var characters: List<String> = emptyList()

    @Synchronized
    fun recognize(imageFile: File): String {
        require(imageFile.isFile) { "OCR image is not available." }
        ensureLoaded()
        val bitmap = BitmapFactory.decodeFile(imageFile.absolutePath)
            ?: throw IllegalArgumentException("Could not decode the screenshot.")
        try {
            val src = bitmapToBgr(bitmap)
            return try {
                recognizeMat(src)
            } finally {
                src.release()
            }
        } finally {
            bitmap.recycle()
        }
    }

    @Synchronized
    fun release() {
        detSession?.close()
        recSession?.close()
        detSession = null
        recSession = null
        env = null
        characters = emptyList()
    }

    private fun ensureLoaded() {
        if (detSession != null && recSession != null && characters.isNotEmpty()) return
        val det = OcrModelManager.detFile(context)
            ?: throw IllegalStateException("OCR detection model is not downloaded and verified.")
        val rec = OcrModelManager.recFile(context)
            ?: throw IllegalStateException("OCR recognition model is not downloaded and verified.")
        val dict = OcrModelManager.dictFile(context)
            ?: throw IllegalStateException("OCR dictionary is not downloaded and verified.")

        System.loadLibrary("opencv_java4")
        env = OrtEnvironment.getEnvironment()
        val options = OrtSession.SessionOptions().apply {
            setOptimizationLevel(OrtSession.SessionOptions.OptLevel.ALL_OPT)
            setIntraOpNumThreads(4)
        }
        try {
            val e = env ?: error("ONNX Runtime environment unavailable.")
            detSession = e.createSession(det.readBytes(), options)
            recSession = e.createSession(rec.readBytes(), options)
            characters = dict.readLines(Charsets.UTF_8).filter { it.isNotEmpty() } + listOf(" ")
        } finally {
            options.close()
        }
    }

    private fun recognizeMat(src: Mat): String {
        val detPre = preprocessDetection(src)
        val (detOut, detShape) = run(detSession ?: error("Detection model unavailable"), detPre.data, detPre.shape)
        val boxes = postprocessDetection(detOut, detShape, src.rows(), src.cols())
        if (boxes.isEmpty()) return ""

        val lines = boxes.mapNotNull { box ->
            val crop = cropQuad(src, box)
            try {
                if (crop.empty()) return@mapNotNull null
                val recPre = preprocessRecognition(crop)
                val (out, shape) = run(recSession ?: error("Recognition model unavailable"), recPre.data, recPre.shape)
                decode(out, shape).takeIf { it.isNotBlank() }
            } finally {
                crop.release()
            }
        }
        return lines.joinToString("\n").trim()
    }

    private data class Tensor(val data: FloatArray, val shape: LongArray)

    private fun preprocessDetection(src: Mat): Tensor {
        val resized = resizeDetection(src)
        try {
            val h = resized.rows()
            val w = resized.cols()
            val floatMat = Mat(h, w, CvType.CV_32FC3)
            resized.convertTo(floatMat, CvType.CV_32F)
            Core.divide(floatMat, Scalar(255.0, 255.0, 255.0), floatMat)
            val channels = mutableListOf<Mat>()
            Core.split(floatMat, channels)
            val means = doubleArrayOf(0.485, 0.456, 0.406)
            val std = doubleArrayOf(0.229, 0.224, 0.225)
            val data = FloatArray(3 * h * w)
            for (c in 0..2) {
                Core.subtract(channels[c], Scalar(means[c]), channels[c])
                Core.divide(channels[c], Scalar(std[c]), channels[c])
                val buf = FloatArray(h * w)
                channels[c].get(0, 0, buf)
                System.arraycopy(buf, 0, data, c * h * w, h * w)
                channels[c].release()
            }
            floatMat.release()
            return Tensor(data, longArrayOf(1, 3, h.toLong(), w.toLong()))
        } finally {
            resized.release()
        }
    }

    private fun resizeDetection(src: Mat): Mat {
        val h = src.rows()
        val w = src.cols()
        val limit = 64.0
        val minSide = min(h, w).toDouble()
        var ratio = if (minSide < limit) limit / minSide else 1.0
        var newH = (h * ratio).toInt()
        var newW = (w * ratio).toInt()
        val maxSideLimit = 4000
        if (max(newH, newW) > maxSideLimit) {
            ratio = maxSideLimit.toDouble() / max(newH, newW).toDouble()
            newH = (newH * ratio).toInt()
            newW = (newW * ratio).toInt()
        }
        newH = max(roundHalfToEven(newH / 32.0) * 32, 32)
        newW = max(roundHalfToEven(newW / 32.0) * 32, 32)
        return Mat().also {
            Imgproc.resize(src, it, Size(newW.toDouble(), newH.toDouble()), 0.0, 0.0, Imgproc.INTER_LINEAR)
        }
    }

    private fun preprocessRecognition(crop: Mat): Tensor {
        val rgb = Mat()
        Imgproc.cvtColor(crop, rgb, Imgproc.COLOR_BGR2RGB)
        try {
            val h = rgb.rows().coerceAtLeast(1)
            val w = rgb.cols().coerceAtLeast(1)
            val newW = ceil(48.0 * w.toDouble() / h.toDouble()).toInt().coerceIn(1, 3200)
            val resized = Mat()
            Imgproc.resize(rgb, resized, Size(newW.toDouble(), 48.0), 0.0, 0.0, Imgproc.INTER_LINEAR)
            try {
                val floatMat = Mat(resized.rows(), resized.cols(), CvType.CV_32FC3)
                resized.convertTo(floatMat, CvType.CV_32F)
                Core.divide(floatMat, Scalar(127.5, 127.5, 127.5), floatMat)
                Core.subtract(floatMat, Scalar(1.0, 1.0, 1.0), floatMat)

                val channels = mutableListOf<Mat>()
                Core.split(floatMat, channels)
                val channelSize = 48 * newW
                val data = FloatArray(3 * channelSize)
                for (c in 0..2) {
                    val buf = FloatArray(channelSize)
                    channels[c].get(0, 0, buf)
                    System.arraycopy(buf, 0, data, c * channelSize, channelSize)
                    channels[c].release()
                }
                floatMat.release()
                return Tensor(data, longArrayOf(1, 3, 48, newW.toLong()))
            } finally {
                resized.release()
            }
        } finally {
            rgb.release()
        }
    }

    private fun run(session: OrtSession, input: FloatArray, shape: LongArray): Pair<FloatArray, LongArray> {
        val e = env ?: error("ONNX Runtime environment unavailable.")
        val tensor = OnnxTensor.createTensor(e, FloatBuffer.wrap(input), shape)
        try {
            session.run(mapOf(session.inputNames.first() to tensor)).use { result ->
                val output = result[0] as OnnxTensor
                val buffer = output.floatBuffer.duplicate()
                buffer.rewind()
                val values = FloatArray(buffer.remaining())
                buffer.get(values)
                return values to output.info.shape
            }
        } finally {
            tensor.close()
        }
    }

    private fun postprocessDetection(
        pred: FloatArray,
        shape: LongArray,
        originalH: Int,
        originalW: Int,
    ): List<List<PointF>> {
        if (shape.size < 4) return emptyList()
        val pH = shape[2].toInt()
        val pW = shape[3].toInt()
        if (pH <= 0 || pW <= 0 || pred.size < pH * pW) return emptyList()

        val prob = Mat(pH, pW, CvType.CV_32FC1)
        val mask = Mat(pH, pW, CvType.CV_8UC1)
        val contours = mutableListOf<MatOfPoint>()
        val hierarchy = Mat()
        try {
            prob.put(0, 0, pred)
            val thresh = Mat()
            Imgproc.threshold(prob, thresh, 0.3, 255.0, Imgproc.THRESH_BINARY)
            thresh.convertTo(mask, CvType.CV_8UC1)
            thresh.release()
            Imgproc.findContours(mask, contours, hierarchy, Imgproc.RETR_LIST, Imgproc.CHAIN_APPROX_SIMPLE)

            val scaleX = originalW.toDouble() / pW.toDouble()
            val scaleY = originalH.toDouble() / pH.toDouble()
            val result = mutableListOf<Pair<List<PointF>, Float>>()

            for (contour in contours.take(3000)) {
                val input = MatOfPoint2f(*contour.toArray())
                val rect = try { Imgproc.minAreaRect(input) } finally { input.release() }
                if (min(rect.size.width, rect.size.height) < 3.0) continue

                val pts = Array(4) { Point() }
                rect.points(pts)
                val ordered = orderPoints(pts)
                val score = boxScore(prob, ordered)
                if (score < 0.6f) continue

                val expanded = unclip(ordered, 1.5)
                val expInput = MatOfPoint2f().apply { fromList(expanded) }
                val expRect = try { Imgproc.minAreaRect(expInput) } finally { expInput.release() }
                if (min(expRect.size.width, expRect.size.height) < 5.0) continue

                val expPoints = Array(4) { Point() }
                expRect.points(expPoints)
                val scaled = orderPoints(expPoints).map {
                    PointF(
                        (it.x * scaleX).coerceIn(0.0, originalW.toDouble()).toFloat(),
                        (it.y * scaleY).coerceIn(0.0, originalH.toDouble()).toFloat(),
                    )
                }
                val width = hypot(scaled[1].x - scaled[0].x, scaled[1].y - scaled[0].y)
                val height = hypot(scaled[3].x - scaled[0].x, scaled[3].y - scaled[0].y)
                if (width > 3.0 && height > 3.0) result += scaled to score
            }

            return result.sortedWith(compareBy({ it.first[0].y }, { it.first[0].x })).map { it.first }
        } finally {
            hierarchy.release()
            contours.forEach { it.release() }
            mask.release()
            prob.release()
        }
    }

    private fun boxScore(prob: Mat, points: List<Point>): Float {
        if (points.isEmpty()) return 0f
        val xmin = points.minOf { floor(it.x).toInt() }.coerceIn(0, prob.cols() - 1)
        val xmax = points.maxOf { ceil(it.x).toInt() }.coerceIn(0, prob.cols() - 1)
        val ymin = points.minOf { floor(it.y).toInt() }.coerceIn(0, prob.rows() - 1)
        val ymax = points.maxOf { ceil(it.y).toInt() }.coerceIn(0, prob.rows() - 1)
        if (xmax < xmin || ymax < ymin) return 0f

        val mask = Mat(ymax - ymin + 1, xmax - xmin + 1, CvType.CV_8UC1, Scalar(0.0))
        val pts = MatOfPoint().apply {
            fromList(points.map { Point((it.x - xmin).toInt().toDouble(), (it.y - ymin).toInt().toDouble()) })
        }
        Imgproc.fillPoly(mask, mutableListOf(pts), Scalar(1.0))
        val roi = prob.submat(ymin, ymax + 1, xmin, xmax + 1)
        val meanValue = Core.mean(roi, mask).`val`[0].toFloat()
        roi.release()
        mask.release()
        pts.release()
        return meanValue
    }

    private fun unclip(points: List<Point>, ratio: Double): List<Point> {
        if (points.size < 3) return points
        var twiceArea = 0.0
        var perimeter = 0.0
        for (i in points.indices) {
            val a = points[i]
            val b = points[(i + 1) % points.size]
            twiceArea += a.x * b.y - b.x * a.y
            perimeter += hypot(b.x - a.x, b.y - a.y)
        }
        val area = abs(twiceArea) / 2.0
        if (area <= 1e-6 || perimeter <= 1e-6) return points
        val distance = area * ratio / perimeter
        if (distance <= 1e-6) return points
        val clockwise = twiceArea > 0.0
        val normals = points.indices.map { i ->
            val a = points[i]
            val b = points[(i + 1) % points.size]
            val dx = b.x - a.x
            val dy = b.y - a.y
            val len = hypot(dx, dy).coerceAtLeast(1e-6)
            if (clockwise) Point(dy / len, -dx / len) else Point(-dy / len, dx / len)
        }

        val out = mutableListOf<Point>()
        for (i in points.indices) {
            val center = points[i]
            val from = normals[(i - 1 + normals.size) % normals.size]
            val to = normals[i]
            var start = kotlin.math.atan2(from.y, from.x)
            var end = kotlin.math.atan2(to.y, to.x)
            if (clockwise) {
                while (end < start) end += 2.0 * Math.PI
            } else {
                while (end > start) end -= 2.0 * Math.PI
            }
            val sweep = end - start
            val steps = ceil(abs(sweep) / (Math.PI / 8.0)).toInt().coerceAtLeast(1)
            for (step in 0..steps) {
                val angle = start + sweep * step.toDouble() / steps.toDouble()
                out += Point(center.x + cos(angle) * distance, center.y + sin(angle) * distance)
            }
        }
        return out
    }

    private fun cropQuad(src: Mat, box: List<PointF>): Mat {
        val points = box.map { Point(it.x.toDouble(), it.y.toDouble()) }
        val inPoints = MatOfPoint2f().apply { fromList(points) }
        val rect = try { Imgproc.minAreaRect(inPoints) } finally { inPoints.release() }
        val rectPoints = Array(4) { Point() }
        rect.points(rectPoints)
        val ordered = orderPoints(rectPoints)
        val width = max(
            hypot(ordered[0].x - ordered[1].x, ordered[0].y - ordered[1].y),
            hypot(ordered[2].x - ordered[3].x, ordered[2].y - ordered[3].y),
        ).toInt().coerceAtLeast(1)
        val height = max(
            hypot(ordered[0].x - ordered[3].x, ordered[0].y - ordered[3].y),
            hypot(ordered[1].x - ordered[2].x, ordered[1].y - ordered[2].y),
        ).toInt().coerceAtLeast(1)

        val srcPts = MatOfPoint2f().apply { fromList(ordered) }
        val dstPts = MatOfPoint2f().apply {
            fromList(
                listOf(
                    Point(0.0, 0.0),
                    Point(width.toDouble(), 0.0),
                    Point(width.toDouble(), height.toDouble()),
                    Point(0.0, height.toDouble()),
                ),
            )
        }
        val transform = Imgproc.getPerspectiveTransform(srcPts, dstPts)
        srcPts.release()
        dstPts.release()

        val dst = Mat(height, width, CvType.CV_8UC3)
        Imgproc.warpPerspective(
            src,
            dst,
            transform,
            Size(width.toDouble(), height.toDouble()),
            Imgproc.INTER_CUBIC,
            Core.BORDER_REPLICATE,
        )
        transform.release()

        if (height.toDouble() / width.toDouble() >= 1.5) {
            val rotated = Mat()
            Core.rotate(dst, rotated, Core.ROTATE_90_COUNTERCLOCKWISE)
            dst.release()
            return rotated
        }
        return dst
    }

    private fun orderPoints(points: Array<Point>): List<Point> {
        val sorted = points.sortedBy { it.x }
        val left = sorted.take(2).sortedBy { it.y }
        val right = sorted.takeLast(2).sortedBy { it.y }
        return listOf(left[0], right[0], right[1], left[1])
    }

    private fun decode(output: FloatArray, shape: LongArray): String {
        if (shape.size != 3) return ""
        val time = shape[1].toInt()
        val classes = shape[2].toInt()
        if (shape[0] < 1 || time < 1 || classes < 2) return ""

        val sb = StringBuilder()
        var previous = -1
        for (t in 0 until time) {
            val offset = t * classes
            var best = 0
            var bestValue = output[offset]
            for (c in 1 until classes) {
                val value = output[offset + c]
                if (value > bestValue) {
                    bestValue = value
                    best = c
                }
            }
            if (best != 0 && best != previous) {
                val idx = best - 1
                if (idx in characters.indices) sb.append(characters[idx])
            }
            previous = best
        }
        return sb.toString().trim()
    }

    private fun bitmapToBgr(bitmap: android.graphics.Bitmap): Mat {
        val rgba = Mat()
        val bgr = Mat()
        Utils.bitmapToMat(bitmap, rgba)
        Imgproc.cvtColor(rgba, bgr, Imgproc.COLOR_RGBA2BGR)
        rgba.release()
        return bgr
    }

    private fun roundHalfToEven(value: Double): Int {
        val floorValue = floor(value)
        val diff = value - floorValue
        return when {
            diff < 0.5 -> floorValue.toInt()
            diff > 0.5 -> floorValue.toInt() + 1
            floorValue.toInt() % 2 == 0 -> floorValue.toInt()
            else -> floorValue.toInt() + 1
        }
    }
}
