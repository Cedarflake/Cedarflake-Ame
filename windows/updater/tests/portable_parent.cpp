#include "../package.h"
#include "../../runner/single_instance_lock.h"

#include <shlobj.h>

#include <fstream>
#include <iostream>
#include <stdexcept>

namespace {
using ame::installer::FileHandle;

void Require(bool condition, const char* detail) {
  if (!condition) throw std::runtime_error(detail);
}

std::string Read(const std::filesystem::path& path) {
  std::ifstream input(path, std::ios::binary);
  Require(static_cast<bool>(input), "Cannot read fixture input");
  return {std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>()};
}

void Write(const std::filesystem::path& path, const std::string& value) {
  const auto temporary = path.native() + L".writing";
  std::ofstream output(temporary, std::ios::binary | std::ios::trunc);
  output << value;
  output.close();
  Require(static_cast<bool>(output), "Cannot write fixture evidence");
  Require(MoveFileExW(temporary.c_str(), path.c_str(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) != FALSE,
          "Cannot publish fixture evidence");
}

std::filesystem::path Application() {
  wchar_t path[32768]{};
  DWORD length = 32768;
  Require(QueryFullProcessImageNameW(GetCurrentProcess(), 0, path, &length) != FALSE,
          "Cannot locate fixture application");
  return std::wstring(path, length);
}

std::filesystem::path Evidence(const std::filesystem::path& application, bool portable) {
  if (portable) return application.parent_path().parent_path();
  PWSTR local = nullptr;
  Require(SUCCEEDED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, nullptr, &local)),
          "Cannot locate installed fixture evidence");
  const auto path = std::filesystem::path(local) / L"AmeInstalledFixture";
  CoTaskMemFree(local);
  return path;
}

int InspectToken(DWORD id) {
  const FileHandle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, id ? id : GetCurrentProcessId()));
  Require(static_cast<bool>(process), "Cannot inspect fixture process");
  HANDLE raw = nullptr;
  Require(OpenProcessToken(process.get(), TOKEN_QUERY, &raw) != FALSE, "Cannot open fixture process token");
  const FileHandle token(raw);
  TOKEN_ELEVATION elevation{};
  TOKEN_ELEVATION_TYPE type{};
  DWORD returned = 0;
  Require(GetTokenInformation(raw, TokenElevation, &elevation, sizeof(elevation), &returned) &&
          GetTokenInformation(raw, TokenElevationType, &type, sizeof(type), &returned),
          "Cannot inspect fixture token elevation");
  TOKEN_LINKED_TOKEN linked{};
  const bool has_linked = GetTokenInformation(raw, TokenLinkedToken, &linked, sizeof(linked), &returned) != FALSE;
  if (has_linked) CloseHandle(linked.LinkedToken);
  std::cout << "{\"elevated\":" << (elevation.TokenIsElevated ? "true" : "false")
            << ",\"type\":" << static_cast<int>(type)
            << ",\"linked\":" << (has_linked ? "true" : "false") << "}\n";
  return 0;
}

FileHandle Launch(const std::filesystem::path& executable, const std::vector<std::wstring>& arguments) {
  auto command = ame::updater::QuoteArgument(executable.native());
  for (const auto& argument : arguments) command += L" " + ame::updater::QuoteArgument(argument);
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  Require(CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, FALSE,
      CREATE_NO_WINDOW, nullptr, executable.parent_path().c_str(), &startup, &process) != FALSE,
      "Cannot start shipping handoff helper");
  CloseHandle(process.hThread);
  return FileHandle(process.hProcess);
}

void Acknowledge(HANDLE process, const std::filesystem::path& ready) {
  const auto deadline = GetTickCount64() + 30000;
  while (!std::filesystem::exists(ready)) {
    Require(WaitForSingleObject(process, 0) == WAIT_TIMEOUT, "Handoff helper exited before readiness");
    Require(GetTickCount64() < deadline, "Handoff readiness timed out");
    Sleep(20);
  }
  Require(Read(ready) == "ready", "Handoff acknowledgement differs");
}

void Commit(const std::filesystem::path& directory) {
  Write(directory / L"commit.tmp", "apply");
  Require(MoveFileExW((directory / L"commit.tmp").c_str(), (directory / L"commit").c_str(),
      MOVEFILE_WRITE_THROUGH) != FALSE, "Cannot publish fixture activation");
}

std::filesystem::path PrepareDirectory(const std::filesystem::path& application, const wchar_t* suffix) {
  const auto directory = ame::updater::UpdateCache() /
      (L"handoff-fixture-" + std::to_wstring(GetCurrentProcessId()) + suffix);
  Require(!std::filesystem::exists(directory), "Fixture handoff already exists");
  std::filesystem::create_directories(directory);
  std::filesystem::copy_file(application.parent_path() / L"cedarflake_ame_update.exe",
                             directory / L"cedarflake_ame_update.exe");
  return directory;
}

