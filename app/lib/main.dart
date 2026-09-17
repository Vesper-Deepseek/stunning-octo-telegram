import 'package:flutter/material.dart';
import 'screens/home_screen.dart';
import 'bridge/loose_ends_bridge.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await LooseEndsBridge.init();
  runApp(const LooseEndsApp());
}

class LooseEndsApp extends StatelessWidget {
  const LooseEndsApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Loose Ends',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
        useMaterial3: true,
      ),
      home: const HomeScreen(),
    );
  }
}
