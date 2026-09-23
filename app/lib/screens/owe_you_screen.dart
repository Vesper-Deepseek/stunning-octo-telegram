import 'package:flutter/material.dart';
import '../bridge/loose_ends_bridge.dart';
import '../models/commitment.dart';
import '../models/direction.dart';

class OweYouScreen extends StatefulWidget {
  final Direction direction;
  const OweYouScreen({super.key, required this.direction});

  @override
  State<OweYouScreen> createState() => _OweYouScreenState();
}

class _OweYouScreenState extends State<OweYouScreen> {
  late Future<List<CommitmentView>> _future;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  void _reload() {
    _future = LooseEndsBridge.listOpen(widget.direction);
  }

  Future<void> _setReminder(CommitmentView c) async {
    final ok = await LooseEndsBridge.scheduleReminderForDate(
      id: c.id,
      description: c.description,
      expectedDate: c.expectedDate,
    );
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          ok
              ? 'Reminder scheduled.'
              : c.expectedDate == null
                  ? 'Add a due date before scheduling a reminder.'
                  : 'Could not schedule reminder.',
        ),
      ),
    );
  }

  Future<void> _resolve(CommitmentView c) async {
    final ok = await LooseEndsBridge.resolveCommitment(c.id);
    if (!mounted) return;
    if (ok) {
      await LooseEndsBridge.cancelReminder(c.id);
      if (!mounted) return;
      setState(_reload);
    }
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(ok ? 'Commitment resolved.' : 'Could not resolve commitment.')),
    );
  }

  Future<void> _snooze(CommitmentView c) async {
    final ok = await LooseEndsBridge.snoozeCommitment(c.id);
    if (!mounted) return;
    if (ok) setState(_reload);
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(ok ? 'Commitment snoozed.' : 'Could not snooze commitment.')),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.direction.displayName),
      ),
      body: FutureBuilder<List<CommitmentView>>(
        future: _future,
        builder: (context, snap) {
          if (snap.connectionState != ConnectionState.done) {
            return const Center(child: CircularProgressIndicator());
          }
          final items = snap.data ?? [];
          if (items.isEmpty) {
            return const Center(child: Text('Nothing here yet.'));
          }
          return ListView.builder(
            itemCount: items.length,
            itemBuilder: (_, i) {
              final c = items[i];
              return Card(
                margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                child: ListTile(
                  title: Text(c.description),
                  subtitle: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      if (c.party != null) Text('Party: ${c.party}'),
                      if (c.expectedDate != null) Text('Due: ${c.expectedDate}'),
                      Text(
                        'Action: ${c.agingAction}',
                        style: const TextStyle(fontSize: 12, color: Colors.grey),
                      ),
                    ],
                  ),
                  trailing: Wrap(
                    spacing: 0,
                    children: [
                      IconButton(
                        tooltip: 'Set reminder',
                        onPressed: () => _setReminder(c),
                        icon: const Icon(Icons.notifications_none),
                      ),
                      IconButton(
                        tooltip: 'Snooze',
                        onPressed: () => _snooze(c),
                        icon: const Icon(Icons.snooze),
                      ),
                      IconButton(
                        tooltip: 'Resolve',
                        onPressed: () => _resolve(c),
                        icon: const Icon(Icons.check_circle_outline),
                      ),
                    ],
                  ),
                ),
              );
            },
          );
        },
      ),
    );
  }
}
