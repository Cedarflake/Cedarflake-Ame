#include "flutter_window.h"

#include <windows.h>

#include <cstdio>
#include <cstdlib>
#include <cwchar>
#include <filesystem>

namespace {

HWND parent_window = nullptr;
HWND child_window = nullptr;
unsigned int child_destroy_notifications = 0;
unsigned int plugin_link_calls = 0;
bool retiring_window = false;
volatile LONG access_violation_observed = 0;

void Require(bool condition, const char* message) {
  if (!condition) {
    std::fprintf(stderr, "FAIL: %s\n", message);
    std::fflush(stderr);
    std::exit(EXIT_FAILURE);
  }
}

LONG CALLBACK ObserveAccessViolation(EXCEPTION_POINTERS* exception) {
  if (exception && exception->ExceptionRecord &&
      exception->ExceptionRecord->ExceptionCode == EXCEPTION_ACCESS_VIOLATION) {
    InterlockedExchange(&access_violation_observed, 1);
  }
  // A window callback exception may be swallowed by the platform. Record it,
  // but never handle it or convert an abnormal exit into success.
  return EXCEPTION_CONTINUE_SEARCH;
}

LRESULT CALLBACK ObserveWindowMessages(int code, WPARAM wparam, LPARAM lparam) {
  if (code == HC_ACTION) {
    const auto* message = reinterpret_cast<const CWPSTRUCT*>(lparam);
    if (message->hwnd == parent_window && message->message == WM_PARENTNOTIFY &&
        LOWORD(message->wParam) == WM_DESTROY &&
        reinterpret_cast<HWND>(message->lParam) == child_window) {
      Require(retiring_window, "Child destruction must occur during retirement");
      ++child_destroy_notifications;
      std::printf("NATIVE_CHILD_DESTROY notifications=%u\n",
                  child_destroy_notifications);
      std::fflush(stdout);
    }
  }
  return CallNextHookEx(nullptr, code, wparam, lparam);
}

void BindActualChild(HWND parent) {
  DWORD parent_pid = 0;
  Require(IsWindow(parent) != FALSE &&
              GetWindowThreadProcessId(parent, &parent_pid) == GetCurrentThreadId() &&
              parent_pid == GetCurrentProcessId(),
          "The real parent HWND must belong to this thread and process");
  const HWND child = FindWindowExW(parent, nullptr, L"FLUTTERVIEW", nullptr);
  Require(child != nullptr &&
              FindWindowExW(parent, child, L"FLUTTERVIEW", nullptr) == nullptr,
          "Exactly one real FLUTTERVIEW child must exist");
  DWORD child_pid = 0;
  Require(GetWindowThreadProcessId(child, &child_pid) == GetCurrentThreadId() &&
              child_pid == GetCurrentProcessId() && GetParent(child) == parent,
          "The Flutter child must belong to this parent, thread, and process");
  SetLastError(ERROR_SUCCESS);
  const LONG_PTR ex_style = GetWindowLongPtrW(child, GWL_EXSTYLE);
  Require(ex_style != 0 || GetLastError() == ERROR_SUCCESS,
          "The child extended style must be readable");
  Require((ex_style & WS_EX_NOPARENTNOTIFY) == 0,
          "The actual child must permit natural parent notification");
  parent_window = parent;
  child_window = child;
  std::printf("ENGINE_READY child_class=FLUTTERVIEW child_owned=1 no_parent_notify=0\n");
  std::fflush(stdout);
}

void RequireInputFile(const std::filesystem::path& path) {
  Require(path.is_absolute(), "Fixture inputs must use absolute paths");
  const DWORD attributes = GetFileAttributesW(path.c_str());
  Require(attributes != INVALID_FILE_ATTRIBUTES &&
              (attributes & FILE_ATTRIBUTE_DIRECTORY) == 0,
          "The prepared fixture input file must exist");
}

}  // namespace

// This link seam preserves real engine creation without loading Ame plugins.
void RegisterPlugins(flutter::PluginRegistry*) {
  ++plugin_link_calls;
}

int wmain(int argc, wchar_t** argv) {
  SetErrorMode(GetErrorMode() | SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
  Require(argc == 4, "Expected case, absolute assets directory, and absolute ICU file");
  const bool explicit_destroy = std::wcscmp(argv[1], L"explicit-destroy") == 0;
  Require(explicit_destroy || std::wcscmp(argv[1], L"scope-exit") == 0,
          "Unknown engine lifecycle case");
  RequireInputFile(std::filesystem::path(argv[2]) / L"kernel_blob.bin");
  RequireInputFile(std::filesystem::path(argv[3]));

  const HRESULT com_status = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
  Require(SUCCEEDED(com_status), "COM initialization must succeed");
  const PVOID exception_observer = AddVectoredExceptionHandler(1, ObserveAccessViolation);
  Require(exception_observer != nullptr, "Exception observation must be installed");
  const HHOOK window_observer = SetWindowsHookExW(
      WH_CALLWNDPROC, ObserveWindowMessages, nullptr, GetCurrentThreadId());
  Require(window_observer != nullptr, "Natural message observation must be installed");
  {
    flutter::DartProject project(argv[2], argv[3], L"");
    FlutterWindow window(project);
    Require(window.Create(L"Ame engine lifecycle fixture", Win32Window::Point(0, 0),
                          Win32Window::Size(100, 100)),
            "The production window must create its real Flutter engine");
    Require(plugin_link_calls == 1, "The production registration boundary must execute once");
    BindActualChild(window.GetHandle());
    retiring_window = true;
    if (explicit_destroy) {
      window.Destroy();
      Require(window.GetHandle() == nullptr && IsWindow(parent_window) == FALSE,
              "Explicit destruction must retire the parent HWND");
    }
    // The scope-exit case reaches the real member destructors without Destroy().
  }
  std::printf("SCOPE_COMPLETE child_destroy_notifications=%u\n",
              child_destroy_notifications);
  std::fflush(stdout);
  const BOOL window_observer_removed = UnhookWindowsHookEx(window_observer);
  const ULONG exception_observer_removed = RemoveVectoredExceptionHandler(exception_observer);
  CoUninitialize();

  Require(window_observer_removed != FALSE && exception_observer_removed != 0,
          "Both observation handles must be retired");
  Require(IsWindow(parent_window) == FALSE && IsWindow(child_window) == FALSE,
          "The parent and Flutter child must both be retired");
  Require(child_destroy_notifications == 1,
          "Exactly one natural child-destroy notification must execute");
  Require(InterlockedCompareExchange(&access_violation_observed, 0, 0) == 0,
          "An observed access violation cannot pass even if Windows swallowed it");
  std::printf("PASS: engine_lifecycle_%s engine_started=1 child_destroy_notifications=1 "
              "scope_completed=1 hwnd_retired=1 access_violations=0 plugins_registered=0 "
              "com_released_after_scope=1\n",
              explicit_destroy ? "explicit_destroy" : "scope_exit");
  return EXIT_SUCCESS;
}
