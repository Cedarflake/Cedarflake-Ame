#include "../handoff_storage.h"

#include <shellapi.h>
#include <winioctl.h>

#include <algorithm>
#include <array>
#include <fstream>
#include <iostream>
#include <stdexcept>

namespace {
void Require(bool value, const char* detail) {
  if (!value) throw std::runtime_error(detail);
}

template <typename Action>
void Reject(Action action, const char* detail) {
  try { action(); } catch (const std::exception&) { return; }
  throw std::runtime_error(detail);
}

void Write(const std::filesystem::path& path, const std::string& contents) {
  std::ofstream stream(path, std::ios::binary | std::ios::trunc);
  stream << contents;
  stream.close();
  Require(static_cast<bool>(stream), "Cannot write test fixture");
}

std::string Read(const std::filesystem::path& path) {
  std::ifstream stream(path, std::ios::binary);
  return std::string(std::istreambuf_iterator<char>(stream), std::istreambuf_iterator<char>());
}

void Junction(const std::filesystem::path& link, const std::filesystem::path& target) {
  std::filesystem::create_directory(link);
  const HANDLE raw = CreateFileW(link.c_str(), GENERIC_WRITE, 0, nullptr, OPEN_EXISTING,
      FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS, nullptr);
  Require(raw != INVALID_HANDLE_VALUE, "Cannot open junction fixture");
  const ame::installer::FileHandle handle(raw);
  const std::wstring substitute = L"\\??\\" + target.native();
  struct MountPoint {
    DWORD tag = IO_REPARSE_TAG_MOUNT_POINT;
    WORD length = 0;
    WORD reserved = 0;
    WORD substitute_offset = 0;
    WORD substitute_length = 0;
    WORD print_offset = 0;
    WORD print_length = 0;
    std::array<wchar_t, 4096> paths{};
  } point;
  Require(substitute.size() + target.native().size() + 2 < point.paths.size(), "Junction fixture path is too long");
  point.substitute_length = static_cast<WORD>(substitute.size() * sizeof(wchar_t));
  point.print_offset = static_cast<WORD>(point.substitute_length + sizeof(wchar_t));
  point.print_length = static_cast<WORD>(target.native().size() * sizeof(wchar_t));
  point.length = static_cast<WORD>(8 + point.print_offset + point.print_length + sizeof(wchar_t));
  std::copy(substitute.begin(), substitute.end(), point.paths.begin());
  std::copy(target.native().begin(), target.native().end(), point.paths.begin() + substitute.size() + 1);
  DWORD returned = 0;
  Require(DeviceIoControl(raw, FSCTL_SET_REPARSE_POINT, &point, point.length + 8,
      nullptr, 0, &returned, nullptr) != FALSE, "Cannot create junction fixture");
}

void StorageProtocol(const std::filesystem::path& cache) {
  const auto directory = cache / L"handoff-fixture";
  std::filesystem::create_directories(directory);
  const auto marker = directory / L"commit";
  const auto original = cache / L"preserved.txt";
  Write(original, "preserve this unrelated content");
  if (!CreateHardLinkW((cache / L"result.json").c_str(), original.c_str(), nullptr)) {
    throw std::runtime_error("Cannot prepare linked result fixture: " + std::to_string(GetLastError()));
  }
  {
    const ame::updater::HandoffStorage storage(directory, cache);
    storage.Acknowledge();
    Require(Read(directory / L"ready") == "ready", "Ready acknowledgement was not fully published");
    const HANDLE competing = CreateFileW((directory / L"lease").c_str(), GENERIC_READ, 0,
        nullptr, OPEN_EXISTING, 0, nullptr);
    Require(competing == INVALID_HANDLE_VALUE, "Active handoff lease allowed another owner");
    Require(!MoveFileExW(cache.c_str(), (cache.native() + L"-moved").c_str(), 0),
            "Pinned update cache was movable");
    Require(!storage.IsCommitted(), "Missing commit activated replacement");
    for (const auto* content : {"", "appl", "applyx", "wrong"}) {
      Write(marker, content);
      Require(!storage.IsCommitted(), "Incomplete or corrupt commit activated replacement");
    }
    Write(marker, "apply");
    Require(storage.IsCommitted(), "Complete commit did not activate replacement");
    storage.WriteResult(L"0.2.0-beta.1+42", ERROR_SUCCESS);
    Require(Read(original) == "preserve this unrelated content", "Result publication truncated another file");
    Require(Read(cache / L"result.json") == "{\"schema\":1,\"version\":\"0.2.0-beta.1+42\",\"code\":0}",
            "Update result changed its version or completion code");
    storage.WriteResult(L"0.2.0-beta.1+42", ERROR_INSTALL_USEREXIT);
    Require(Read(cache / L"result.json").find("\"code\":1602}") != std::string::npos,
            "Later installation failure did not replace the result");
    std::filesystem::remove(cache / L"result.json");
    Junction(cache / L"result.json", cache);
    Reject([&] { storage.WriteResult(L"0.2.0", ERROR_SUCCESS); }, "Redirected result path was accepted");
    Require(RemoveDirectoryW((cache / L"result.json").c_str()) != FALSE, "Cannot retire junction fixture");
  }
  Require(MoveFileExW(cache.c_str(), (cache.native() + L"-moved").c_str(), 0) != FALSE,
          "Retired handoff retained its directory handles");
  Require(MoveFileExW((cache.native() + L"-moved").c_str(), cache.c_str(), 0) != FALSE,
          "Cannot restore fixture cache");
}

void PackageLease(const std::filesystem::path& root) {
  const auto packages = root / L"packages";
  std::filesystem::create_directories(packages);
  const auto package = packages / L"Ame-full.nupkg";
  Write(package, "abc");
  const std::wstring hash = L"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
  {
    const auto lease = ame::updater::VerifyPackage(package, hash, 3, root);
    const HANDLE write = CreateFileW(package.c_str(), GENERIC_WRITE, FILE_SHARE_READ,
        nullptr, OPEN_EXISTING, 0, nullptr);
    Require(write == INVALID_HANDLE_VALUE, "Verified package remained writable during handoff");
    Require(!DeleteFileW(package.c_str()), "Verified package could be removed during handoff");
    Require(!MoveFileExW(packages.c_str(), (root / L"replaced-packages").c_str(), 0),
            "Verified package directory could be substituted during handoff");
  }
  Reject([&] { ame::updater::VerifyPackage(package, hash, 4, root); }, "Wrong package length was accepted");
  Write(package, "abd");
  Reject([&] { ame::updater::VerifyPackage(package, hash, 3, root); }, "Corrupt retained package was accepted");
  Reject([&] { ame::updater::VerifyPackage(package, hash, 3, root / L"other"); }, "Package outside the engine cache was accepted");
}

void QuotedArguments() {
  const std::vector<std::wstring> values = {L"", L"C:\\路径 with spaces\\", L"with\"quote", L"plain"};
  auto command = std::wstring(L"fixture.exe");
  for (const auto& value : values) command += L" " + ame::updater::QuoteArgument(value);
  int count = 0;
  auto** arguments = CommandLineToArgvW(command.c_str(), &count);
  const ame::installer::LocalAllocation allocation(arguments);
  Require(arguments && count == static_cast<int>(values.size()) + 1, "Quoted argument count changed");
  for (size_t i = 0; i < values.size(); ++i) {
    Require(values[i] == arguments[i + 1], "Quoted path did not survive Windows command-line parsing");
  }
}
}  // namespace

int wmain(int count, wchar_t** arguments) {
  try {
    Require(count == 2, "Pass an isolated fixture directory");
    const auto root = std::filesystem::absolute(arguments[1]);
    ame::updater::RejectReparse(root);
    std::filesystem::create_directories(root);
    StorageProtocol(root / L"storage");
    PackageLease(root / L"portable");
    QuotedArguments();
    std::cout << "update_handoff: storage commit, result safety, package lease and argument cases passed\n";
    return 0;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
