import 'package:flutter/material.dart';
import '../bridge/loose_ends_bridge.dart';
import 'model_manager_screen.dart';
import 'home_screen.dart';

class OnboardingScreen extends StatelessWidget {
  const OnboardingScreen({super.key});

  Future<void> _finish(BuildContext context) async {
    await LooseEndsBridge.markOnboardingComplete();
    if (!context.mounted) return;
    Navigator.of(context).pushReplacement(
      MaterialPageRoute(builder: (_) => const HomeScreen()),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.link_off, size: 72),
              const SizedBox(height: 20),
              Text(
                'Intellex',
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: 12),
              const Text(
                'Track commitments without an account or cloud service. '
                'AI extraction is optional and stays on this device.',
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 24),
              FilledButton.icon(
                onPressed: () async {
                  await Navigator.push(
                    context,
                    MaterialPageRoute(builder: (_) => const ModelManagerScreen()),
                  );
                  if (context.mounted) await _finish(context);
                },
                icon: const Icon(Icons.download),
                label: const Text('Set up on-device AI'),
              ),
              const SizedBox(height: 12),
              OutlinedButton(
                onPressed: () => _finish(context),
                child: const Text('Skip for now'),
              ),
              const SizedBox(height: 16),
              const Text(
                'You can add or change models later in Settings.',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 12),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
