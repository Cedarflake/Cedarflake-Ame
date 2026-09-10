#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <windows.h>

#include "flutter_window.h"
#include "single_instance_lock.h"
#include "utils.h"

namespace {

int RunFlutterWindow() {
  flutter::DartProject project(L"data");
  project.set_dart_entrypoint_arguments(GetCommandLineArguments());

  FlutterWindow window(project);
  if (!window.Create(L"Cedarflake Ame", Win32Window::Point(10, 10),
                     Win32Window::Size(1280, 720))) {
    return EXIT_FAILURE;
  }
  window.SetQuitOnClose(true);

  ::MSG msg;
  BOOL result;
  while ((result = ::GetMessage(&msg, nullptr, 0, 0)) > 0) {
    ::TranslateMessage(&msg);
    ::DispatchMessage(&msg);
  }
  return result == -1 ? EXIT_FAILURE : EXIT_SUCCESS;
}

}  // namespace

int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  SingleInstanceLock single_instance_lock;
  const auto single_instance_result = single_instance_lock.Acquire();
  if (single_instance_result == SingleInstanceLockResult::kAlreadyRunning) {
    return EXIT_SUCCESS;
  }
  if (single_instance_result == SingleInstanceLockResult::kFailed) {
    return EXIT_FAILURE;
  }

  // Attach to console when present (e.g., 'flutter run') or create a
  // new console when running with a debugger.
  if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
    CreateAndAttachConsole();
  }

  const HRESULT com_result =
      ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
  if (FAILED(com_result)) {
    return EXIT_FAILURE;
  }

  // Window and engine teardown may dispatch messages that still require COM.
  const int exit_code = RunFlutterWindow();
  ::CoUninitialize();
  return exit_code;
}
