import "dart:async";
import "dart:convert";
import "dart:io";

enum NativeDirectoryPickerAction { confirm, cancel }

typedef NativeDirectoryPickerAutomation = ({
  Process process,
  Future<String> output,
  Future<String> error,
});

Future<NativeDirectoryPickerAutomation> startNativeDirectoryPickerAutomation(
  NativeDirectoryPickerAction action,
) async {
  final scriptPath =
      "${Directory.current.path}${Platform.pathSeparator}integration_test"
      "${Platform.pathSeparator}support${Platform.pathSeparator}"
      "control_native_directory.ps1";
  final process = await Process.start("powershell.exe", [
    "-NoLogo",
    "-NoProfile",
    "-NonInteractive",
    "-File",
    scriptPath,
    "-TargetProcessId",
    "$pid",
    "-Action",
    switch (action) {
      NativeDirectoryPickerAction.confirm => "Confirm",
      NativeDirectoryPickerAction.cancel => "Cancel",
    },
  ]);
  final ready = Completer<void>();
  final output = process.stdout
      .transform(const Utf8Decoder(allowMalformed: true))
      .transform(const LineSplitter())
      .map((line) {
        if (line == "NATIVE_PICKER_READY" && !ready.isCompleted) {
          ready.complete();
        }
        return line;
      })
      .join("\n");
  final error = process.stderr
      .transform(const Utf8Decoder(allowMalformed: true))
      .join();
  try {
    // Compilation and startup finish before the caller starts the picker action deadline.
    await Future.any<void>([
      ready.future,
      process.exitCode.then<void>((code) async {
        throw StateError(
          "Native directory picker helper exited before readiness: $code\n"
          "${await output}\n${await error}",
        );
      }),
    ]).timeout(
      const Duration(seconds: 30),
      onTimeout: () => throw TimeoutException(
        "Native directory picker helper did not become ready within 30 seconds",
      ),
    );
  } catch (startupError, stackTrace) {
    process.kill();
    try {
      await process.exitCode.timeout(const Duration(seconds: 5));
    } on TimeoutException {
      Error.throwWithStackTrace(
        StateError(
          "$startupError\nThe owned picker helper did not exit after termination",
        ),
        stackTrace,
      );
    }
    Error.throwWithStackTrace(startupError, stackTrace);
  }
  return (process: process, output: output, error: error);
}
