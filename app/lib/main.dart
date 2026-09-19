import 'package:flutter/material.dart';
import 'screens/home_screen.dart';
import 'screens/onboarding_screen.dart';
import 'bridge/loose_ends_bridge.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await LooseEndsBridge.init();
  final showOnboarding = await LooseEndsBridge.shouldShowOnboarding();
  runApp(LooseEndsApp(showOnboarding: showOnboarding));
}

class LooseEndsApp extends StatelessWidget {
  final bool showOnboarding;

  const LooseEndsApp({
    super.key,
    required this.showOnboarding,
  });

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Loose Ends',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
        useMaterial3: true,
      ),
      home: showOnboarding ? const OnboardingScreen() : const HomeScreen(),
    );
  }
}
