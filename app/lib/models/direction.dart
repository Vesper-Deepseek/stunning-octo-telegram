enum Direction {
  userOwes('user_owes'),
  owedToUser('owed_to_user'),
  unclear('unclear');

  final String name;
  const Direction(this.name);

  static Direction fromString(String s) {
    return Direction.values.firstWhere(
      (d) => d.name == s,
      orElse: () => Direction.unclear,
    );
  }

  String get displayName {
    switch (this) {
      case Direction.userOwes:
        return 'You Owe';
      case Direction.owedToUser:
        return 'Owed to You';
      case Direction.unclear:
        return 'Unclear';
    }
  }
}
