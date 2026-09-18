import 'package:flutter/material.dart';
import '../bridge/loose_ends_bridge.dart';
import '../models/direction.dart';

class ManualEntryScreen extends StatefulWidget {
  const ManualEntryScreen({super.key});

  @override
  State<ManualEntryScreen> createState() => _ManualEntryScreenState();
}

class _ManualEntryScreenState extends State<ManualEntryScreen> {
  final _description = TextEditingController();
  final _party = TextEditingController();
  final _date = TextEditingController();
  Direction _direction = Direction.userOwes;
  bool _saving = false;

  @override
  void dispose() {
    _description.dispose();
    _party.dispose();
    _date.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final description = _description.text.trim();
    if (description.isEmpty) return;
    setState(() => _saving = true);
    final id = await LooseEndsBridge.createCommitment(
      description: description,
      direction: _direction,
      expectedDate: _date.text.trim().isEmpty ? null : _date.text.trim(),
      party: _party.text.trim().isEmpty ? null : _party.text.trim(),
    );
    if (!mounted) return;
    setState(() => _saving = false);
    if (id != null && id > 0) {
      Navigator.pop(context, true);
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Could not save commitment.')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Manual entry')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          TextField(
            controller: _description,
            maxLines: 3,
            decoration: const InputDecoration(
              labelText: 'What is the commitment?',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 12),
          DropdownButtonFormField<Direction>(
            initialValue: _direction,
            decoration: const InputDecoration(
              labelText: 'Direction',
              border: OutlineInputBorder(),
            ),
            items: const [
              DropdownMenuItem(
                value: Direction.userOwes,
                child: Text('You Owe'),
              ),
              DropdownMenuItem(
                value: Direction.owedToUser,
                child: Text('Owed to You'),
              ),
            ],
            onChanged: (value) {
              if (value != null) setState(() => _direction = value);
            },
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _party,
            decoration: const InputDecoration(
              labelText: 'Person / organization (optional)',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _date,
            keyboardType: TextInputType.datetime,
            decoration: const InputDecoration(
              labelText: 'Expected date YYYY-MM-DD (optional)',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 20),
          FilledButton.icon(
            onPressed: _saving ? null : _save,
            icon: const Icon(Icons.save),
            label: Text(_saving ? 'Saving…' : 'Save commitment'),
          ),
          const SizedBox(height: 8),
          const Text(
            'Manual entries are stored locally in the Rust/SQLite core.',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 12),
          ),
        ],
      ),
    );
  }
}
