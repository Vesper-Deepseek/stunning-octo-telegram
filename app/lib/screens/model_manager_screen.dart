import 'dart:async';
import 'package:flutter/material.dart';
import '../bridge/loose_ends_bridge.dart';

class ModelManagerScreen extends StatefulWidget {
  const ModelManagerScreen({super.key});

  @override
  State<ModelManagerScreen> createState() => _ModelManagerScreenState();
}

class _ModelManagerScreenState extends State<ModelManagerScreen> {
  List<Map<String, dynamic>> _models = const [];
  Map<String, dynamic> _status = const {};
  Map<String, dynamic> _voiceStatus = const {};
  StreamSubscription<Map<String, dynamic>>? _progressSub;
  Map<String, dynamic>? _progress;
  String? _error;
  bool _busy = false;
  bool _allowMobile = false;

  @override
  void initState() {
    super.initState();
    _load();
    _progressSub = LooseEndsBridge.modelProgress.listen((event) {
      if (!mounted) return;
      setState(() => _progress = event);
    });
  }

  @override
  void dispose() {
    _progressSub?.cancel();
    super.dispose();
  }

  Future<void> _load() async {
    final models = await LooseEndsBridge.modelCatalog();
    final status = await LooseEndsBridge.modelStatus();
    final voiceStatus = await LooseEndsBridge.voiceModelStatus();
    if (!mounted) return;
    setState(() {
      _models = models;
      _status = status;
      _voiceStatus = voiceStatus;
    });
  }

