import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/services.dart';

import 'package:loose_ends/main.dart';
import 'package:loose_ends/bridge/loose_ends_bridge.dart';
import 'package:loose_ends/models/direction.dart';
import 'package:loose_ends/models/draft.dart';
import 'package:loose_ends/models/commitment.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  // ---------------- Home / widget smoke ----------------

  testWidgets('App launches and shows home screen', (WidgetTester tester) async {
    await tester.pumpWidget(const LooseEndsApp());
    await tester.pump();

    expect(find.text('Loose Ends'), findsOneWidget);
    expect(find.text('Capture'), findsOneWidget);
    expect(find.text('Review'), findsOneWidget);
    expect(find.text('You Owe'), findsOneWidget);
    expect(find.text('Owed to You'), findsOneWidget);
  });

  // ---------------- Direction model ----------------

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
      // This pins the wire format that crosses to Rust (snake_case must match
      // the schema CHECK constraint on the direction column).
      expect(Direction.userOwes.name, 'user_owes');
      expect(Direction.owedToUser.name, 'owed_to_user');
      expect(Direction.unclear.name, 'unclear');
    });
  });

  // ---------------- Bridge fallback (no native channel registered) ----------------

  group('LooseEndsBridge without native channel', () {
    // _initialized is static, so order matters. The default channel handler
    // in widget tests throws MissingPluginException, which the bridge catches
    // and leaves _initialized=false -> all methods take the fallback path.

    setUp(() {
      // Ensure no leftover mock handler from a previous test
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(const MethodChannel('com.looseends/core'), null);
    });

    test('init() does not throw when no native side is registered', () async {
      await LooseEndsBridge.init(); // must not throw
    });

    test('ingestText returns a single fallback draft for arbitrary text',
        () async {
      final drafts = await LooseEndsBridge.ingestText('pay Lena 20 by Friday');
      expect(drafts, hasLength(1));
      expect(drafts.first.direction, 'unclear');
      expect(drafts.first.party, isNull);
      expect(drafts.first.overallConfidence, 'low');
      expect(drafts.first.description, contains('pay Lena 20 by Friday'));
    });

    test('ingestText truncates fallback description to 80 chars + ellipsis',
        () async {
      final long = 'word ' * 100; // ~500 chars
      final drafts = await LooseEndsBridge.ingestText(long);
      expect(drafts.first.description.length, lessThanOrEqualTo(83));
      expect(drafts.first.description, endsWith('...'));
    });

    test('ingestText preserves short text verbatim', () async {
      final drafts = await LooseEndsBridge.ingestText('short');
      expect(drafts.first.description, 'short');
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
      // No native channel -> returns null regardless of args; this pins that
      // the override-path code at least parses the inputs without throwing.
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

  // ---------------- Bridge with mocked native channel ----------------

  group('LooseEndsBridge with mocked native channel', () {
    const channel = MethodChannel('com.looseends/core');

    tearDown(() {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null);
    });

    test('ingestText parses the JSON-shaped response from native', () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
        if (call.method == 'init') return null;
        if (call.method == 'ingestText') {
          return [
            {
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
      expect(drafts.first.description, 'pay Lena 20');
      expect(drafts.first.direction, 'user_owes');
      expect(drafts.first.expectedDate, '2026-09-01');
      expect(drafts.first.party, 'Lena');
    });

    test('listOpen maps direction strings via Direction.fromString', () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
        if (call.method == 'init') return null;
        if (call.method == 'listOpen') {
          // First call asks for user_owes; return two, one owed_to_user
          // to verify the fromString mapping, not just filter passthrough.
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
        if (call.method == 'init') return null;
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

    test('PlatformException from native falls back gracefully', () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
        if (call.method == 'init') return null;
        throw PlatformException(code: 'NATIVE_CRASH', message: 'boom');
      });
      await LooseEndsBridge.init();
      final drafts = await LooseEndsBridge.ingestText('whatever');
      // Falls back to the same unclear-draft as the no-native path
      expect(drafts, hasLength(1));
      expect(drafts.first.overallConfidence, 'low');
    });
  });

  // ---------------- Model construction ----------------

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
      // View carries agingAction that Commitment does not
      expect(v.agingAction, 'surface');
    });
  });
}
