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
    _drafts = List<Draft>.from(widget.newDrafts ?? const <Draft>[]);
    // Always reload from SQLite. The capture result is only a convenience for
    // immediate navigation; the database is the source of truth for recovery.
    _loadDrafts();
  }

  Future<void> _loadDrafts() async {
    final persisted = await LooseEndsBridge.listDrafts();
    if (!mounted) return;

    final byId = <int, Draft>{
      for (final draft in _drafts)
        if (draft.id > 0) draft.id: draft,
    };
    for (final draft in persisted) {
      byId[draft.id] = draft;
    }

    setState(() {
      _drafts = byId.values.toList();
      _drafts.sort((a, b) => a.id.compareTo(b.id));
    });
  }

  Future<void> _confirm(Draft draft) async {
    var current = draft;
    var dir = Direction.fromString(current.direction);
    if (dir == Direction.unclear) {
      await _editDraft(current);
      if (!mounted) return;
      final index = _drafts.indexWhere((d) => d.id == current.id);
      if (index < 0) return;
      current = _drafts[index];
      dir = Direction.fromString(current.direction);
      if (dir == Direction.unclear) return;
    }

    setState(() => _busy = true);
    try {
      final id = await LooseEndsBridge.confirmDraft(current);
      if (!mounted) return;

      if (id != null && id != 0) {
        setState(() {
          _drafts.removeWhere((d) => d.id == current.id);
        });
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Saved')),
        );
      } else {
        // Never remove a draft after a failed confirmation. The persisted row
        // remains available for retry instead of becoming a "ghost" draft.
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Could not save this draft. It was kept for retry.')),
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
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

    final updatedDescription =
        edited.description.isEmpty ? draft.description : edited.description;
    final saved = await LooseEndsBridge.updateDraft(
      draft,
      description: updatedDescription,
      direction: edited.direction,
      expectedDate: edited.date,
      party: edited.party,
    );
    if (!mounted) return;
    if (!saved) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Could not persist the draft changes.')),
      );
      return;
    }

    setState(() {
      _drafts[index] = Draft(
        id: draft.id,
        description: updatedDescription,
        direction: edited.direction.name,
        expectedDate: edited.date,
        party: edited.party,
        partyConfidence: edited.party == draft.party ? draft.partyConfidence : 'high',
        dateConfidence: edited.date == draft.expectedDate ? draft.dateConfidence : 'high',
        overallConfidence: edited.direction == Direction.unclear ? 'low' : (
          edited.party != draft.party || edited.date != draft.expectedDate
              ? 'high'
              : draft.overallConfidence
        ),
        sourceProvenance: draft.sourceProvenance,
        createdAt: draft.createdAt,
      );
    });
  }

  Future<void> _dismiss(Draft draft) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      final deleted = await LooseEndsBridge.deleteDraft(draft);
      if (!mounted) return;
      if (deleted) {
        setState(() => _drafts.removeWhere((d) => d.id == draft.id));
      } else {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Could not dismiss this draft.')),
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
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
