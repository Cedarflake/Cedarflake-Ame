#include "flutter_diagnostic_environment.h"

#include <windows.h>
#include <setupapi.h>
#include <initguid.h>
#include <devguid.h>
#include <devpkey.h>

#include <flutter/standard_method_codec.h>

#include <cstdint>
#include <cwchar>
#include <exception>
#include <limits>
#include <memory>
#include <string>
#include <system_error>

namespace {

using flutter::EncodableList;
using flutter::EncodableMap;
using flutter::EncodableValue;

constexpr size_t kTextCapacity = 256;
constexpr DWORD kMaximumAdapters = 8;

std::string Utf8Descriptor(const wchar_t* text) {
  const int length = static_cast<int>(wcsnlen(text, kTextCapacity));
  const int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text,
                                     length, nullptr, 0, nullptr, nullptr);
  if (size <= 0) return {};
  std::string result(size, '\0');
  if (!WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text, length,
                          result.data(), size, nullptr, nullptr)) {
    return {};
  }
  return result;
}

std::string CpuModel() {
  wchar_t text[kTextCapacity]{};
  DWORD bytes = sizeof(text);
  if (RegGetValueW(HKEY_LOCAL_MACHINE,
                   L"HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0",
                   L"ProcessorNameString", RRF_RT_REG_SZ, nullptr, text,
                   &bytes) != ERROR_SUCCESS) {
    return {};
  }
  return Utf8Descriptor(text);
}

std::string DeviceProperty(HDEVINFO devices, SP_DEVINFO_DATA& device,
                           const DEVPROPKEY& key) {
  wchar_t text[kTextCapacity]{};
  DEVPROPTYPE type = 0;
  if (!SetupDiGetDevicePropertyW(devices, &device, &key, &type,
                                reinterpret_cast<PBYTE>(text), sizeof(text),
                                nullptr, 0) ||
      type != DEVPROP_TYPE_STRING) {
    return {};
  }
  return Utf8Descriptor(text);
}

struct DisplayDevices {
  HDEVINFO handle = SetupDiGetClassDevsW(&GUID_DEVCLASS_DISPLAY, nullptr,
                                       nullptr, DIGCF_PRESENT);
  ~DisplayDevices() {
    if (handle != INVALID_HANDLE_VALUE) SetupDiDestroyDeviceInfoList(handle);
  }
};

EncodableValue ReadEnvironment() {
  EncodableMap facts{{EncodableValue("cpuModel"), EncodableValue(CpuModel())}};
  MEMORYSTATUSEX memory{};
  memory.dwLength = sizeof(memory);
  if (GlobalMemoryStatusEx(&memory) &&
      memory.ullTotalPhys <= static_cast<ULONGLONG>(
                                std::numeric_limits<std::int64_t>::max())) {
    facts[EncodableValue("physicalMemoryBytes")] =
        EncodableValue(static_cast<std::int64_t>(memory.ullTotalPhys));
  }

  const DisplayDevices devices;
  EncodableList adapters;
  std::string state = "unavailable";
  if (devices.handle != INVALID_HANDLE_VALUE) {
    state = "available";
    for (DWORD index = 0; index <= kMaximumAdapters; ++index) {
      SP_DEVINFO_DATA device{};
      device.cbSize = sizeof(device);
      if (!SetupDiEnumDeviceInfo(devices.handle, index, &device)) {
        if (GetLastError() != ERROR_NO_MORE_ITEMS) state = "partial";
        break;
      }
      if (index == kMaximumAdapters) {
        state = "truncated";
        break;
      }
      const auto model =
          DeviceProperty(devices.handle, device, DEVPKEY_Device_DeviceDesc);
      const auto driver =
          DeviceProperty(devices.handle, device, DEVPKEY_Device_DriverVersion);
      if (model.empty() || driver.empty()) state = "partial";
      adapters.emplace_back(EncodableMap{
          {EncodableValue("model"), EncodableValue(model)},
          {EncodableValue("driverVersion"), EncodableValue(driver)},
      });
    }
  }
  facts[EncodableValue("graphicsAdapters")] = EncodableValue(adapters);
  facts[EncodableValue("graphicsAdapterState")] = EncodableValue(state);
  return EncodableValue(facts);
}

}  // namespace

FlutterDiagnosticEnvironment::FlutterDiagnosticEnvironment(
    flutter::BinaryMessenger* messenger)
    : channel_(messenger, "cedarflake_ame/diagnostics",
               &flutter::StandardMethodCodec::GetInstance()) {
  channel_.SetMethodCallHandler(
      [this](const flutter::MethodCall<EncodableValue>& call,
             std::unique_ptr<flutter::MethodResult<EncodableValue>> result) {
        if (call.method_name() != "getEnvironment") {
          result->NotImplemented();
          return;
        }
        if (collecting_.exchange(true)) {
          result->Error("diagnostic_environment_busy");
          return;
        }
        if (worker_.joinable()) worker_.join();
        auto reply = std::shared_ptr<flutter::MethodResult<EncodableValue>>(
            std::move(result));
        try {
          worker_ = std::thread([this, reply] {
            try {
              // The pinned Windows messenger retains and locks replies across
              // threads. Device enumeration never runs on the window thread.
              reply->Success(ReadEnvironment());
            } catch (const std::exception&) {
              reply->Error("diagnostic_environment_unavailable");
            }
            collecting_ = false;
          });
        } catch (const std::system_error&) {
          collecting_ = false;
          reply->Error("diagnostic_environment_unavailable");
        }
      });
}

FlutterDiagnosticEnvironment::~FlutterDiagnosticEnvironment() {
  channel_.SetMethodCallHandler(nullptr);
  if (worker_.joinable()) worker_.join();
}
