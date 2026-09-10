#include "flutter_window.h"

#include <windows.h>

#include <cstdio>
#include <cstdlib>
#include <string_view>

namespace {

enum class TestPhase { kControl, kStartup, kTeardown };

void Require(bool condition, const char* message) {
  if (!condition) {
    std::fprintf(stderr, "FAIL: %s\n", message);
    std::fflush(stderr);
    std::exit(EXIT_FAILURE);
  }
}

class LifecycleWindow final : public FlutterWindow {
 public:
  explicit LifecycleWindow(TestPhase phase)
      : FlutterWindow(flutter::DartProject(L"unused-no-engine")), phase_(phase) {}

  unsigned int create_messages() const { return create_messages_; }
  unsigned int on_create_calls() const { return on_create_calls_; }
  unsigned int font_messages() const { return font_messages_; }
  unsigned int font_returns() const { return font_returns_; }
  unsigned int null_messages() const { return null_messages_; }
  bool sent_during_teardown() const { return sent_during_teardown_; }

 protected:
  bool OnCreate() override {
    ++on_create_calls_;
    Require(create_messages_ == 1, "OnCreate must follow the real WM_CREATE");
    // Keep the real HWND and message dispatcher without creating a Flutter engine.
    return Win32Window::OnCreate();
  }

  void OnDestroy() override {
    const HWND window = GetHandle() ? GetHandle() : destroying_window_;
    FlutterWindow::OnDestroy();
    if (phase_ == TestPhase::kTeardown && window && !sent_during_teardown_) {
      sent_during_teardown_ = true;
      Require(IsWindow(window) != FALSE, "OnDestroy must precede HWND retirement");
      SendFontChange(window, "teardown");
    }
  }

  LRESULT MessageHandler(HWND window, UINT message, WPARAM wparam,
                         LPARAM lparam) noexcept override {
    if (message == WM_CREATE) {
      ++create_messages_;
      Require(on_create_calls_ == 0, "WM_CREATE must precede Flutter setup");
      if (phase_ == TestPhase::kStartup) {
        SendFontChange(window, "startup");
      }
    }
    if (message == WM_FONTCHANGE) {
      ++font_messages_;
      const LRESULT result =
          FlutterWindow::MessageHandler(window, message, wparam, lparam);
      ++font_returns_;
      return result;
    }
    if (message == WM_NULL) {
      ++null_messages_;
    }
    if (message == WM_DESTROY) {
      destroying_window_ = window;
    }
    const LRESULT result =
        FlutterWindow::MessageHandler(window, message, wparam, lparam);
    if (message == WM_DESTROY) {
      destroying_window_ = nullptr;
    }
    return result;
  }

 private:
  void SendFontChange(HWND window, const char* phase) {
    Require(IsWindow(window) != FALSE, "Font message requires a live owned HWND");
    const unsigned int before_messages = font_messages_;
    const unsigned int before_returns = font_returns_;
    std::printf("font-dispatch phase=%s live_hwnd=1 engine_started=0\n", phase);
    std::fflush(stdout);
    SendMessageW(window, WM_FONTCHANGE, 0, 0);
    Require(font_messages_ == before_messages + 1,
            "Font message must reach the production message handler once");
    Require(font_returns_ == before_returns + 1,
            "The production font handler must return without an exception");
  }

  const TestPhase phase_;
  HWND destroying_window_ = nullptr;
  unsigned int create_messages_ = 0;
  unsigned int on_create_calls_ = 0;
  unsigned int font_messages_ = 0;
  unsigned int font_returns_ = 0;
  unsigned int null_messages_ = 0;
  bool sent_during_teardown_ = false;
};

}  // namespace

// Link only: executing plugin registration would violate the engine-free fixture.
void RegisterPlugins(flutter::PluginRegistry*) {
  Require(false, "Lifecycle fixture must not register plugins or start Flutter");
}

int main(int argc, char** argv) {
  // Suppress this process's fault reporting, not its failure or exception exit.
  SetErrorMode(GetErrorMode() | SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
  Require(argc == 2, "Expected one case: control, startup, or teardown");
  const std::string_view name(argv[1]);
  Require(name == "control" || name == "startup" || name == "teardown",
          "Unknown lifecycle case");
  const TestPhase phase = name == "startup"    ? TestPhase::kStartup
                          : name == "teardown" ? TestPhase::kTeardown
                                               : TestPhase::kControl;

  LifecycleWindow window(phase);
  Require(window.Create(L"Ame owned window lifecycle fixture",
                        Win32Window::Point(0, 0), Win32Window::Size(100, 100)),
          "The real Win32 window must be created");
  const HWND handle = window.GetHandle();
  Require(IsWindow(handle) != FALSE, "Created HWND must be valid");
  Require(IsWindowVisible(handle) == FALSE, "Fixture HWND must remain hidden");
  Require(window.on_create_calls() == 1, "Engine-free creation must execute once");
  const unsigned int before_null = window.null_messages();
  SendMessageW(handle, WM_NULL, 0, 0);
  Require(window.null_messages() == before_null + 1,
          "The real WndProc must dispatch ordinary messages");

  SendMessageW(handle, WM_CLOSE, 0, 0);
  Require(IsWindow(handle) == FALSE, "WM_CLOSE must retire the owned HWND");
  Require(window.GetHandle() == nullptr, "Production teardown must clear HWND");
  Require(window.create_messages() == 1, "Exactly one native creation is required");
  const unsigned int expected_fonts = phase == TestPhase::kControl ? 0 : 1;
  Require(window.font_messages() == expected_fonts,
          "The selected phase must deliver exactly its expected font messages");
  Require(window.font_returns() == expected_fonts,
          "Every delivered font message must return normally");
  Require(window.sent_during_teardown() == (phase == TestPhase::kTeardown),
          "Teardown injection must execute only for the selected phase");
  std::printf("PASS: %s fonts=%u returned=%u engine_started=0 hwnd_retired=1\n",
              argv[1], window.font_messages(), window.font_returns());
  return EXIT_SUCCESS;
}
