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
  final String sourceProvenance; // 'model_extracted' | 'rule_extracted' | 'manual'
  final String? createdAt;

  const Draft({
    this.id = 0,
    required this.description,
    required this.direction,
    this.expectedDate,
    this.party,
    required this.partyConfidence,
    required this.dateConfidence,
    required this.overallConfidence,
    this.sourceProvenance = 'rule_extracted',
    this.createdAt,
  });
}
