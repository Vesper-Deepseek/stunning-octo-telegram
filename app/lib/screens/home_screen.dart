import 'package:flutter/material.dart';
import 'capture_screen.dart';
import 'review_screen.dart';
import 'owe_you_screen.dart';
import 'owed_to_you_screen.dart';
import '../models/direction.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  final int _pendingDrafts = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Loose Ends'),
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
        ],
      ),
    );
  }
}
