/// A candidate commitment extracted from text, pending user review.
class Draft {
  /// The persistent draft id returned by the Rust core, when available.
  ///
  /// Fallback drafts created without a native backend do not have an id and
  /// therefore cannot be confirmed through the native bridge.
  final int? id;
  final String description;
  final String direction; // 'user_owes' | 'owed_to_user' | 'unclear'
  final String? expectedDate;
  final String? party;
  final String partyConfidence; // 'high' | 'low'
  final String dateConfidence;
  final String overallConfidence;

  Draft({
    this.id,
    required this.description,
    required this.direction,
    this.expectedDate,
    this.party,
    required this.partyConfidence,
    required this.dateConfidence,
    required this.overallConfidence,
  });
}
