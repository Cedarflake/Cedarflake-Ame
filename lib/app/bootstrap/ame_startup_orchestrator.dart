Future<void> mountAmeApplicationAndRevealWindow({
  required void Function() mountApplication,
  required void Function() startInBackground,
  required Future<void> Function() revealWindowAfterFirstFrame,
}) async {
  mountApplication();
  startInBackground();
  await revealWindowAfterFirstFrame();
}
