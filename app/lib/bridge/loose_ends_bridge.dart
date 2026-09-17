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
  static bool _initialized = false;

  static Future<void> init() async {
    if (_initialized) return;

    // Prefer the platform channel. This is the Android path and also makes
    // the bridge testable on Linux without requiring a native .so in tests.
    try {
      final initialized = await _channel.invokeMethod<bool>('init') ?? false;
      if (initialized) {
        _initialized = true;
        return;
      }
    } on PlatformException catch (e) {
      debugPrint('Bridge init failed: ${e.message}');
      return;
    } on MissingPluginException {
      // Desktop builds can fall through to the FFI implementation below.
    }

    if (Platform.isLinux) {
      try {
        await LooseEndsBridgeLinux.init();
        _initialized = true;
      } catch (e) {
        debugPrint('Linux native bridge unavailable; running in stub mode: $e');
      }
    } else {
      debugPrint('Native bridge unavailable; running in stub mode');
    }
  }

  static Future<List<Draft>> ingestText(String text) async {
    if (!_initialized) return _fallbackIngest(text);
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
      return _fallbackIngest(text);
    } on MissingPluginException {
      return _fallbackIngest(text);
    }
  }

  static bool _channelAvailable = false;

  static Future<int?> confirmDraft(
    Draft draft, {
    String? descriptionOverride,
    Direction? directionOverride,
    String? dateOverride,
  }) async {
    if (!_initialized) return null;
    if (Platform.isLinux && !_channelAvailable) {
      return LooseEndsBridgeLinux.confirmDraft(
        <String, dynamic>{
          'id': draft.id,
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
        'draftId': draft.id,
        'description': descriptionOverride ?? draft.description,
        'direction': (directionOverride ?? Direction.fromString(draft.direction)).name,
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

  static List<Draft> _fallbackIngest(String text) {
    return [
      Draft(
        id: 0,
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
