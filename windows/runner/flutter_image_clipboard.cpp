#include "flutter_image_clipboard.h"

#include <flutter/standard_method_codec.h>

#include <cstdint>
#include <cstring>
#include <memory>
#include <vector>

namespace {

constexpr std::int64_t kMaximumPixels = 32 * 1024 * 1024;

std::int64_t Integer(const flutter::EncodableMap &arguments, const char *key) {
  const auto found = arguments.find(flutter::EncodableValue(key));
  if (found == arguments.end()) return -1;
  if (const auto value = std::get_if<std::int32_t>(&found->second))
    return *value;
  if (const auto value = std::get_if<std::int64_t>(&found->second))
    return *value;
  return -1;
}

struct ClipboardMemory {
  HGLOBAL handle;
  ~ClipboardMemory() {
    if (handle) ::GlobalFree(handle);
  }
};

struct ClipboardAccess {
  ~ClipboardAccess() { ::CloseClipboard(); }
};

DWORD WriteImage(HWND window, const std::vector<std::uint8_t> &rgba,
                 const std::vector<std::uint8_t> &png, LONG width,
                 LONG height) {
  const UINT png_format = ::RegisterClipboardFormat(L"PNG");
  if (!png_format) return ::GetLastError();
  ClipboardMemory png_memory{::GlobalAlloc(GMEM_MOVEABLE, png.size())};
  if (!png_memory.handle) return ERROR_NOT_ENOUGH_MEMORY;
  auto *png_address = ::GlobalLock(png_memory.handle);
  if (!png_address) return ::GetLastError();
  std::memcpy(png_address, png.data(), png.size());
  ::GlobalUnlock(png_memory.handle);
  ClipboardMemory memory{
      ::GlobalAlloc(GMEM_MOVEABLE, sizeof(BITMAPV5HEADER) + rgba.size())};
  if (!memory.handle) return ERROR_NOT_ENOUGH_MEMORY;
  auto *address = static_cast<std::uint8_t *>(::GlobalLock(memory.handle));
  if (!address) return ::GetLastError();
  BITMAPV5HEADER header{};
  header.bV5Size = sizeof(header);
  header.bV5Width = width;
  header.bV5Height = -height;
  header.bV5Planes = 1;
  header.bV5BitCount = 32;
  header.bV5Compression = BI_BITFIELDS;
  header.bV5SizeImage = static_cast<DWORD>(rgba.size());
  header.bV5RedMask = 0x00ff0000;
  header.bV5GreenMask = 0x0000ff00;
  header.bV5BlueMask = 0x000000ff;
  header.bV5AlphaMask = 0xff000000;
  header.bV5CSType = LCS_sRGB;
  header.bV5Intent = LCS_GM_IMAGES;
  std::memcpy(address, &header, sizeof(header));
  auto *pixels = address + sizeof(header);
  for (size_t offset = 0; offset < rgba.size(); offset += 4) {
    pixels[offset] = rgba[offset + 2];
    pixels[offset + 1] = rgba[offset + 1];
    pixels[offset + 2] = rgba[offset];
    pixels[offset + 3] = rgba[offset + 3];
  }
  ::GlobalUnlock(memory.handle);
  if (!::OpenClipboard(window)) return ::GetLastError();
  const ClipboardAccess access;
  if (!::EmptyClipboard()) return ::GetLastError();
  if (!::SetClipboardData(png_format, png_memory.handle))
    return ::GetLastError();
  png_memory.handle = nullptr;
  if (!::SetClipboardData(CF_DIBV5, memory.handle)) return ::GetLastError();
  // Windows owns the unlocked allocation after successful publication.
  memory.handle = nullptr;
  return ERROR_SUCCESS;
}

}  // namespace

FlutterImageClipboard::FlutterImageClipboard(
    HWND window, flutter::BinaryMessenger *messenger)
    : channel_(messenger, "cedarflake_ame/image_clipboard",
               &flutter::StandardMethodCodec::GetInstance()) {
  channel_.SetMethodCallHandler(
      [window](const flutter::MethodCall<flutter::EncodableValue> &call,
               std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
                   result) {
        if (call.method_name() != "writeImage") {
          result->NotImplemented();
          return;
        }
        const auto *arguments =
            call.arguments()
                ? std::get_if<flutter::EncodableMap>(call.arguments())
                : nullptr;
        if (!arguments) {
          result->Error("clipboard_image_invalid",
                        "Image arguments are missing");
          return;
        }
        const auto width = Integer(*arguments, "width");
        const auto height = Integer(*arguments, "height");
        const auto found = arguments->find(flutter::EncodableValue("rgba"));
        const auto *rgba =
            found == arguments->end()
                ? nullptr
                : std::get_if<std::vector<std::uint8_t>>(&found->second);
        const auto png_found = arguments->find(flutter::EncodableValue("png"));
        const auto *png =
            png_found == arguments->end()
                ? nullptr
                : std::get_if<std::vector<std::uint8_t>>(&png_found->second);
        if (width <= 0 || height <= 0 || width > kMaximumPixels ||
            height > kMaximumPixels || width * height > kMaximumPixels ||
            !rgba || !png || png->empty() ||
            png->size() > static_cast<size_t>(kMaximumPixels * 5) ||
            rgba->size() != static_cast<size_t>(width * height * 4)) {
          result->Error("clipboard_image_invalid",
                        "Image dimensions or pixels are invalid");
          return;
        }
        const DWORD error =
            WriteImage(window, *rgba, *png, static_cast<LONG>(width),
                       static_cast<LONG>(height));
        if (error != ERROR_SUCCESS) {
          result->Error(
              "clipboard_unavailable", "Windows could not copy the image",
              flutter::EncodableValue(static_cast<std::int64_t>(error)));
          return;
        }
        result->Success();
      });
}

FlutterImageClipboard::~FlutterImageClipboard() {
  channel_.SetMethodCallHandler(nullptr);
}
