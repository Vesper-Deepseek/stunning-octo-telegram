import 'package:flutter/material.dart';
import '../models/direction.dart';
import 'owe_you_screen.dart';

class OwedToYouScreen extends StatelessWidget {
  const OwedToYouScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return OweYouScreen(direction: Direction.owedToUser);
  }
}
