import 'package:flutter/material.dart';
import '../bridge/loose_ends_bridge.dart';
import 'model_manager_screen.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  Map<String, dynamic> _status = const {};

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    final status = await LooseEndsBridge.modelStatus();
    if (mounted) setState(() => _status = status);
  }

  @override
  Widget build(BuildContext context) {
    final selected = _status['selectedModelId']?.toString();
    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Card(
            child: ListTile(
              leading: const Icon(Icons.memory),
              title: const Text('AI model'),
              subtitle: Text(
                selected == null
                    ? 'No model selected — rules-only mode is active'
                    : 'Selected: ' + selected,
              ),
              trailing: const Icon(Icons.chevron_right),
              onTap: () async {
                await Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => const ModelManagerScreen()),
                );
                await _refresh();
              },
            ),
          ),
          const SizedBox(height: 12),
          const Card(
            child: ListTile(
              leading: Icon(Icons.lock_outline),
              title: Text('Privacy'),
              subtitle: Text(
                'Commitments are processed locally. Model downloads are separate from your commitment data and never upload it.',
              ),
            ),
          ),
        ],
      ),
    );
  }
}
