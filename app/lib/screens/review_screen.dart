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
    if (widget.newDrafts == null) {
      _loadDrafts();
    }
  }

  Future<void> _loadDrafts() async {
    final drafts = await LooseEndsBridge.listDrafts();
    if (mounted) {
      setState(() => _drafts = drafts);
    }
  }

  Future<void> _confirm(Draft draft) async {
    final dir = Direction.fromString(draft.direction);
    if (dir == Direction.unclear) {
      await _editDraft(draft);
      if (!mounted) return;
      final index = _drafts.indexWhere((d) => d.id == draft.id);
      if (index < 0 || Direction.fromString(_drafts[index].direction) == Direction.unclear) {
        return;
      }
      draft = _drafts[index];
    }

    setState(() => _busy = true);
    final id = await LooseEndsBridge.confirmDraft(draft);
    if (!mounted) return;
    setState(() {
      _drafts.removeWhere((d) => d.id == draft.id);
      _busy = false;
    });
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(id != null && id != 0 ? 'Saved' : 'Failed to save')),
    );
  }

  Future<void> _editDraft(Draft draft) async {
    final description = TextEditingController(text: draft.description);
    final party = TextEditingController(text: draft.party ?? '');
    final date = TextEditingController(text: draft.expectedDate ?? '');
    var direction = Direction.fromString(draft.direction);

    final edited = await showDialog<_DraftEdit>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: const Text('Edit before confirming'),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: description,
                  maxLines: 3,
                  decoration: const InputDecoration(
                    labelText: 'Description',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: party,
                  decoration: const InputDecoration(
                    labelText: 'Party',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                DropdownButtonFormField<Direction>(
                  initialValue: direction,
                  decoration: const InputDecoration(
                    labelText: 'Direction',
                    border: OutlineInputBorder(),
                  ),
                  items: Direction.values.map((value) => DropdownMenuItem(
                    value: value,
                    child: Text(value.displayName),
                  )).toList(),
                  onChanged: (value) {
                    if (value != null) {
                      setDialogState(() => direction = value);
                    }
                  },
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: date,
                  decoration: const InputDecoration(
                    labelText: 'Expected date (YYYY-MM-DD, optional)',
                    border: OutlineInputBorder(),
                  ),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(
                context,
                _DraftEdit(
                  description.text.trim(),
                  party.text.trim().isEmpty ? null : party.text.trim(),
                  direction,
                  date.text.trim().isEmpty ? null : date.text.trim(),
                ),
              ),
              child: const Text('Apply'),
            ),
          ],
        ),
      ),
    );

    if (edited == null || !mounted) return;
    final index = _drafts.indexWhere((d) => d.id == draft.id);
    if (index < 0) return;
    setState(() {
      _drafts[index] = Draft(
        id: draft.id,
        description: edited.description.isEmpty ? draft.description : edited.description,
        direction: edited.direction.name,
        expectedDate: edited.date,
        party: edited.party,
        partyConfidence: edited.party == draft.party ? draft.partyConfidence : 'low',
        dateConfidence: edited.date == draft.expectedDate ? draft.dateConfidence : 'low',
        overallConfidence: edited.direction == Direction.unclear ? 'low' : draft.overallConfidence,
        sourceProvenance: draft.sourceProvenance,
        createdAt: draft.createdAt,
      );
    });
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
                        Wrap(
                          spacing: 8,
                          runSpacing: 6,
                          children: [
                            _badge(dir.displayName, _directionColor(dir)),
                            if (d.party != null) _badge('Party: ${d.party}', Colors.blue),
                            _badge('Source: ${_provenanceName(d.sourceProvenance)}', Colors.blueGrey),
                            if (d.overallConfidence == 'low' ||
                                d.partyConfidence == 'low' ||
                                d.dateConfidence == 'low')
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
                            TextButton(
                              onPressed: _busy ? null : () => _editDraft(d),
                              child: const Text('Edit'),
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

  String _provenanceName(String value) {
    switch (value) {
      case 'model_extracted':
        return 'model';
      case 'rule_extracted':
        return 'rules';
      case 'manual':
        return 'manual';
      default:
        return value;
    }
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

class _DraftEdit {
  final String description;
  final String? party;
  final Direction direction;
  final String? date;

  const _DraftEdit(this.description, this.party, this.direction, this.date);
}
