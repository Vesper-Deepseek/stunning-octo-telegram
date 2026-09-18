import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'package:loose_ends/main.dart' as app;
import 'package:loose_ends/bridge/loose_ends_bridge.dart';
import 'package:loose_ends/models/direction.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('MVP end-to-end Android flow', (WidgetTester tester) async {
    await app.main();
    await tester.pumpAndSettle(const Duration(seconds: 3));

    expect(find.text('Loose Ends'), findsOneWidget);
    if (find.text('Skip for now').evaluate().isNotEmpty) {
      await tester.tap(find.text('Skip for now'));
      await tester.pumpAndSettle();
    }

    expect(find.text('Capture'), findsOneWidget);
    expect(find.text('Review'), findsOneWidget);
    expect(find.text('You Owe'), findsOneWidget);
    expect(find.text('Owed to You'), findsOneWidget);
    expect(find.text('Settings'), findsOneWidget);

    await tester.tap(find.text('Manual entry'));
    await tester.pumpAndSettle();
    final manualFields = find.byType(TextField);
    expect(manualFields, findsNWidgets(3));
    await tester.enterText(manualFields.at(0), 'Rosa will send the photos');
    await tester.tap(find.byType(DropdownButtonFormField<Direction>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Owed to You').last);
    await tester.pumpAndSettle();
    await tester.enterText(manualFields.at(1), 'Rosa');
    await tester.enterText(manualFields.at(2), '2030-01-01');
    await tester.tap(find.text('Save commitment'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Owed to You'));
    await tester.pumpAndSettle();
    expect(find.text('Rosa will send the photos'), findsOneWidget);
    expect(find.text('Party: Rosa'), findsOneWidget);
    expect(find.text('Due: 2030-01-01'), findsOneWidget);
    expect(find.byTooltip('Set reminder'), findsOneWidget);
    await tester.tap(find.byTooltip('Set reminder'));
    await tester.pumpAndSettle();
    expect(find.text('Reminder scheduled.'), findsOneWidget);
    await tester.tap(find.byTooltip('Resolve'));
    await tester.pumpAndSettle();
    expect(find.text('Rosa will send the photos'), findsNothing);
    await tester.pageBack();
    await tester.pumpAndSettle();

    await tester.tap(find.text('Capture'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byType(TextField),
      'I need to pay Aisha 25 by 2030-01-02',
    );
    await tester.tap(find.text('Extract'));
    await tester.pumpAndSettle(const Duration(seconds: 2));
    expect(find.text('Review'), findsOneWidget);
    expect(find.textContaining('I will pay Aisha 25'), findsOneWidget);
    await tester.tap(find.text('Edit').first);
    await tester.pumpAndSettle();
    final editFields = find.byType(TextField);
    expect(editFields, findsNWidgets(3));
    await tester.enterText(editFields.at(0), 'Pay Aisha 30');
    await tester.tap(find.text('Apply'));
    await tester.pumpAndSettle();
    expect(find.text('Pay Aisha 30'), findsOneWidget);
    await tester.tap(find.text('Confirm').first);
    await tester.pumpAndSettle();
    expect(find.text('Saved'), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    await tester.tap(find.text('You Owe'));
    await tester.pumpAndSettle();
    expect(find.text('Pay Aisha 30'), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    final snoozeId = await LooseEndsBridge.createCommitment(
      description: 'Snooze this commitment',
      direction: Direction.userOwes,
      expectedDate: '2030-01-03',
      party: 'Marco',
    );
    expect(snoozeId, greaterThan(0));

    await tester.tap(find.text('You Owe'));
    await tester.pumpAndSettle();
    expect(find.text('Snooze this commitment'), findsOneWidget);
    await tester.tap(find.byTooltip('Snooze').first);
    await tester.pumpAndSettle();
    expect(find.text('Commitment snoozed.'), findsOneWidget);
    expect(find.text('Snooze this commitment'), findsNothing);
    await tester.pageBack();
    await tester.pumpAndSettle();

    await tester.tap(find.text('Capture'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'I need to call Daniel tomorrow');
    await tester.tap(find.text('Extract'));
    await tester.pumpAndSettle(const Duration(seconds: 2));
    expect(find.textContaining('I will call Daniel'), findsOneWidget);
    await tester.tap(find.text('Dismiss').first);
    await tester.pumpAndSettle();
    expect(find.text('No drafts to review'), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    await tester.tap(find.text('Settings'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('AI model'));
    await tester.pumpAndSettle();
    expect(find.text('Qwen 2.5 1.5B Instruct'), findsOneWidget);
    expect(find.text('MiniCPM5 2B'), findsOneWidget);
    final mobileToggle = find.byType(SwitchListTile);
    expect(mobileToggle, findsOneWidget);
    expect(tester.widget<SwitchListTile>(mobileToggle).value, isFalse);
    await tester.tap(mobileToggle);
    await tester.pumpAndSettle();
    expect(tester.widget<SwitchListTile>(mobileToggle).value, isTrue);
  });
}
