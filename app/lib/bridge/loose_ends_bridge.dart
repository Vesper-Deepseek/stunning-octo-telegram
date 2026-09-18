import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import '../models/commitment.dart';
import '../models/direction.dart';
import '../models/draft.dart';
import 'loose_ends_bridge_linux.dart';

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
    try {
      final ok = await _channel.invokeMethod<bool>('init') ?? false;
      if (ok) {
        _channelAvailable = true;
        _initialized = true;
        return;
      }
    } on PlatformException catch (e) {
      debugPrint('Bridge init failed: ' + e.message.toString());
    } on MissingPluginException {
      // Linux desktop can fall through to direct FFI.
    }
    if (Platform.isLinux) {
      try {
        await LooseEndsBridgeLinux.init();
        _initialized = true;
      } catch (e) {
        debugPrint('Linux native bridge unavailable: ' + e.toString());
      }
    }
  }

  static Draft _draftFromMap(Map map) {
    return Draft(
      id: (map['id'] as num?)?.toInt() ?? 0,
      description: map['description'] as String? ?? '',
      direction: map['direction'] as String? ?? 'unclear',
      expectedDate: map['expected_date'] as String?,
      party: map['party'] as String?,
      partyConfidence: map['party_confidence'] as String? ?? 'low',
      dateConfidence: map['date_confidence'] as String? ?? 'low',
      overallConfidence: map['overall_confidence'] as String? ?? 'low',
      sourceProvenance: map['source_provenance'] as String? ?? 'rule_extracted',
      createdAt: map['created_at'] as String?,
    );
  }

  static Future<List<Draft>> ingestText(String text) async {
    if (!_initialized) return const [];
    if (Platform.isLinux && !_channelAvailable) {
      return LooseEndsBridgeLinux.ingestText(text).map(_draftFromMap).toList();
    }
    try {
      final result = await _channel.invokeMethod('ingestText', {'text': text});
      final list = result is List ? result : const [];
      return list.cast<Map>().map(_draftFromMap).toList();
    } on PlatformException catch (e) {
      debugPrint('Text extraction failed: ' + e.message.toString());
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
        {
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
      return (result as num?)?.toInt();
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  static Future<List<Draft>> listDrafts() async {
    if (!_initialized || !_channelAvailable) return const [];
    try {
      final result = await _channel.invokeMethod('listDrafts');
      final list = result is List ? result : const [];
      return list.cast<Map>().map(_draftFromMap).toList();
    } on PlatformException {
      return const [];
    } on MissingPluginException {
      return const [];
    }
  }

  static Future<List<CommitmentView>> listOpen(Direction dir) async {
    if (!_initialized) return const [];
    if (Platform.isLinux && !_channelAvailable) {
      return LooseEndsBridgeLinux.listOpen(dir.name).map(
        (m) => CommitmentView(
          id: m['id'] as int,
          description: m['description'] as String,
          direction: Direction.fromString(m['direction'] as String),
          expectedDate: m['expected_date'] as String?,
          party: m['party'] as String?,
          agingAction: m['aging_action'] as String,
          createdAt: m['created_at'] as String,
        ),
      ).toList();
    }
    try {
      final result = await _channel.invokeMethod('listOpen', {'direction': dir.name});
      final list = result is List ? result : const [];
      return list.cast<Map>().map(
        (m) => CommitmentView(
          id: (m['id'] as num).toInt(),
          description: m['description'] as String,
          direction: Direction.fromString(m['direction'] as String),
          expectedDate: m['expected_date'] as String?,
          party: m['party'] as String?,
          agingAction: m['aging_action'] as String,
          createdAt: m['created_at'] as String,
        ),
      ).toList();
    } on PlatformException {
      return const [];
    } on MissingPluginException {
      return const [];
    }
  }

  static Future<int?> createCommitment({
    required String description,
    required Direction direction,
    String? expectedDate,
    String? party,
  }) async {
    if (!_initialized || direction == Direction.unclear) return null;
    if (Platform.isLinux && !_channelAvailable) return null;
    try {
      return await _channel.invokeMethod<int>('createCommitment', {
        'description': description,
        'direction': direction.name,
        'expected_date': expectedDate,
        'party': party,
      });
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  static Future<bool> resolveCommitment(int id, {String? note}) async {
    if (!_initialized) return false;
    try {
      return await _channel.invokeMethod<bool>(
        'resolveCommitment',
        {'id': id, 'note': note},
      ) ?? false;
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

  static Future<List<Map<String, dynamic>>> modelCatalog() async {
    if (!_initialized || !_channelAvailable) return const [];
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
    if (!_initialized || !_channelAvailable) return const {};
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

  static Future<String?> downloadModel(String modelId, {bool allowMobile = false}) async {
    if (!_initialized || !_channelAvailable) return 'Native model manager unavailable.';
    try {
      final result = await _channel.invokeMethod<Map>('startModelDownload', {
        'modelId': modelId,
        'allowMobile': allowMobile,
      });
      if (result == null) return 'Model download failed.';
      return result['ok'] == true
          ? null
          : result['message']?.toString() ?? 'Model download failed.';
    } on PlatformException catch (e) {
      return e.message ?? 'Model download failed.';
    } on MissingPluginException {
      return 'Native model manager unavailable.';
    }
  }

  static Future<void> cancelModelDownload() async {
    if (!_initialized || !_channelAvailable) return;
    try {
      await _channel.invokeMethod('cancelModelDownload');
    } on PlatformException {
      // Native side removes the partial file.
    } on MissingPluginException {}
  }

  static Future<bool> selectModel(String modelId) async {
    if (!_initialized || !_channelAvailable) return false;
    try {
      return await _channel.invokeMethod<bool>('selectModel', {'modelId': modelId}) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<bool> deleteModel(String modelId) async {
    if (!_initialized || !_channelAvailable) return false;
    try {
      return await _channel.invokeMethod<bool>('deleteModel', {'modelId': modelId}) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<Map<String, dynamic>?> pickCustomGguf() async {
    if (!_initialized || !_channelAvailable) return null;
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

  static Future<bool> shouldShowOnboarding() async {
    if (!_initialized || !_channelAvailable) return false;
    try {
      return await _channel.invokeMethod<bool>('shouldShowOnboarding') ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<bool> markOnboardingComplete() async {
    if (!_initialized || !_channelAvailable) return false;
    try {
      return await _channel.invokeMethod<bool>('markOnboardingComplete') ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  static Future<bool> scheduleReminderForDate({required int id, required String description, String? expectedDate}) async {
    if (!_initialized || !_channelAvailable || expectedDate == null) return false;
    final due = DateTime.tryParse(expectedDate);
    if (due == null) return false;
    final trigger = DateTime(due.year, due.month, due.day, 9);
    if (!trigger.isAfter(DateTime.now())) return false;
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
    if (!_initialized || !_channelAvailable) return false;
    try {
      return await _channel.invokeMethod<bool>('cancelReminder', {'id': id}) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }
}
