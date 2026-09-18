import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import '../models/commitment.dart';
import '../models/direction.dart';
import '../models/draft.dart';
import 'loose_ends_bridge_linux.dart';

/// Bridge to the Rust core.
///
/// On Linux desktop, uses dart:ffi to call the native shared library directly.
/// On Android, uses the MethodChannel/JNI bridge.
class LooseEndsBridge {
  static const _channel = MethodChannel('com.looseends/core');
  static const _modelProgressChannel = EventChannel('com.looseends/model_progress');
  static Stream<Map<String, dynamic>> get modelProgress =>
      _modelProgressChannel.receiveBroadcastStream().map(
        (event) => event is Map
            ? event.map((key, value) => MapEntry(key.toString(), value))
            : <String, dynamic>{},
      );
  static bool _initialized = false;
  static bool _channelAvailable = false;

  static Future<void> init() async {
    if (_initialized) return;

    // Prefer the platform channel. This is the Android path and also makes
    // the bridge testable on Linux without requiring a native .so in tests.
    try {
      final initialized = await _channel.invokeMethod<bool>('init') ?? false;
      if (initialized) {
        _channelAvailable = true;
        _initialized = true;
        return;
      }
    } on PlatformException catch (e) {
      debugPrint('Bridge init failed: ${e.message}');
    } on MissingPluginException {
      // Desktop builds can fall through to the FFI implementation below.
    }

    if (Platform.isLinux) {
      try {
        await LooseEndsBridgeLinux.init();
        _initialized = true;
        _channelAvailable = false;
      } catch (e) {
        debugPrint('Linux native bridge unavailable; running in stub mode: $e');
      }
    } else {
      debugPrint('Native bridge unavailable; running in stub mode');
    }
  }

  static Future<List<Draft>> ingestText(String text) async {
    if (!_initialized) return const [];
    if (Platform.isLinux && !_channelAvailable) {
      final maps = LooseEndsBridgeLinux.ingestText(text);
      return maps.map((m) => Draft(
            id: m['id'] as int? ?? 0,
            description: m['description'] as String,
            direction: m['direction'] as String,
            expectedDate: m['expected_date'] as String?,
            party: m['party'] as String?,
            partyConfidence: m['party_confidence'] as String,
            dateConfidence: m['date_confidence'] as String,
            overallConfidence: m['overall_confidence'] as String,
          )).toList();
    }

    try {
      final result = await _channel.invokeMethod('ingestText', {'text': text});
      final list = result is List ? result : const [];
      return list.cast<Map>().map((m) => Draft(
            id: m['id'] as int? ?? 0,
            description: m['description'] as String,
            direction: m['direction'] as String,
            expectedDate: m['expected_date'] as String?,
            party: m['party'] as String?,
            partyConfidence: m['party_confidence'] as String,
            dateConfidence: m['date_confidence'] as String,
            overallConfidence: m['overall_confidence'] as String,
          )).toList();
    } on PlatformException {
      return const [];
    } on MissingPluginException {
      return const [];
    }
  }

