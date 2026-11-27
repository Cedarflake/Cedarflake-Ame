#include "user_data_cleanup.h"

#include <stdexcept>

namespace ame::installer {
namespace {

constexpr size_t kMaximumDepth = 128;
constexpr size_t kMaximumEntries = 10000000;

struct FindHandleDeleter {
  void operator()(void* handle) const noexcept { FindClose(handle); }
};
using FindHandle = std::unique_ptr<void, FindHandleDeleter>;

std::wstring ExtendedPath(const std::filesystem::path& path) {
  return L"\\\\?\\" + path.native();
}

FileHandle OpenEntry(const std::filesystem::path& path, bool removable) {
  const DWORD access = FILE_READ_ATTRIBUTES | (removable ? DELETE : 0);
  FileHandle handle(CreateFileW(ExtendedPath(path).c_str(), access,
      FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, OPEN_EXISTING,
      FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, nullptr));
  if (handle.get() == INVALID_HANDLE_VALUE) {
    const DWORD error = GetLastError();
    handle.release();
    if (error == ERROR_FILE_NOT_FOUND || error == ERROR_PATH_NOT_FOUND) return {};
    throw std::runtime_error("Cannot open Ame data for removal (Windows " +
                             std::to_string(error) + ")");
  }
  return handle;
}

DWORD Attributes(HANDLE handle) {
  BY_HANDLE_FILE_INFORMATION information{};
  if (GetFileType(handle) != FILE_TYPE_DISK || !GetFileInformationByHandle(handle, &information)) {
    throw std::runtime_error("Cannot validate an Ame data entry");
  }
  if (information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) {
    throw std::runtime_error("Ame data contains a redirected entry; it has been retained");
  }
  return information.dwFileAttributes;
}

void RemoveOpenedEntry(const std::filesystem::path& path, FileHandle handle,
                       size_t depth, size_t& entries) {
  if (depth > kMaximumDepth || ++entries > kMaximumEntries) {
    throw std::runtime_error("Ame data exceeds the bounded uninstall cleanup");
  }
  if (Attributes(handle.get()) & FILE_ATTRIBUTE_DIRECTORY) {
    WIN32_FIND_DATAW found{};
    FindHandle search(FindFirstFileExW(ExtendedPath(path / L"*").c_str(),
        FindExInfoBasic, &found, FindExSearchNameMatch, nullptr, FIND_FIRST_EX_LARGE_FETCH));
    if (search.get() == INVALID_HANDLE_VALUE) {
      const DWORD error = GetLastError();
      search.release();
      if (error != ERROR_FILE_NOT_FOUND) {
        throw std::runtime_error("Cannot enumerate Ame data for removal");
      }
    } else {
      for (;;) {
        const std::wstring name(found.cFileName);
        if (name != L"." && name != L"..") {
          const auto child_path = path / name;
          auto child = OpenEntry(child_path, true);
          if (child) RemoveOpenedEntry(child_path, std::move(child), depth + 1, entries);
        }
        if (!FindNextFileW(search.get(), &found)) {
          if (GetLastError() != ERROR_NO_MORE_FILES) {
            throw std::runtime_error("Cannot finish enumerating Ame data");
          }
          break;
        }
      }
    }
  }
  FILE_DISPOSITION_INFO disposition{TRUE};
  if (!SetFileInformationByHandle(handle.get(), FileDispositionInfo, &disposition,
                                  sizeof(disposition))) {
    throw std::runtime_error("Some Ame data could not be removed (Windows " +
                             std::to_string(GetLastError()) + ")");
  }
}

}  // namespace

std::filesystem::path AmeUserDataDirectory(const std::filesystem::path& local_app_data) {
  const auto root = local_app_data.root_name().native();
  if (!local_app_data.is_absolute() || root.size() != 2 || root[1] != L':' ||
      local_app_data != local_app_data.lexically_normal() ||
      local_app_data == local_app_data.root_path()) {
    throw std::runtime_error("The current account's local application data path is invalid");
  }
  return local_app_data / L"Packages" / L"Cedarflake.Cedarflake-Ame_t341beqjcwv0y" /
      L"LocalState" / L"Ame";
}

bool RemoveAmeUserData(const std::filesystem::path& local_app_data) {
  const auto target = AmeUserDataDirectory(local_app_data);
  auto current = target.root_path();
  std::vector<FileHandle> ancestors;
  auto root = OpenEntry(current, false);
  if (!root || !(Attributes(root.get()) & FILE_ATTRIBUTE_DIRECTORY)) {
    throw std::runtime_error("Cannot validate the Ame data volume");
  }
  ancestors.push_back(std::move(root));
  for (const auto& component : target.relative_path()) {
    current /= component;
    const bool removable = current == target;
    auto handle = OpenEntry(current, removable);
    if (!handle) return false;
    if (!(Attributes(handle.get()) & FILE_ATTRIBUTE_DIRECTORY)) {
      throw std::runtime_error("The Ame data location is not a directory");
    }
    if (removable) {
      size_t entries = 0;
      RemoveOpenedEntry(target, std::move(handle), 0, entries);
      return true;
    }
    // Pin every ancestor against replacement while resolving and deleting its children.
    ancestors.push_back(std::move(handle));
  }
  throw std::runtime_error("Cannot locate the fixed Ame data directory");
}

}  // namespace ame::installer
