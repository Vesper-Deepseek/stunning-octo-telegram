import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import '../models/commitment.dart';
import '../models/direction.dart';
import '../models/draft.dart';
import 'loose_ends_bridge_linux.dart';

/// Bridge to the Rust core.
///
/// On Linux desktop, uses dart:ffi to call the native shared library directly
/// when it is available. The MethodChannel remains a test-friendly fallback.
/// On Android, uses the MethodChannel/JNI bridge.
class LooseEndsBridge {
  static const _channel = MethodChannel('com.looseends/core');
  static bool _initialized = false;
  static bool _usingFfi = false;

  static Future<void> init() async {
    if (_initialized) return;

    if (Platform.isLinux) {
      try {
        await LooseEndsBridgeLinux.init();
        _usingFfi = true;
        _initialized = true;
        return;
      } catch (e) {
        debugPrint('Linux FFI bridge unavailable: $e');
      }
    }

    try {
      await _channel.invokeMethod('init');
      _usingFfi = false;
      _initialized = true;
    } on PlatformException catch (e) {
      debugPrint('Bridge init failed: ${e.message}');
    } on MissingPluginException {
      debugPrint('Native channel not registered; running in stub mode');
    }
  }

  static Draft _draftFromMap(Map<dynamic, dynamic> m) {
    return Draft(
      id: (m['id'] as num?)?.toInt(),
      description: m['description'] as String,
      direction: m['direction'] as String,
      expectedDate: m['expected_date'] as String?,
      party: m['party'] as String?,
      partyConfidence: m['party_confidence'] as String,
      dateConfidence: m['date_confidence'] as String,
      overallConfidence: m['overall_confidence'] as String,
    );
  }

  static Future<List<Draft>> ingestText(String text) async {
    if (!_initialized) return _fallbackIngest(text);
    if (Platform.isLinux && _usingFfi) {
      final maps = LooseEndsBridgeLinux.ingestText(text);
      return maps.map(_draftFromMap).toList();
    }

    try {
      final result = await _channel.invokeMethod('ingestText', {'text': text});
      final drafts = (result as List).cast<Map>().map(_draftFromMap).toList();
      return drafts;
    } on PlatformException {
      return _fallbackIngest(text);
    } on MissingPluginException {
      return _fallbackIngest(text);
    }
  }

  static Future<int?> confirmDraft(
    Draft draft, {
    String? descriptionOverride,
    Direction? directionOverride,
    String? dateOverride,
  }) async {
    if (!_initialized) return null;
    final draftId = draft.id ?? 0;

    if (Platform.isLinux && _usingFfi) {
      return LooseEndsBridgeLinux.confirmDraft(
        <String, dynamic>{
          'id': draftId,
          'description': draft.description,
          'direction': draft.direction,
          'expected_date': draft.expectedDate,
          'party': draft.party,
        },
        descriptionOverride: descriptionOverride,
        directionOverride: directionOverride?.name,
        dateOverride: dateOverride,
      );
    }

    try {
      final result = await _channel.invokeMethod('confirmDraft', {
        'draftId': draftId,
        'description': descriptionOverride ?? draft.description,
        'direction':
            (directionOverride ?? Direction.fromString(draft.direction)).name,
        'expected_date': dateOverride ?? draft.expectedDate,
        'party': draft.party,
      });
      return result as int?;
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  static Future<List<CommitmentView>> listOpen(Direction dir) async {
    if (!_initialized) return [];
    if (Platform.isLinux && _usingFfi) {
      final maps = LooseEndsBridgeLinux.listOpen(dir.name);
      return maps
          .map((m) => CommitmentView(
                id: m['id'] as int,
                description: m['description'] as String,
                direction: Direction.fromString(m['direction'] as String),
                expectedDate: m['expected_date'] as String?,
                party: m['party'] as String?,
                agingAction: m['aging_action'] as String,
                createdAt: m['created_at'] as String,
              ))
          .toList();
    }

    try {
      final result = await _channel.invokeMethod('listOpen', {'direction': dir.name});
      return (result as List)
          .cast<Map>()
          .map((m) => CommitmentView(
                id: (m['id'] as num).toInt(),
                description: m['description'] as String,
                direction: Direction.fromString(m['direction'] as String),
                expectedDate: m['expected_date'] as String?,
                party: m['party'] as String?,
                agingAction: m['aging_action'] as String,
                createdAt: m['created_at'] as String,
              ))
          .toList();
    } on PlatformException {
      return [];
    } on MissingPluginException {
      return [];
    }
  }

  static List<Draft> _fallbackIngest(String text) {
    return [
      Draft(
        description: text.length > 80 ? '${text.substring(0, 80)}...' : text,
        direction: 'unclear',
        expectedDate: null,
        party: null,
        partyConfidence: 'low',
        dateConfidence: 'low',
        overallConfidence: 'low',
      ),
    ];
  }
}
