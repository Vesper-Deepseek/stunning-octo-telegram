import 'direction.dart';

class Commitment {
  final int id;
  final String description;
  final Direction direction;
  final String? expectedDate;
  final String? party;
  final String createdAt;

  Commitment({
    required this.id,
    required this.description,
    required this.direction,
    this.expectedDate,
    this.party,
    required this.createdAt,
  });
}

/// Commitment + planner-recommended action.
class CommitmentView {
  final int id;
  final String description;
  final Direction direction;
  final String? expectedDate;
  final String? party;
  final String agingAction; // 'surface' | 'snooze' | 'escalate' | 'archive'
  final String createdAt;

  CommitmentView({
    required this.id,
    required this.description,
    required this.direction,
    this.expectedDate,
    this.party,
    required this.agingAction,
    required this.createdAt,
  });
}
