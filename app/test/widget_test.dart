import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/services.dart';

import 'package:loose_ends/main.dart';
import 'package:loose_ends/screens/home_screen.dart';
import 'package:loose_ends/bridge/loose_ends_bridge.dart';
import 'package:loose_ends/models/direction.dart';
import 'package:loose_ends/models/draft.dart';
import 'package:loose_ends/models/commitment.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('App launches and shows home screen', (WidgetTester tester) async {
    await tester.pumpWidget(const LooseEndsApp(showOnboarding: false));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.byType(HomeScreen), findsOneWidget);
    final appBar = tester.widget<AppBar>(find.byType(AppBar));
    expect(appBar.title, isA<Text>());
    expect((appBar.title! as Text).data, 'Loose Ends');
    expect(find.text('Capture'), findsOneWidget);
    expect(find.text('Review'), findsOneWidget);
    expect(find.text('You Owe'), findsOneWidget);
    expect(find.text('Owed to You'), findsOneWidget);
  });

  group('Direction', () {
    test('displayName matches each enum value', () {
      expect(Direction.userOwes.displayName, 'You Owe');
      expect(Direction.owedToUser.displayName, 'Owed to You');
      expect(Direction.unclear.displayName, 'Unclear');
    });

    test('round-trip: fromString(asString) returns same enum', () {
      for (final d in Direction.values) {
        expect(Direction.fromString(d.name), d);
      }
    });

    test('fromString returns Unclear on unknown input, not null', () {
      expect(Direction.fromString('nonsense'), Direction.unclear);
      expect(Direction.fromString(''), Direction.unclear);
      expect(Direction.fromString('OWED_TO_USER'), Direction.unclear,
          reason: 'matching is case-sensitive on purpose');
    });

    test('direction wire string is the snake_case stored value, not enum identifier', () {
      expect(Direction.userOwes.name, 'user_owes');
      expect(Direction.owedToUser.name, 'owed_to_user');
      expect(Direction.unclear.name, 'unclear');
    });
  });

  group('LooseEndsBridge without native channel', () {
    setUp(() {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(const MethodChannel('com.looseends/core'), null);
    });

    test('init() does not throw when no native side is registered', () async {
      await LooseEndsBridge.init();
    });

    test('ingestText returns no fabricated draft when native side is unavailable',
        () async {
      final drafts = await LooseEndsBridge.ingestText('pay Lena 20 by Friday');
      expect(drafts, isEmpty);
    });

    test('ingestText returns no fabricated draft for long arbitrary text',
        () async {
      final long = 'word ' * 100;
      final drafts = await LooseEndsBridge.ingestText(long);
      expect(drafts, isEmpty);
    });

    test('ingestText returns no fabricated draft for short arbitrary text', () async {
      final drafts = await LooseEndsBridge.ingestText('short');
      expect(drafts, isEmpty);
    });

    test('confirmDraft returns null when not initialized', () async {
      final d = Draft(
        description: 'x',
        direction: 'user_owes',
        expectedDate: null,
        party: null,
        partyConfidence: 'low',
        dateConfidence: 'low',
        overallConfidence: 'low',
      );
      final id = await LooseEndsBridge.confirmDraft(d);
      expect(id, isNull);
    });

    test('confirmDraft honors description/direction/date overrides', () async {
      final d = Draft(
        description: 'orig',
        direction: 'user_owes',
        expectedDate: null,
        party: null,
        partyConfidence: 'low',
        dateConfidence: 'low',
        overallConfidence: 'low',
      );
      final id = await LooseEndsBridge.confirmDraft(
        d,
        descriptionOverride: 'edited',
        directionOverride: Direction.owedToUser,
        dateOverride: '2026-09-01',
      );
      expect(id, isNull);
    });

    test('listOpen returns empty list when not initialized', () async {
      final owed = await LooseEndsBridge.listOpen(Direction.userOwes);
      final owedTo = await LooseEndsBridge.listOpen(Direction.owedToUser);
      expect(owed, isEmpty);
      expect(owedTo, isEmpty);
    });
  });

  group('LooseEndsBridge with mocked native channel', () {
    const channel = MethodChannel('com.looseends/core');

    tearDown(() {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null);
    });

    test('ingestText parses the JSON-shaped response from native', () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
        if (call.method == 'init') return true;
        if (call.method == 'ingestText') {
          return [
            {
              'id': 7,
              'description': 'pay Lena 20',
              'direction': 'user_owes',
              'expected_date': '2026-09-01',
              'party': 'Lena',
              'party_confidence': 'high',
              'date_confidence': 'high',
              'overall_confidence': 'high',
            }
          ];
        }
        return null;
      });

      await LooseEndsBridge.init();
      final drafts = await LooseEndsBridge.ingestText('whatever');
      expect(drafts, hasLength(1));
      expect(drafts.first.id, 7);
      expect(drafts.first.description, 'pay Lena 20');
      expect(drafts.first.direction, 'user_owes');
      expect(drafts.first.expectedDate, '2026-09-01');
      expect(drafts.first.party, 'Lena');
    });

    test('listOpen maps direction strings via Direction.fromString', () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
        if (call.method == 'init') return true;
        if (call.method == 'listOpen') {
          return [
            {
              'id': 1,
              'description': 'pay back Marco',
              'direction': 'user_owes',
              'expected_date': null,
              'party': 'Marco',
              'aging_action': 'surface',
              'created_at': '2026-08-20T10:00:00+00:00',
            },
            {
              'id': 2,
              'description': 'Rosa to return book',
              'direction': 'owed_to_user',
              'expected_date': '2026-09-01',
              'party': 'Rosa',
              'aging_action': 'escalate',
              'created_at': '2026-08-22T10:00:00+00:00',
            },
            {
              'id': 3,
              'description': 'unclear thing',
              'direction': 'bogus_value',
              'expected_date': null,
              'party': null,
              'aging_action': 'archive',
              'created_at': '2026-07-01T10:00:00+00:00',
            },
          ];
        }
        return null;
      });

      await LooseEndsBridge.init();
      final items = await LooseEndsBridge.listOpen(Direction.userOwes);
      expect(items, hasLength(3));
      expect(items[0].direction, Direction.userOwes);
      expect(items[1].direction, Direction.owedToUser);
      expect(items[2].direction, Direction.unclear,
          reason: 'unknown direction strings must NOT throw — must degrade to Unclear');
      expect(items[0].agingAction, 'surface');
      expect(items[1].agingAction, 'escalate');
    });

    test('confirmDraft returns the integer id from native', () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
        if (call.method == 'init') return true;
        if (call.method == 'confirmDraft') return 42;
        return null;
      });
      await LooseEndsBridge.init();
      final d = Draft(
        description: 'x',
        direction: 'user_owes',
        expectedDate: null,
        party: null,
        partyConfidence: 'low',
        dateConfidence: 'low',
        overallConfidence: 'low',
      );
      expect(await LooseEndsBridge.confirmDraft(d), 42);
    });

    test('PlatformException from native returns no fabricated extraction', () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
        if (call.method == 'init') return true;
        throw PlatformException(code: 'NATIVE_CRASH', message: 'boom');
      });
      await LooseEndsBridge.init();
      final drafts = await LooseEndsBridge.ingestText('whatever');
      expect(drafts, isEmpty);
    });
  });

  group('Commitment model', () {
    test('Commitment and CommitmentView are independent value types', () {
      final c = Commitment(
        id: 1,
        description: 'd',
        direction: Direction.userOwes,
        createdAt: '2026-08-26T00:00:00+00:00',
      );
      final v = CommitmentView(
        id: 1,
        description: 'd',
        direction: Direction.userOwes,
        agingAction: 'surface',
        createdAt: '2026-08-26T00:00:00+00:00',
      );
      expect(c.id, v.id);
      expect(c.party, isNull);
      expect(c.expectedDate, isNull);
      expect(v.expectedDate, isNull);
      expect(v.agingAction, 'surface');
    });
  });
}
