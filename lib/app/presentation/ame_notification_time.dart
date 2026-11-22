import "ame_localizations.dart";

String formatAmeNotificationElapsed(
  AppLocalizations strings,
  Duration elapsed,
) {
  final seconds = elapsed.isNegative ? 0 : elapsed.inSeconds;
  if (seconds < 60) {
    return strings.elapsedSeconds(seconds);
  }
  final minutes = seconds ~/ 60;
  if (minutes < 60) {
    return strings.elapsedMinutes(minutes, seconds % 60);
  }
  final hours = minutes ~/ 60;
  return strings.elapsedHours(hours, minutes % 60);
}

String formatAmeNotificationOccurredAt(
  AppLocalizations strings,
  DateTime occurredAt,
) {
  final local = occurredAt.toLocal();
  return strings.eventDateTime(local, local);
}
