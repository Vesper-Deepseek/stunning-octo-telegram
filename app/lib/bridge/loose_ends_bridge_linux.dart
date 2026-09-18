import 'dart:ffi';
import 'dart:io';
import 'dart:convert';

class LooseEndsBridgeLinux {
  static DynamicLibrary? _lib;
  static Pointer<Void>? _storeHandle;

  static Pointer<Void> Function(Pointer<Uint8>)? _looseEndsOpen;
  static void Function(Pointer<Uint8>)? _looseEndsFree;
  static Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, int, int, int)?
      _looseEndsIngestRules;
  static int Function(Pointer<Void>, int, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>)?
      _looseEndsConfirmDraft;
  static Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, int, int, int)?
      _looseEndsListOpen;
  static int Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>)?
      _looseEndsCreateCommitment;

  static Pointer<Void> Function(int)? _malloc;
  static void Function(Pointer<Void>)? _free;

  static Future<void> init() async {
    if (_lib != null) return;

    _lib = await _loadLibrary();

    _looseEndsOpen = _lib!.lookupFunction<
        Pointer<Void> Function(Pointer<Uint8>),
        Pointer<Void> Function(Pointer<Uint8>)>('loose_ends_open');

    _looseEndsFree = _lib!.lookupFunction<
        Void Function(Pointer<Uint8>),
        void Function(Pointer<Uint8>)>('loose_ends_free');

    _looseEndsIngestRules = _lib!.lookupFunction<
        Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, Int32, Int32, Int32),
        Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, int, int, int)>(
        'loose_ends_ingest_rules');

    _looseEndsConfirmDraft = _lib!.lookupFunction<
        Int64 Function(Pointer<Void>, Int64, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>),
        int Function(Pointer<Void>, int, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>)>(
        'loose_ends_confirm_draft');

    _looseEndsListOpen = _lib!.lookupFunction<
        Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, Int32, Int32, Int32),
        Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, int, int, int)>(
        'loose_ends_list_open');

    _looseEndsCreateCommitment = _lib!.lookupFunction<
        Int64 Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>),
        int Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>)>(
        'loose_ends_create_commitment');

    final procLib = DynamicLibrary.process();
    _malloc = procLib.lookupFunction<Pointer<Void> Function(Int64), Pointer<Void> Function(int)>('malloc');
    _free = procLib.lookupFunction<Void Function(Pointer<Void>), void Function(Pointer<Void>)>('free');

    final home = Platform.environment['HOME'];
    final dataRoot = Platform.environment['XDG_DATA_HOME'] ??
        (home == null ? Directory.current.path : '$home/.local/share');
    final appDir = Directory('$dataRoot${Platform.pathSeparator}loose_ends');
    await appDir.create(recursive: true);
    final dbPath = '${appDir.path}${Platform.pathSeparator}loose_ends.db';
    final cPath = _stringToCString(dbPath);
    _storeHandle = _looseEndsOpen!(cPath);
    _freeCString(cPath);
    if (_storeHandle == nullptr) {
      throw Exception('Failed to open store at $dbPath');
    }
  }

  static Pointer<Uint8> _stringToCString(String s) {
    final bytes = utf8.encode(s);
    final ptr = _malloc!(bytes.length + 1);
    final view = ptr.cast<Uint8>().asTypedList(bytes.length + 1);
    for (var i = 0; i < bytes.length; i++) {
      view[i] = bytes[i];
    }
    view[bytes.length] = 0;
    return ptr.cast<Uint8>();
  }

  static void _freeCString(Pointer<Uint8> ptr) {
    if (ptr != nullptr) {
      _free!(ptr.cast<Void>());
    }
  }

  static String _cStringToDart(Pointer<Uint8> ptr) {
    if (ptr == nullptr) return '';
    final bytes = <int>[];
    var offset = 0;
    while (true) {
      final b = ptr[offset];
      if (b == 0) break;
      bytes.add(b);
      offset++;
    }
    return utf8.decode(bytes);
  }

  static Future<DynamicLibrary> _loadLibrary() async {
    final candidates = <String>[];

    if (Platform.isLinux) {
      final exePath = Platform.resolvedExecutable;
      final separator = Platform.pathSeparator;
      final cut = exePath.lastIndexOf(separator);
      final exeDir = cut >= 0 ? exePath.substring(0, cut) : Directory.current.path;
      candidates.addAll([
        '$exeDir${Platform.pathSeparator}libloose_ends_native.so',
        '$exeDir${Platform.pathSeparator}lib${Platform.pathSeparator}libloose_ends_native.so',
        '${Directory.current.path}${Platform.pathSeparator}native${Platform.pathSeparator}target${Platform.pathSeparator}debug${Platform.pathSeparator}libloose_ends_native.so',
        '${Directory.current.path}${Platform.pathSeparator}app${Platform.pathSeparator}native${Platform.pathSeparator}target${Platform.pathSeparator}debug${Platform.pathSeparator}libloose_ends_native.so',
      ]);
    }

    for (final candidate in candidates) {
      final file = File(candidate);
      if (await file.exists()) {
        return DynamicLibrary.open(candidate);
      }
    }

    return DynamicLibrary.open('libloose_ends_native.so');
  }

  static void _checkStore() {
    if (_storeHandle == null) throw StateError('Bridge not initialized');
  }

  static List<Map<String, dynamic>> ingestText(String text) {
    _checkStore();
    final now = DateTime.now();
    final cText = _stringToCString(text);
    final resultPtr = _looseEndsIngestRules!(
      _storeHandle!,
      cText,
      now.year,
      now.month,
      now.day,
    );
    _freeCString(cText);

    final resultStr = _cStringToDart(resultPtr);
    _looseEndsFree!(resultPtr);

    final json = jsonDecode(resultStr) as Map<String, dynamic>;
    final drafts = (json['drafts'] as List).cast<Map>().map((m) {
      return {
        'id': m['id'] as int,
        'description': m['description'] as String,
        'direction': m['direction'] as String,
        'expected_date': m['expected_date'] as String?,
        'party': m['party'] as String?,
        'party_confidence': m['party_confidence'] as String,
        'date_confidence': m['date_confidence'] as String,
        'overall_confidence': m['overall_confidence'] as String,
      };
    }).toList();
    return drafts;
  }

  static int? confirmDraft(
    Map<String, dynamic> draft, {
    String? descriptionOverride,
    String? directionOverride,
    String? dateOverride,
  }) {
    _checkStore();
    final descC = _stringToCString(descriptionOverride ?? draft['description'] as String);
    final dirC = _stringToCString(directionOverride ?? draft['direction'] as String);
    final dateC = _stringToCString(dateOverride ?? draft['expected_date'] as String? ?? '');
    final partyC = _stringToCString(draft['party'] as String? ?? '');

    final id = _looseEndsConfirmDraft!(
      _storeHandle!,
      draft['id'] as int? ?? 0,
      descC,
      dirC,
      dateC,
      partyC,
    );

    _freeCString(descC);
    _freeCString(dirC);
    _freeCString(dateC);
    _freeCString(partyC);

    return id > 0 ? id : null;
  }

  static List<Map<String, dynamic>> listOpen(String direction) {
    _checkStore();
    final now = DateTime.now();
    final dirC = _stringToCString(direction);
    final resultPtr = _looseEndsListOpen!(
      _storeHandle!,
      dirC,
      now.year,
      now.month,
      now.day,
    );
    _freeCString(dirC);

    final resultStr = _cStringToDart(resultPtr);
    _looseEndsFree!(resultPtr);

    final list = jsonDecode(resultStr) as List;
    return list.cast<Map>().map((m) {
      return {
        'id': m['id'] as int,
        'description': m['description'] as String,
        'direction': m['direction'] as String,
        'expected_date': m['expected_date'] as String?,
        'party': m['party'] as String?,
        'aging_action': m['aging_action'] as String,
        'created_at': m['created_at'] as String,
      };
    }).toList();
  }

  static int createCommitment({
    required String description,
    required String direction,
    String? expectedDate,
    String? party,
  }) {
    _checkStore();
    final descC = _stringToCString(description);
    final dirC = _stringToCString(direction);
    final dateC = _stringToCString(expectedDate ?? '');
    final partyC = _stringToCString(party ?? '');

    final id = _looseEndsCreateCommitment!(
      _storeHandle!,
      descC,
      dirC,
      dateC,
      partyC,
    );

    _freeCString(descC);
    _freeCString(dirC);
    _freeCString(dateC);
    _freeCString(partyC);

    return id > 0 ? id : 0;
  }
}
