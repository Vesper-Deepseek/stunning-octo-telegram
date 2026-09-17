import 'package:flutter/material.dart';
import '../bridge/loose_ends_bridge.dart';
import '../models/direction.dart';
import '../models/draft.dart';

class ReviewScreen extends StatefulWidget {
  final List<Draft>? newDrafts;
  const ReviewScreen({super.key, this.newDrafts});

  @override
  State<ReviewScreen> createState() => _ReviewScreenState();
}

class _ReviewScreenState extends State<ReviewScreen> {
  late List<Draft> _drafts;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _drafts = widget.newDrafts ?? [];
  }

  Future<void> _confirm(Draft draft) async {
    setState(() => _busy = true);
    final id = await LooseEndsBridge.confirmDraft(draft);
    if (!mounted) return;
    setState(() {
      _drafts.remove(draft);
      _busy = false;
    });
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(id != null ? 'Saved' : 'Failed to save')),
    );
  }

  void _dismiss(Draft draft) {
    setState(() => _drafts.remove(draft));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Review')),
      body: _drafts.isEmpty
          ? const Center(
              child: Text('No drafts to review'),
            )
          : ListView.builder(
              itemCount: _drafts.length,
              itemBuilder: (_, i) {
                final d = _drafts[i];
                final dir = Direction.fromString(d.direction);
                return Card(
                  margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                  child: Padding(
                    padding: const EdgeInsets.all(12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            _badge(dir.displayName, _directionColor(dir)),
                            const SizedBox(width: 8),
                            if (d.party != null) _badge('Party: ${d.party}', Colors.blue),
                            const SizedBox(width: 8),
                            if (d.overallConfidence == 'low')
                              const Icon(Icons.warning_amber, color: Colors.orange, size: 18),
                          ],
                        ),
                        const SizedBox(height: 8),
                        Text(d.description,
                            style: Theme.of(context).textTheme.bodyLarge),
                        if (d.expectedDate != null)
                          Padding(
                            padding: const EdgeInsets.only(top: 4),
                            child: Text(
                              'Due: ${d.expectedDate}',
                              style: const TextStyle(color: Colors.black54),
                            ),
                          ),
                        const SizedBox(height: 8),
                        Row(
                          mainAxisAlignment: MainAxisAlignment.end,
                          children: [
                            TextButton(
                              onPressed: _busy ? null : () => _dismiss(d),
                              child: const Text('Dismiss'),
                            ),
                            const SizedBox(width: 8),
                            FilledButton(
                              onPressed: _busy ? null : () => _confirm(d),
                              child: const Text('Confirm'),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
    );
  }

  Widget _badge(String text, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.2),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.6)),
      ),
      child: Text(text, style: TextStyle(color: color, fontSize: 12)),
    );
  }

  Color _directionColor(Direction d) {
    switch (d) {
      case Direction.userOwes:
        return Colors.red;
      case Direction.owedToUser:
        return Colors.green;
      case Direction.unclear:
        return Colors.grey;
    }
  }
}
