#include "flutter_window.h"

#include <dwmapi.h>

#include <cstdint>
#include <memory>
#include <optional>
#include <string>

#include <flutter/standard_method_codec.h>

#include "flutter/generated_plugin_registrant.h"

namespace {

constexpr char kSystemThemeChannelName[] = "cedarflake_ame/system_theme";
constexpr char kGetAccentColorMethod[] = "getAccentColor";
constexpr char kAccentColorChangedMethod[] = "accentColorChanged";

std::optional<std::int64_t> ReadSystemAccentColor() {
  DWORD color = 0;
  BOOL is_opaque = FALSE;
  if (FAILED(::DwmGetColorizationColor(&color, &is_opaque))) {
    return std::nullopt;
  }
  return static_cast<std::int64_t>(color);
}

}  // namespace

FlutterWindow::FlutterWindow(const flutter::DartProject& project)
    : project_(project) {}

FlutterWindow::~FlutterWindow() {}

bool FlutterWindow::OnCreate() {
  if (!Win32Window::OnCreate()) {
    return false;
  }

  RECT frame = GetClientArea();

  // The size here must match the window dimensions to avoid unnecessary surface
  // creation / destruction in the startup path.
  flutter_controller_ = std::make_unique<flutter::FlutterViewController>(
      frame.right - frame.left, frame.bottom - frame.top, project_);
  // Ensure that basic setup of the controller was successful.
  if (!flutter_controller_->engine() || !flutter_controller_->view()) {
    return false;
  }
  RegisterPlugins(flutter_controller_->engine());
  InitializeSystemThemeChannel();
  SetChildContent(flutter_controller_->view()->GetNativeWindow());

  flutter_controller_->engine()->SetNextFrameCallback([&]() {
    this->Show();
  });

  // Flutter can complete the first frame before the "show window" callback is
  // registered. The following call ensures a frame is pending to ensure the
  // window is shown. It is a no-op if the first frame hasn't completed yet.
  flutter_controller_->ForceRedraw();

  return true;
}

void FlutterWindow::OnDestroy() {
  if (system_theme_channel_) {
    system_theme_channel_->SetMethodCallHandler(nullptr);
    system_theme_channel_.reset();
  }
  if (flutter_controller_) {
    flutter_controller_ = nullptr;
  }

  Win32Window::OnDestroy();
}

LRESULT
FlutterWindow::MessageHandler(HWND hwnd, UINT const message,
                              WPARAM const wparam,
                              LPARAM const lparam) noexcept {
  if (message == WM_DWMCOLORIZATIONCOLORCHANGED) {
    NotifySystemAccentColor();
  }

  // Give Flutter, including plugins, an opportunity to handle window messages.
  if (flutter_controller_) {
    std::optional<LRESULT> result =
        flutter_controller_->HandleTopLevelWindowProc(hwnd, message, wparam,
                                                      lparam);
    if (result) {
      return *result;
    }
  }

  switch (message) {
    case WM_FONTCHANGE:
      flutter_controller_->engine()->ReloadSystemFonts();
      break;
  }

  return Win32Window::MessageHandler(hwnd, message, wparam, lparam);
}

void FlutterWindow::InitializeSystemThemeChannel() {
  system_theme_channel_ =
      std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
          flutter_controller_->engine()->messenger(), kSystemThemeChannelName,
          &flutter::StandardMethodCodec::GetInstance());
  system_theme_channel_->SetMethodCallHandler(
      [](const flutter::MethodCall<flutter::EncodableValue>& call,
         std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
             result) {
        if (call.method_name() != kGetAccentColorMethod) {
          result->NotImplemented();
          return;
        }
        const std::optional<std::int64_t> color = ReadSystemAccentColor();
        if (!color) {
          result->Error("system_accent_unavailable",
                        "Windows did not provide a system accent color");
          return;
        }
        result->Success(flutter::EncodableValue(*color));
      });
}

void FlutterWindow::NotifySystemAccentColor() {
  if (!system_theme_channel_) {
    return;
  }
  const std::optional<std::int64_t> color = ReadSystemAccentColor();
  if (!color) {
    return;
  }
  system_theme_channel_->InvokeMethod(
      kAccentColorChangedMethod,
      std::make_unique<flutter::EncodableValue>(*color));
}