int Begin(const std::filesystem::path& application, wchar_t** arguments,
          std::unique_ptr<SingleInstanceLock>& instance, bool portable) {
  const auto root = application.parent_path().parent_path();
  const auto evidence = Evidence(application, portable);
  const auto version = portable ? application.parent_path() / L"fixture-version.txt"
                                : evidence / L"fixture-version.txt";
  const auto original_version = Read(version);
  const std::wstring mode(arguments[2]);
  Require(mode == L"commit" || mode == L"no-commit" || mode == L"dead-helper", "Unknown fixture mode");
  auto directory = PrepareDirectory(application, L"-update");
  if (portable) std::filesystem::copy_file(root / L"target-engine.exe", directory / L"Update.exe");
  const auto result = ame::updater::UpdateCache() / L"result.json";
  const auto package = (portable ? root / L"packages" : ame::updater::UpdateCache() / arguments[4]) / arguments[3];
  auto helper = Launch(directory / L"cedarflake_ame_update.exe", {
      portable ? L"--portable" : L"--apply", std::to_wstring(GetCurrentProcessId()), application.native(), package.native(),
      arguments[4], arguments[5], (directory / L"ready").native(), result.native(),
      arguments[6], arguments[7], arguments[8]});
  Acknowledge(helper.get(), directory / L"ready");
  if (mode == L"dead-helper") {
    Require(TerminateProcess(helper.get(), 99) != FALSE, "Cannot inject owned helper failure");
    Require(WaitForSingleObject(helper.get(), 10000) == WAIT_OBJECT_0, "Failed helper did not retire");
    directory = PrepareDirectory(application, L"-recovery");
    helper = Launch(directory / L"cedarflake_ame_update.exe", {
        L"--recover", std::to_wstring(GetCurrentProcessId()), application.native(),
        (directory / L"ready").native(), result.native(), arguments[6]});
    Acknowledge(helper.get(), directory / L"ready");
    Commit(directory);  // Recovery mode must ignore even a complete activation marker.
  } else if (mode == L"commit") {
    Commit(directory);
  }
  Write(evidence / L"helper-process.txt", std::to_string(GetProcessId(helper.get())));
  instance.reset();
  const auto competing = Launch(application, {L"--race-probe"});
  Require(WaitForSingleObject(competing.get(), 10000) == WAIT_OBJECT_0,
          "Competing application did not retire");
  DWORD competing_code = 0;
  Require(GetExitCodeProcess(competing.get(), &competing_code) && competing_code == 42,
          "Handoff did not retain the application startup guard");
  Write(evidence / L"startup-guard.txt", "competing launch rejected after parent released its own handle");
  Sleep(300);
  Require(Read(version) == original_version, "Update ran before parent exit");
  Require(!std::filesystem::exists(evidence / L"restarted.txt"), "Application restarted before parent exit");
  Require(WaitForSingleObject(helper.get(), 0) == WAIT_TIMEOUT, "Handoff did not wait for the parent");
  Write(evidence / L"parent-exit.txt", "ready acknowledged; activation policy checked; parent exits normally");
  return 0;
}
}  // namespace

int wmain(int count, wchar_t** arguments) {
  const auto application = Application();
  const auto root = application.parent_path().parent_path();
  const bool portable = std::filesystem::is_regular_file(root / L".portable");
  const auto evidence = Evidence(application, portable);
  try {
    if (count == 3 && std::wstring(arguments[1]) == L"--inspect-token") {
      return InspectToken(std::stoul(arguments[2]));
    }
    if (count > 1 && std::wstring(arguments[1]).rfind(L"--veloapp-", 0) == 0) return 0;
    auto instance = std::make_unique<SingleInstanceLock>();
    const auto acquired = instance->Acquire();
    if (count == 2 && std::wstring(arguments[1]) == L"--race-probe") {
      return acquired == SingleInstanceLockResult::kAlreadyRunning ? 42 : 2;
    }
    Require(acquired == SingleInstanceLockResult::kAcquired, "Fixture application startup guard was not available");
    if (count == 9 && std::wstring(arguments[1]) == (portable ? L"--begin" : L"--begin-installed")) {
      return Begin(application, arguments, instance, portable);
    }
    Require(count == 1, "Unexpected fixture arguments");
    Write(evidence / L"restarted.txt", Read(portable ? application.parent_path() / L"fixture-version.txt"
                                                  : evidence / L"fixture-version.txt"));
    return 0;
  } catch (const std::exception& error) {
    Write(evidence / L"parent-error.txt", error.what());
    std::cerr << error.what() << '\n';
    return 2;
  }
}
