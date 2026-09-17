enum Direction {
  userOwes('user_owes'),
  owedToUser('owed_to_user'),
  unclear('unclear');

  final String name;
  const Direction(this.name);

  static Direction fromString(String s) {
    switch (s) {
      case 'user_owes':
      case 'userowes':
        return Direction.userOwes;
      case 'owed_to_user':
      case 'owedtouser':
        return Direction.owedToUser;
      case 'unclear':
        return Direction.unclear;
      default:
        return Direction.unclear;
    }
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
