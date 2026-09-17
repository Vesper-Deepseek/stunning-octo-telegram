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
  bool _busy = false;

  Future<void> _submit() async {
    final text = _controller.text.trim();
    if (text.isEmpty) return;

    setState(() => _busy = true);
    try {
      final drafts = await LooseEndsBridge.ingestText(text);
      if (!mounted) return;

      if (drafts.isEmpty) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('No commitment detected.')),
        );
        return;
      }

      // Auto-navigate to review with the new drafts
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

  @override
  Widget build(BuildContext context) {
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
                    onPressed: _busy ? null : _submit,
                    icon: const Icon(Icons.send),
                    label: Text(_busy ? 'Extracting…' : 'Extract'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 24),
            const Text(
              'Local extraction. Your text never leaves the device.',
              style: TextStyle(fontSize: 12, color: Colors.grey),
            ),
          ],
        ),
      ),
    );
  }
}
