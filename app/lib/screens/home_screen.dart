import 'package:flutter/material.dart';
import 'capture_screen.dart';
import 'manual_entry_screen.dart';
import 'review_screen.dart';
import 'owe_you_screen.dart';
import 'owed_to_you_screen.dart';
import 'settings_screen.dart';
import '../models/direction.dart';
import '../bridge/loose_ends_bridge.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  int _pendingDrafts = 0;

  @override
  void initState() {
    super.initState();
    _refreshDraftCount();
  }

  Future<void> _refreshDraftCount() async {
    final drafts = await LooseEndsBridge.listDrafts();
    if (mounted) {
      setState(() => _pendingDrafts = drafts.length);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Intellex'),
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Card(
            color: Theme.of(context).colorScheme.primaryContainer,
            child: ListTile(
              leading: const Icon(Icons.add_circle, size: 36),
              title: const Text('Capture'),
              subtitle: const Text('Type, paste, or dictate a new commitment'),
              onTap: () async {
                await Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => const CaptureScreen()),
                );
              },
            ),
          ),
          const SizedBox(height: 8),
          Card(
            child: ListTile(
              leading: const Icon(Icons.edit_note, size: 36),
              title: const Text('Manual entry'),
              subtitle: const Text('Add a commitment directly'),
              onTap: () async {
                await Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => const ManualEntryScreen()),
                );
              },
            ),
          ),
          const SizedBox(height: 8),
          Card(
            child: ListTile(
              leading: const Icon(Icons.inbox, size: 36),
              title: const Text('Review'),
              subtitle: Text(_pendingDrafts > 0
                  ? '$_pendingDrafts pending draft(s)'
                  : 'No drafts to review'),
              onTap: () async {
                await Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => const ReviewScreen()),
                );
                await _refreshDraftCount();
              },
            ),
          ),
          const SizedBox(height: 8),
          Card(
            child: ListTile(
              leading: const Icon(Icons.outbond, size: 36),
              title: const Text('You Owe'),
              subtitle: const Text('Things you committed to do'),
              onTap: () {
                Navigator.push(
                  context,
                  MaterialPageRoute(
                    builder: (_) => const OweYouScreen(direction: Direction.userOwes),
                  ),
                );
              },
            ),
          ),
          Card(
            child: ListTile(
              leading: const Icon(Icons.call_received, size: 36),
              title: const Text('Owed to You'),
              subtitle: const Text('Things others committed to do'),
              onTap: () {
                Navigator.push(
                  context,
                  MaterialPageRoute(
                    builder: (_) => const OwedToYouScreen(),
                  ),
                );
              },
            ),
          ),
          const SizedBox(height: 8),
          Card(
            child: ListTile(
              leading: const Icon(Icons.settings, size: 32),
              title: const Text('Settings'),
              subtitle: const Text('AI model, privacy, and local app controls'),
              onTap: () {
                Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => const SettingsScreen()),
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
