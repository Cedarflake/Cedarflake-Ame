#include "package.h"
#include "handoff_session.h"

#include <shellapi.h>

#include <optional>
#include <stdexcept>

namespace {
using ame::installer::FileHandle;

bool MatchesInstalledApplication(const std::filesystem::path& application) {
  const auto product = ame::installer::ReadRegisteredProduct();
  return product && ame::installer::SamePath(application,
      product->application_directory / L"cedarflake_ame.exe");
}

DWORD RunInstaller(const std::filesystem::path& package) {
  wchar_t system[32768]{};
  const UINT size = GetSystemDirectoryW(system, 32768);
  if (!size || size >= 32768) throw std::runtime_error("Cannot locate Windows Installer");
  const auto executable = std::filesystem::path(system) / L"msiexec.exe";
  const auto arguments = L"/i " + ame::updater::QuoteArgument(package.native()) + L" /passive /norestart";
  SHELLEXECUTEINFOW info{};
  info.cbSize = sizeof(info);
  info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
  info.lpVerb = L"runas";
  info.lpFile = executable.c_str();
  info.lpParameters = arguments.c_str();
  info.nShow = SW_SHOWNORMAL;
  if (!ShellExecuteExW(&info)) return GetLastError();
  const FileHandle process(info.hProcess);
  if (!process || WaitForSingleObject(process.get(), INFINITE) != WAIT_OBJECT_0) {
    throw std::runtime_error("Windows Installer could not be joined");
  }
  DWORD result = ERROR_INSTALL_FAILURE;
  if (!GetExitCodeProcess(process.get(), &result)) throw std::runtime_error("Cannot read Windows Installer result");
  return result;
}

DWORD RunPortableUpdate(const std::filesystem::path& executable,
                        const std::filesystem::path& root, const std::filesystem::path& package) {
  ame::updater::RejectReparse(executable);
  auto arguments = ame::updater::QuoteArgument(executable.native()) + L" apply --package " +
      ame::updater::QuoteArgument(package.native()) + L" --root " +
      ame::updater::QuoteArgument(root.native()) + L" --packageDir " +
      ame::updater::QuoteArgument((root / L"packages").native()) + L" --noRestart";
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  if (!CreateProcessW(executable.c_str(), arguments.data(), nullptr, nullptr, FALSE,
      0, nullptr, root.c_str(), &startup, &process)) {
    throw std::runtime_error("Cannot start the portable update engine");
  }
  const FileHandle child(process.hProcess);
  CloseHandle(process.hThread);
  if (WaitForSingleObject(child.get(), INFINITE) != WAIT_OBJECT_0) {
    return ERROR_INSTALL_FAILURE;
  }
  DWORD code = ERROR_INSTALL_FAILURE;
  if (!GetExitCodeProcess(child.get(), &code)) return ERROR_INSTALL_FAILURE;
  return code;
}

int Apply(wchar_t** arguments, bool portable) {
  const DWORD parent_id = std::stoul(arguments[2]);
  const std::filesystem::path application(arguments[3]);
  const std::filesystem::path package(arguments[4]);
  const std::wstring hash(arguments[5]);
  const auto size = std::stoull(arguments[6]);
  const std::filesystem::path ready(arguments[7]);
  const std::filesystem::path result_file(arguments[8]);
  const std::wstring version(arguments[9]);
  const auto root = application.parent_path().parent_path();
  const bool matches = portable
      ? application.parent_path().filename() == L"current" && std::filesystem::is_regular_file(root / L".portable")
      : MatchesInstalledApplication(application);
  if (!matches) throw std::runtime_error("Unexpected application update installation");
  ame::updater::HandoffSession session(parent_id, application, ready, result_file, version);
  if (portable) {
    ame::updater::RejectReparse(application);
    ame::updater::RejectReparse(root / L".portable");
    ame::updater::RejectReparse(root / L"Update.exe");
    if (!std::filesystem::is_regular_file(root / L"Update.exe")) {
      throw std::runtime_error("Portable update engine is missing");
    }
  }
  const auto package_lease = ame::updater::VerifyPackage(package, hash, size, portable ? root : std::filesystem::path());
  const auto engine = session.directory() / L"Update.exe";
  const auto engine_lease = portable
      ? std::make_optional(ame::updater::VerifyEngine(engine, arguments[10], std::stoull(arguments[11])))
      : std::nullopt;
  session.Acknowledge();
  if (!session.WaitForParent()) return 2;
  if (!session.IsCommitted()) return session.Reopen(ERROR_INSTALL_FAILURE);
  if (portable) {
    DWORD code = ERROR_INSTALL_FAILURE;
    try {
      code = RunPortableUpdate(engine, root, package);
    }
    catch (const std::exception&) { code = ERROR_INSTALL_FAILURE; }
    return session.Reopen(code);
  }
  DWORD code = ERROR_INSTALL_FAILURE;
  try {
    if (MatchesInstalledApplication(application)) code = RunInstaller(package);
  } catch (const std::exception&) { code = ERROR_INSTALL_FAILURE; }
  return session.Reopen(code);
}

int Recover(wchar_t** arguments) {
  ame::updater::HandoffSession session(std::stoul(arguments[2]), arguments[3],
                                      arguments[4], arguments[5], arguments[6]);
  session.Acknowledge();
  if (!session.WaitForParent()) return 2;
  return session.Reopen(ERROR_INSTALL_FAILURE);
}
}  // namespace

int APIENTRY wWinMain(HINSTANCE, HINSTANCE, wchar_t*, int) {
  int count = 0;
  wchar_t** arguments = CommandLineToArgvW(GetCommandLineW(), &count);
  const ame::installer::LocalAllocation allocation(arguments);
  if (!arguments) return 2;
  const HRESULT com = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
  if (FAILED(com)) return 2;
  int code = 2;
  try {
    if (count == 3 && std::wstring(arguments[1]) == L"--probe") {
      code = MatchesInstalledApplication(arguments[2]) ? 0 : 1;
    } else if (count == 7 && std::wstring(arguments[1]) == L"--recover") {
      code = Recover(arguments);
    } else if (count == 12 && (std::wstring(arguments[1]) == L"--apply" || std::wstring(arguments[1]) == L"--portable")) {
      code = Apply(arguments, std::wstring(arguments[1]) == L"--portable");
    }
  } catch (const std::exception&) { code = 2; }
  CoUninitialize();
  return code;
}
