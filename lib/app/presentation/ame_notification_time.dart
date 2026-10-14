String formatAmeNotificationElapsed(Duration elapsed) {
  final seconds = elapsed.isNegative ? 0 : elapsed.inSeconds;
  if (seconds < 60) {
    return "$seconds 秒";
  }
  final minutes = seconds ~/ 60;
  if (minutes < 60) {
    return "$minutes 分 ${seconds % 60} 秒";
  }
  final hours = minutes ~/ 60;
  return "$hours 小时 ${minutes % 60} 分";
}

String formatAmeNotificationOccurredAt(DateTime occurredAt) {
  final local = occurredAt.toLocal();
  String twoDigits(int value) => value.toString().padLeft(2, "0");
  return "${local.year}-${twoDigits(local.month)}-${twoDigits(local.day)} "
      "${twoDigits(local.hour)}:${twoDigits(local.minute)}";
}
