/// A candidate commitment extracted from text, pending user review.
class Draft {
  final int id;
  final String description;
  final String direction; // 'user_owes' | 'owed_to_user' | 'unclear'
  final String? expectedDate;
  final String? party;
  final String partyConfidence; // 'high' | 'low'
  final String dateConfidence;
  final String overallConfidence;

  const Draft({
    this.id = 0,
    required this.description,
    required this.direction,
    this.expectedDate,
    this.party,
    required this.partyConfidence,
    required this.dateConfidence,
    required this.overallConfidence,
  });
}