  static Future<int?> confirmDraft(
    Draft draft, {
    String? descriptionOverride,
    Direction? directionOverride,
    String? dateOverride,
    String? partyOverride,
  }) async {
    if (!_initialized) return null;
    if (Platform.isLinux && !_channelAvailable) {
      return LooseEndsBridgeLinux.confirmDraft(
        <String, dynamic>{
          'id': draft.id,
          'description': draft.description,
          'direction': draft.direction,
          'expected_date': draft.expectedDate,
          'party': partyOverride ?? draft.party,
        },
        descriptionOverride: descriptionOverride,
        directionOverride: directionOverride?.name,
        dateOverride: dateOverride,
      );
    }

    try {
      final result = await _channel.invokeMethod('confirmDraft', {
        'draftId': draft.id,
        'description': descriptionOverride ?? draft.description,
        'direction': (directionOverride ?? Direction.fromString(draft.direction)).name,
        'expected_date': dateOverride ?? draft.expectedDate,
        'party': partyOverride ?? draft.party,
      });
      return result as int?;
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  static Future<List<Draft>> listDrafts() async {
    if (!_initialized) return const [];
    try {
      final result = await _channel.invokeMethod('listDrafts');
      final list = result is List ? result : const [];
      return list.cast<Map>().map((m) => Draft(
        id: (m['id'] as num?)?.toInt() ?? 0,
        description: m['description'] as String,
        direction: m['direction'] as String,
        expectedDate: m['expected_date'] as String?,
        party: m['party'] as String?,
        partyConfidence: m['party_confidence'] as String? ?? 'low',
        dateConfidence: m['date_confidence'] as String? ?? 'low',
        overallConfidence: m['overall_confidence'] as String? ?? 'low',
        sourceProvenance: m['source_provenance'] as String? ?? 'rule_extracted',
      )).toList();
    } on PlatformException {
      return const [];
    } on MissingPluginException {
      return const [];
    }
  }

  static Future<List<Map<String, dynamic>>> modelCatalog() async {
    if (!_initialized) return const [];
    try {
      final result = await _channel.invokeMethod('models');
      final list = result is List ? result : const [];
      return list.cast<Map>().map(
        (m) => m.map((key, value) => MapEntry(key.toString(), value)),
      ).toList();
    } on PlatformException {
      return const [];
    } on MissingPluginException {
      return const [];
    }
  }

  static Future<Map<String, dynamic>> modelStatus() async {
    if (!_initialized) return const {};
    try {
      final result = await _channel.invokeMethod('modelStatus');
      return result is Map
          ? result.map((key, value) => MapEntry(key.toString(), value))
          : const {};
    } on PlatformException {
      return const {};
    } on MissingPluginException {
      return const {};
    }
  }

  static Future<String?> downloadModel(
    String modelId, {
    bool allowMobile = false,
  }) async {
    if (!_initialized) return 'Native bridge unavailable.';
    try {
      final result = await _channel.invokeMethod<Map>('startModelDownload', {
        'modelId': modelId,
        'allowMobile': allowMobile,
      });
      if (result == null) return 'Model download failed.';
      return result['ok'] == true ? null : result['message']?.toString();
    } on PlatformException catch (e) {
      return e.message ?? 'Model download failed.';
    } on MissingPluginException {
      return 'Native bridge unavailable.';
    }
  }

  static Future<void> cancelModelDownload() async {
    if (!_initialized) return;
    try {
      await _channel.invokeMethod('cancelModelDownload');
    } on PlatformException {
      // The native side also tears down the partial file on cancellation.
    } on MissingPluginException {
      // No native side.
    }
  }

  static Future<bool> selectModel(String modelId) async {
    if (!_initialized) return false;
    try {
      return await _channel.invokeMethod<bool>('selectModel', {'modelId': modelId}) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<bool> deleteModel(String modelId) async {
    if (!_initialized) return false;
    try {
      return await _channel.invokeMethod<bool>('deleteModel', {'modelId': modelId}) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<Map<String, dynamic>?> pickCustomGguf() async {
    if (!_initialized) return null;
    try {
      final result = await _channel.invokeMethod('pickCustomGguf');
      return result is Map
          ? result.map((key, value) => MapEntry(key.toString(), value))
          : null;
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  static Future<bool> markOnboardingComplete() async {
    if (!_initialized) return false;
    try {
      return await _channel.invokeMethod<bool>('markOnboardingComplete') ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<bool> shouldShowOnboarding() async {
    if (!_initialized) return false;
    try {
      return await _channel.invokeMethod<bool>('shouldShowOnboarding') ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<String?> pickModel() async {
    if (!_initialized || Platform.isLinux && !_channelAvailable) return null;
    try {
      return await _channel.invokeMethod<String>('pickModel');
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  static Future<Map<String, dynamic>> modelStatus() async {
    if (!_initialized || Platform.isLinux && !_channelAvailable) {
      return const {'configured': false};
    }
    try {
      final result = await _channel.invokeMethod('modelStatus');
      return result is Map
          ? result.map((key, value) => MapEntry(key.toString(), value))
          : const {'configured': false};
    } on PlatformException {
      return const {'configured': false};
    } on MissingPluginException {
      return const {'configured': false};
    }
  }

  static Future<bool> scheduleReminderForDate({
    required int id,
    required String description,
    String? expectedDate,
  }) async {
    if (!_initialized || Platform.isLinux && !_channelAvailable || expectedDate == null) {
      return false;
    }
    final due = DateTime.tryParse(expectedDate);
    if (due == null) return false;
    var trigger = DateTime(due.year, due.month, due.day, 9);
    if (!trigger.isAfter(DateTime.now())) {
      return false;
    }
    try {
      return await _channel.invokeMethod<bool>('scheduleReminder', {
        'id': id,
        'title': 'Loose Ends reminder',
        'text': description,
        'triggerAtMillis': trigger.millisecondsSinceEpoch,
      }) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<bool> cancelReminder(int id) async {
    if (!_initialized || Platform.isLinux && !_channelAvailable) return false;
    try {
      return await _channel.invokeMethod<bool>('cancelReminder', {'id': id}) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<int?> createCommitment({
    required String description,
    required Direction direction,
    String? expectedDate,
    String? party,
  }) async {
    if (!_initialized || direction == Direction.unclear) return null;
    try {
      final result = await _channel.invokeMethod<int>('createCommitment', {
        'description': description,
        'direction': direction.name,
        'expected_date': expectedDate,
        'party': party,
      });
      return result;
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  static Future<bool> resolveCommitment(int id, {String? note}) async {
    if (!_initialized) return false;
    try {
      return await _channel.invokeMethod<bool>('resolveCommitment', {
        'id': id,
        'note': note,
      }) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<bool> snoozeCommitment(int id) async {
    if (!_initialized) return false;
    try {
      return await _channel.invokeMethod<bool>('snoozeCommitment', {'id': id}) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<List<CommitmentView>> listOpen(Direction dir) async {
    if (!_initialized) return [];
    if (Platform.isLinux && !_channelAvailable) {
      final maps = LooseEndsBridgeLinux.listOpen(dir.name);
      return maps.map((m) => CommitmentView(
            id: m['id'] as int,
            description: m['description'] as String,
            direction: Direction.fromString(m['direction'] as String),
            expectedDate: m['expected_date'] as String?,
            party: m['party'] as String?,
            agingAction: m['aging_action'] as String,
            createdAt: m['created_at'] as String,
          )).toList();
    }

    try {
      final result = await _channel.invokeMethod('listOpen', {'direction': dir.name});
      final list = result is List ? result : const [];
      return list.cast<Map>().map((m) => CommitmentView(
            id: m['id'] as int,
            description: m['description'] as String,
            direction: Direction.fromString(m['direction'] as String),
            expectedDate: m['expected_date'] as String?,
            party: m['party'] as String?,
            agingAction: m['aging_action'] as String,
            createdAt: m['created_at'] as String,
          )).toList();
    } on PlatformException {
      return [];
    } on MissingPluginException {
      return [];
    }
  }

}
