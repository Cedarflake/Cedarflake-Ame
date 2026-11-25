#include "package.h"

#include <bcrypt.h>
#include <msiquery.h>
#include <shlobj.h>

#include <algorithm>
#include <array>
#include <stdexcept>

namespace ame::updater {

std::filesystem::path UpdateCache() {
  PWSTR raw = nullptr;
  if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, nullptr, &raw))) {
    throw std::runtime_error("Cannot locate application update storage");
  }
  const std::filesystem::path local(raw);
  CoTaskMemFree(raw);
  return local / L"Cedarflake" / L"Ame" / L"cache" / L"application-updates";
}

void RejectReparse(const std::filesystem::path& path) {
  for (auto current = path; !current.empty(); current = current.parent_path()) {
    const auto attributes = GetFileAttributesW(current.c_str());
    if (attributes != INVALID_FILE_ATTRIBUTES && (attributes & FILE_ATTRIBUTE_REPARSE_POINT)) {
      throw std::runtime_error("Application update path is redirected");
    }
    if (current == current.root_path()) break;
  }
}

std::vector<installer::FileHandle> PinDirectoryTree(const std::filesystem::path& directory) {
  if (!directory.is_absolute()) throw std::runtime_error("Update directory must be absolute");
  std::vector<std::filesystem::path> paths;
  for (auto current = directory; !current.empty(); current = current.parent_path()) {
    paths.push_back(current);
    if (current == current.root_path()) break;
  }
  std::vector<installer::FileHandle> handles;
  for (auto entry = paths.rbegin(); entry != paths.rend(); ++entry) {
    const HANDLE raw = CreateFileW(entry->c_str(), FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, nullptr);
    if (raw == INVALID_HANDLE_VALUE) throw std::runtime_error("Cannot pin update directory");
    installer::FileHandle handle(raw);
    BY_HANDLE_FILE_INFORMATION information{};
    if (!GetFileInformationByHandle(raw, &information) ||
        !(information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ||
        (information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT)) {
      throw std::runtime_error("Update directory is redirected");
    }
    handles.push_back(std::move(handle));
  }
  return handles;
}

std::wstring QuoteArgument(const std::wstring& value) {
  std::wstring result = L"\"";
  size_t slashes = 0;
  for (const auto character : value) {
    if (character == L'\\') { ++slashes; continue; }
    result.append(slashes * (character == L'"' ? 2 : 1), L'\\');
    slashes = 0;
    if (character == L'"') result.push_back(L'\\');
    result.push_back(character);
  }
  result.append(slashes * 2, L'\\');
  result.push_back(L'"');
  return result;
}

namespace {
VerifiedPackage VerifyFile(const std::filesystem::path& path,
                           const std::wstring& expected, unsigned long long length) {
  if (length == 0 || length > 2ULL * 1024 * 1024 * 1024 || expected.size() != 64 ||
      !std::all_of(expected.begin(), expected.end(), [](wchar_t c) {
        return (c >= L'0' && c <= L'9') || (c >= L'a' && c <= L'f');
      })) {
    throw std::runtime_error("Unexpected application update package");
  }
  auto directories = PinDirectoryTree(path.parent_path());
  RejectReparse(path);
  const HANDLE raw = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
                                OPEN_EXISTING, FILE_FLAG_OPEN_REPARSE_POINT, nullptr);
  if (raw == INVALID_HANDLE_VALUE) throw std::runtime_error("Cannot open update package");
  installer::FileHandle file(raw);
  BY_HANDLE_FILE_INFORMATION information{};
  if (!GetFileInformationByHandle(raw, &information) ||
      (information.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT))) {
    throw std::runtime_error("Application update package is redirected");
  }
  LARGE_INTEGER size{};
  if (!GetFileSizeEx(raw, &size) || static_cast<unsigned long long>(size.QuadPart) != length) {
    throw std::runtime_error("Application update size differs");
  }
  BCRYPT_HASH_HANDLE hash = nullptr;
  if (BCryptCreateHash(BCRYPT_SHA256_ALG_HANDLE, &hash, nullptr, 0, nullptr, 0, 0) < 0) {
    throw std::runtime_error("Cannot initialize application update hash");
  }
  const auto release_hash = [](void* value) { BCryptDestroyHash(value); };
  const std::unique_ptr<void, decltype(release_hash)> hash_guard(hash, release_hash);
  std::array<BYTE, 128 * 1024> buffer{};
  for (;;) {
    DWORD read = 0;
    if (!ReadFile(raw, buffer.data(), static_cast<DWORD>(buffer.size()), &read, nullptr)) {
      throw std::runtime_error("Cannot read application update package");
    }
    if (!read) break;
    if (BCryptHashData(hash, buffer.data(), read, 0) < 0) throw std::runtime_error("Cannot hash application update");
  }
  std::array<BYTE, 32> digest{};
  if (BCryptFinishHash(hash, digest.data(), static_cast<ULONG>(digest.size()), 0) < 0) {
    throw std::runtime_error("Cannot finish application update hash");
  }
  constexpr wchar_t hex[] = L"0123456789abcdef";
  std::wstring actual;
  for (BYTE value : digest) { actual.push_back(hex[value >> 4]); actual.push_back(hex[value & 15]); }
  if (actual != expected) throw std::runtime_error("Application update hash differs");
  return {std::move(directories), std::move(file)};
}
}  // namespace

VerifiedPackage VerifyEngine(const std::filesystem::path& path,
                             const std::wstring& expected, unsigned long long length) {
  if (path.filename() != L"Update.exe" || length > 64ULL * 1024 * 1024 ||
      path.parent_path().filename().wstring().rfind(L"handoff-", 0) != 0 ||
      !installer::SamePath(path.parent_path().parent_path(), UpdateCache())) {
    throw std::runtime_error("Unexpected staged update engine");
  }
  return VerifyFile(path, expected, length);
}

VerifiedPackage VerifyPackage(const std::filesystem::path& path,
                                  const std::wstring& expected, unsigned long long length,
                                  const std::filesystem::path& portable_root) {
  const bool portable = !portable_root.empty();
  const auto directory = portable ? portable_root / L"packages" : UpdateCache() / expected;
  if (path.extension() != (portable ? L".nupkg" : L".msi") ||
      !installer::SamePath(path.parent_path(), directory)) {
    throw std::runtime_error("Unexpected application update package");
  }
  auto verified = VerifyFile(path, expected, length);
  if (portable) return verified;

  MSIHANDLE database = 0;
  if (MsiOpenDatabaseW(path.c_str(), MSIDBOPEN_READONLY, &database) != ERROR_SUCCESS) {
    throw std::runtime_error("Cannot read application update identity");
  }
  MSIHANDLE view = 0;
  MSIHANDLE record = 0;
  bool admitted = MsiDatabaseOpenViewW(database,
      L"SELECT `Value` FROM `Property` WHERE `Property` = 'UpgradeCode'", &view) == ERROR_SUCCESS;
  admitted = admitted && MsiViewExecute(view, 0) == ERROR_SUCCESS && MsiViewFetch(view, &record) == ERROR_SUCCESS;
  wchar_t code[39]{};
  DWORD capacity = 39;
  admitted = admitted && MsiRecordGetStringW(record, 1, code, &capacity) == ERROR_SUCCESS &&
             std::wstring(code) == installer::kUpgradeCode;
  if (record) MsiCloseHandle(record);
  if (view) MsiCloseHandle(view);
  MsiCloseHandle(database);
  if (!admitted) throw std::runtime_error("Application update product identity differs");
  return verified;
}

}  // namespace ame::updater
