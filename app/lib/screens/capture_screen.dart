import 'dart:async';
import 'package:flutter/material.dart';
import '../bridge/loose_ends_bridge.dart';
import 'review_screen.dart';

class CaptureScreen extends StatefulWidget {
  const CaptureScreen({super.key});

  @override
  State<CaptureScreen> createState() => _CaptureScreenState();
}

class _CaptureScreenState extends State<CaptureScreen> {
  final _controller = TextEditingController();
  StreamSubscription<Map<String, dynamic>>? _progressSub;
  bool _busy = false;
  bool _recording = false;
  bool _voiceBusy = false;
  double? _voiceProgress;
  String? _voiceMessage;
  bool _ocrBusy = false;
  double? _ocrProgress;
  String? _ocrMessage;
  String _sourceType = 'text';

  @override
  void initState() {
    super.initState();
    _progressSub = LooseEndsBridge.modelProgress.listen((event) {
      if (!mounted) return;
      final percent = (event['percent'] as num?)?.toDouble();
      if (event['modelId']?.toString() == 'whisper_tiny_en_q5_1') {
        setState(() => _voiceProgress = percent);
      } else if (event['assetId'] != null) {
        setState(() => _ocrProgress = percent);
      }
    });
  }

  @override
  void dispose() {
    _progressSub?.cancel();
    _controller.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final text = _controller.text.trim();
    if (text.isEmpty) return;

    setState(() => _busy = true);
    try {
      final drafts = await LooseEndsBridge.ingestText(
        text,
        sourceType: _sourceType,
      );
      if (!mounted) return;

      if (drafts.isEmpty) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('No commitment detected.')),
        );
        return;
      }

      await Navigator.pushReplacement(
        context,
        MaterialPageRoute(
          builder: (_) => ReviewScreen(newDrafts: drafts),
        ),
      );
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<bool> _ensureVoiceModel() async {
    var status = await LooseEndsBridge.voiceModelStatus();
    if (status['downloaded'] == true) return true;
    if (!mounted) return false;

    final accepted = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Download voice model?'),
        content: const Text(
          'Loose Ends uses an on-device Whisper model for voice transcription. '
          'The model is downloaded only after you approve it, from the fixed official source, '
          'and its SHA-256 is checked before use. Wi-Fi is required by default.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Download'),
          ),
        ],
      ),
    );
    if (accepted != true || !mounted) return false;

    setState(() {
      _voiceBusy = true;
      _voiceProgress = 0;
      _voiceMessage = null;
    });
    final error = await LooseEndsBridge.downloadVoiceModel();
    if (!mounted) return false;

    status = await LooseEndsBridge.voiceModelStatus();
    if (!mounted) return false;
    setState(() {
      _voiceBusy = false;
      _voiceMessage = error;
      _voiceProgress = error == null && status['downloaded'] == true ? 100 : _voiceProgress;
    });
    if (error != null || status['downloaded'] != true) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(error ?? 'Voice model is not ready.')),
      );
      return false;
    }
    return true;
  }

  Future<bool> _ensureOcrModels() async {
    var status = await LooseEndsBridge.ocrModelStatus();
    if (status['ready'] == true) return true;
    if (!mounted) return false;

    final accepted = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Download OCR models?'),
        content: const Text(
          'Loose Ends uses on-device OCR for screenshots. The OCR models are downloaded only '
          'after you approve it, from a fixed Apache-2.0 model source, and every file is '
          'SHA-256 checked before use. Wi-Fi is required by default.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Download'),
          ),
        ],
      ),
    );
    if (accepted != true || !mounted) return false;

    setState(() {
      _ocrBusy = true;
      _ocrProgress = 0;
      _ocrMessage = null;
    });
    final error = await LooseEndsBridge.downloadOcrModels();
    if (!mounted) return false;
    status = await LooseEndsBridge.ocrModelStatus();
    if (!mounted) return false;
    setState(() {
      _ocrBusy = false;
      _ocrMessage = error;
      _ocrProgress = error == null && status['ready'] == true ? 100 : _ocrProgress;
    });
    if (error != null || status['ready'] != true) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(error ?? 'OCR models are not ready.')),
      );
      return false;
    }
    return true;
  }

  Future<void> _pickScreenshot() async {
    if (_busy || _voiceBusy || _ocrBusy) return;
    final ready = await _ensureOcrModels();
    if (!ready || !mounted) return;

    setState(() {
      _ocrBusy = true;
      _ocrMessage = null;
    });
    try {
      final text = await LooseEndsBridge.pickScreenshotAndExtract();
      if (!mounted) return;
      if (text == null || text.trim().isEmpty) {
        setState(() => _ocrMessage = 'No readable text was found in that image.');
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('No readable text found.')),
        );
        return;
      }
      _controller.text = text.trim();
      _sourceType = 'screenshot';
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Screenshot text extracted locally.')),
      );
    } finally {
      if (mounted) setState(() => _ocrBusy = false);
    }
  }

  Future<void> _toggleVoice() async {
    if (_voiceBusy || _busy) return;

    if (!_recording) {
      final ready = await _ensureVoiceModel();
      if (!ready || !mounted) return;

      setState(() {
        _voiceBusy = true;
        _voiceMessage = null;
      });
      final started = await LooseEndsBridge.startVoiceRecording();
      if (!mounted) return;
      setState(() {
        _voiceBusy = false;
        _recording = started;
        if (!started) {
          _voiceMessage = 'Could not start microphone recording. Check microphone permission and device input.';
        }
      });
      return;
    }

    setState(() => _voiceBusy = true);
    try {
      final wavPath = await LooseEndsBridge.stopVoiceRecording();
      if (wavPath == null) {
        if (mounted) {
          setState(() {
            _recording = false;
            _voiceMessage = 'No usable audio was recorded.';
          });
        }
        return;
      }

      final transcript = await LooseEndsBridge.transcribeVoice(wavPath);
      if (!mounted) return;
      setState(() {
        _recording = false;
        _voiceMessage = transcript == null
            ? 'Whisper transcription failed. The audio remains local and was discarded.'
            : null;
        if (transcript != null && transcript.trim().isNotEmpty) {
          _controller.text = transcript.trim();
          _sourceType = 'voice';
        }
      });
      if (transcript == null || transcript.trim().isEmpty) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('No speech was detected.')),
        );
      }
    } finally {
      if (mounted) setState(() => _voiceBusy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final voiceDownloading = _voiceBusy && (_voiceProgress ?? 0) > 0 && !_recording;

    return Scaffold(
      appBar: AppBar(title: const Text('Capture')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            TextField(
              controller: _controller,
              maxLines: 5,
              decoration: const InputDecoration(
                hintText: 'Paste or type a message containing a commitment…',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: FilledButton.icon(
                    onPressed: _busy || _voiceBusy ? null : _submit,
                    icon: const Icon(Icons.send),
                    label: Text(_busy ? 'Extracting…' : 'Extract'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: _busy || _ocrBusy ? null : _toggleVoice,
                    icon: Icon(_recording ? Icons.stop : Icons.mic),
                    label: Text(
                      _recording
                          ? 'Stop & transcribe'
                          : (_voiceBusy ? 'Preparing…' : 'Voice'),
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              onPressed: _busy || _voiceBusy ? null : _pickScreenshot,
              icon: const Icon(Icons.image_search),
              label: Text(_ocrBusy ? 'Reading screenshot…' : 'Screenshot → OCR'),
            ),
            if (_ocrBusy && (_ocrProgress ?? 0) > 0) ...[
              const SizedBox(height: 8),
              LinearProgressIndicator(value: (_ocrProgress ?? 0) / 100),
              const SizedBox(height: 4),
              Text(
                'OCR models: ${(_ocrProgress ?? 0).toStringAsFixed(0)}%',
                style: const TextStyle(fontSize: 12),
              ),
            ],
            if (_ocrMessage != null) ...[
              const SizedBox(height: 8),
              Text(
                _ocrMessage!,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.error,
                  fontSize: 12,
                ),
              ),
            ],
            if (voiceDownloading) ...[
              const SizedBox(height: 12),
              LinearProgressIndicator(
                value: (_voiceProgress ?? 0) / 100,
              ),
              const SizedBox(height: 4),
              Text(
                'Downloading Whisper model: ${(_voiceProgress ?? 0).toStringAsFixed(0)}%',
                style: const TextStyle(fontSize: 12),
              ),
            ],
            if (_recording) ...[
              const SizedBox(height: 12),
              const Text(
                'Recording locally… tap Stop & transcribe when finished.',
                style: TextStyle(fontSize: 12),
              ),
            ],
            if (_voiceMessage != null) ...[
              const SizedBox(height: 12),
              Text(
                _voiceMessage!,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.error,
                  fontSize: 12,
                ),
              ),
            ],
            const SizedBox(height: 24),
            const Text(
              'Text and voice extraction stay on this device. Voice uses the local Whisper model.',
              style: TextStyle(fontSize: 12, color: Colors.grey),
            ),
          ],
        ),
      ),
    );
  }
}