  Future<void> _download(Map<String, dynamic> model) async {
    final name = model['name']?.toString() ?? 'model';
    final accepted = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Download $name?'),
        content: const Text(
          'This downloads a large model over the internet. Wi-Fi is required by default. '
          'Mobile data can be enabled only by you. The model is downloaded from the fixed '
          'official Hugging Face source and is SHA-256 checked before it is made available.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Continue'),
          ),
        ],
      ),
    );
    if (accepted != true || !mounted) return;

    setState(() {
      _busy = true;
      _error = null;
      _progress = null;
    });
    final error = await LooseEndsBridge.downloadModel(
      model['id']?.toString() ?? '',
      allowMobile: _allowMobile,
    );
    if (!mounted) return;
    setState(() {
      _busy = false;
      _error = error;
    });
    await _load();
  }

  Future<void> _cancel() async {
    await LooseEndsBridge.cancelModelDownload();
    if (!mounted) return;
    setState(() => _busy = false);
    await _load();
  }

  Future<void> _downloadVoice() async {
    final accepted = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Download Whisper voice model?'),
        content: const Text(
          'This downloads the on-device Whisper tiny.en model from the fixed official source. '
          'The file is SHA-256 checked before use and Wi-Fi is required by default.',
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
    if (accepted != true || !mounted) return;
    setState(() {
      _busy = true;
      _error = null;
      _progress = null;
    });
    final error = await LooseEndsBridge.downloadVoiceModel(allowMobile: _allowMobile);
    if (!mounted) return;
    setState(() {
      _busy = false;
      _error = error;
    });
    await _load();
  }

  Future<void> _cancelVoice() async {
    await LooseEndsBridge.cancelVoiceModelDownload();
    if (!mounted) return;
    setState(() => _busy = false);
    await _load();
  }

  Future<void> _deleteVoice() async {
    final yes = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete Whisper voice model?'),
        content: const Text('Voice transcription will ask you to download it again later.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (yes != true) return;
    await LooseEndsBridge.cancelVoiceModelDownload();
    await _load();
  }

  Future<void> _delete(String modelId, String name) async {
    final yes = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Delete $name?'),
        content: const Text('This frees local storage. You can download it again later.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (yes != true) return;
    await LooseEndsBridge.deleteModel(modelId);
    await _load();
  }

  Future<void> _select(String modelId) async {
    final ok = await LooseEndsBridge.selectModel(modelId);
    if (!mounted) return;
    if (!ok) {
      setState(() => _error = 'This model is not fully verified and ready.');
    }
    await _load();
  }

  Future<void> _import() async {
    final imported = await LooseEndsBridge.pickCustomGguf();
    if (!mounted || imported == null) return;
    final verified = imported['verified'] == true;
    setState(() {
      _error = verified
          ? null
          : 'Imported GGUF is not one of the known model hashes. It is marked user-imported and unverified by this app.';
    });
    await _load();
  }

  String _bytes(Object? value) {
    final n = value is num ? value.toDouble() : 0;
    if (n >= 1024 * 1024 * 1024) {
      return '${(n / 1024 / 1024 / 1024).toStringAsFixed(2)} GB';
    }
    return '${(n / 1024 / 1024).toStringAsFixed(0)} MB';
  }

  List<Map<String, dynamic>> get _statusModels {
    final value = _status['models'];
    if (value is! List) return const [];
    return value.cast<Map>().map(
      (m) => m.map((key, value) => MapEntry(key.toString(), value)),
    ).toList();
  }

  @override
  Widget build(BuildContext context) {
    final statusModels = _statusModels;
    final progress = _progress;
    final activeId = progress?['modelId']?.toString();
    final percent = (progress?['percent'] as num?)?.toDouble() ?? 0.0;

    return Scaffold(
      appBar: AppBar(title: const Text('AI model')),
      body: RefreshIndicator(
        onRefresh: _load,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            const Text(
              'Models stay on this device. Nothing from your commitments is sent during model download.',
            ),
            const SizedBox(height: 16),
            SwitchListTile(
              value: _allowMobile,
              onChanged: _busy ? null : (value) => setState(() => _allowMobile = value),
              title: const Text('Allow mobile data'),
              subtitle: const Text(
                'Off by default; model downloads are Wi-Fi-only unless you explicitly enable this.',
              ),
            ),
            const SizedBox(height: 8),
            ..._models.map((model) {
              final id = model['id']?.toString() ?? '';
              final matching = statusModels.where((m) => m['id']?.toString() == id);
              final item = matching.isNotEmpty ? matching.first : model;
              final ready = item['ready'] == true;
              final downloaded = item['downloaded'] == true;
              final isActive = id == activeId && _busy;

              return Card(
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        item['name']?.toString() ?? '',
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 4),
                      Text(item['description']?.toString() ?? ''),
                      const SizedBox(height: 4),
                      Text(
                        '${_bytes(item['sizeBytes'])} • SHA-256 verified before use',
                      ),
                      if (isActive) ...[
                        const SizedBox(height: 12),
                        LinearProgressIndicator(value: percent / 100),
                        const SizedBox(height: 4),
                        Text(
                          '${percent.toStringAsFixed(1)}% • ${_bytes(progress?['downloadedBytes'])} / ${_bytes(progress?['totalBytes'])}',
                        ),
                      ],
                      const SizedBox(height: 8),
                      Wrap(
                        spacing: 8,
                        children: [
                          if (isActive)
                            OutlinedButton(
                              onPressed: _cancel,
                              child: const Text('Cancel'),
                            )
                          else if (ready)
                            FilledButton(
                              onPressed: () => _select(id),
                              child: const Text('Use this model'),
                            )
                          else
                            FilledButton(
                              onPressed: _busy ? null : () => _download(item),
                              child: Text(downloaded ? 'Verify & use' : 'Download'),
                            ),
                          if (downloaded || ready)
                            TextButton(
                              onPressed: _busy
                                  ? null
                                  : () => _delete(
                                        id,
                                        item['name']?.toString() ?? 'model',
                                      ),
                              child: const Text('Delete'),
                            ),
                        ],
                      ),
                      if (item['selected'] == true)
                        const Padding(
                          padding: EdgeInsets.only(top: 4),
                          child: Text('Selected for AI extraction.'),
                        ),
                    ],
                  ),
                ),
              );
            }),
            const SizedBox(height: 16),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Builder(
                  builder: (context) {
                    final voiceDownloaded = _voiceStatus['downloaded'] == true;
                    final voiceSelected = _voiceStatus['selected'] == true;
                    final voiceActive =
                        _progress?['modelId']?.toString() == 'whisper_tiny_en_q5_1' && _busy;
                    final voicePercent =
                        (_progress?['percent'] as num?)?.toDouble() ?? 0.0;
                    return Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Voice transcription',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        const SizedBox(height: 4),
                        const Text(
                          'Whisper tiny.en runs locally on recorded audio. '
                          'The recording is removed after transcription.',
                        ),
                        const SizedBox(height: 4),
                        Text('${_bytes(_voiceStatus["sizeBytes"])} • SHA-256 verified before use'),
                        if (voiceActive) ...[
                          const SizedBox(height: 12),
                          LinearProgressIndicator(value: voicePercent / 100),
                          const SizedBox(height: 4),
                          Text('${voicePercent.toStringAsFixed(1)}%'),
                        ],
                        const SizedBox(height: 8),
                        Wrap(
                          spacing: 8,
                          children: [
                            if (voiceActive)
                              OutlinedButton(
                                onPressed: _cancelVoice,
                                child: const Text('Cancel'),
                              )
                            else if (voiceDownloaded)
                              OutlinedButton(
                                onPressed: null,
                                child: Text(
                                  voiceSelected ? 'Ready for Voice' : 'Available for Voice',
                                ),
                              )
                            else
                              FilledButton(
                                onPressed: _busy ? null : _downloadVoice,
                                child: const Text('Download'),
                              ),
                            if (voiceDownloaded)
                              TextButton(
                                onPressed: _busy ? null : _deleteVoice,
                                child: const Text('Delete'),
                              ),
                          ],
                        ),
                      ],
                    );
                  },
                ),
              ),
            ),
            const SizedBox(height: 12),
            OutlinedButton.icon(
              onPressed: _busy ? null : _import,
              icon: const Icon(Icons.folder_open),
              label: const Text('Import an existing GGUF'),
            ),
            const SizedBox(height: 6),
            const Text(
              'Advanced import: choose a GGUF you already downloaded and verified yourself. Known model hashes are recognized automatically.',
              style: TextStyle(fontSize: 12),
            ),
            if (_error != null) ...[
              const SizedBox(height: 12),
              Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
