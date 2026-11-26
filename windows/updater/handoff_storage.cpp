#include "handoff_storage.h"

#include <array>
#include <stdexcept>

namespace ame::updater {
namespace {
installer::FileHandle OpenRegularFile(const std::filesystem::path& path, DWORD access,
                                      DWORD sharing, DWORD disposition) {
  RejectReparse(path);
  const HANDLE raw = CreateFileW(path.c_str(), access, sharing, nullptr, disposition,
                                FILE_FLAG_OPEN_REPARSE_POINT, nullptr);
  if (raw == INVALID_HANDLE_VALUE) throw std::runtime_error("Cannot open update handoff file");
  installer::FileHandle file(raw);
  BY_HANDLE_FILE_INFORMATION information{};
  if (!GetFileInformationByHandle(raw, &information) ||
      (information.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY))) {
    throw std::runtime_error("Update handoff file is redirected");
  }
  return file;
}

void Publish(const std::filesystem::path& temporary, const std::filesystem::path& destination,
             const std::string& content, bool replace) {
  RejectReparse(destination);
  {
    auto file = OpenRegularFile(temporary, GENERIC_WRITE, 0, CREATE_NEW);
    DWORD written = 0;
    if (!WriteFile(file.get(), content.data(), static_cast<DWORD>(content.size()), &written, nullptr) ||
        written != content.size() || !FlushFileBuffers(file.get())) {
      throw std::runtime_error("Cannot write update handoff file");
    }
  }
  if (!MoveFileExW(temporary.c_str(), destination.c_str(),
      MOVEFILE_WRITE_THROUGH | (replace ? MOVEFILE_REPLACE_EXISTING : 0))) {
    throw std::runtime_error("Cannot publish update handoff file");
  }
}
}  // namespace

HandoffStorage::HandoffStorage(const std::filesystem::path& directory,
                               const std::filesystem::path& cache)
    : directory_(directory), cache_(cache) {
  if (!directory.is_absolute() || !installer::SamePath(directory.parent_path(), cache)) {
    throw std::runtime_error("Unexpected update handoff directory");
  }
  directories_ = PinDirectoryTree(directory);
  lease_ = OpenRegularFile(directory / L"lease", GENERIC_READ | GENERIC_WRITE, 0, CREATE_NEW);
}

void HandoffStorage::Acknowledge() const {
  Publish(directory_ / L"ready.tmp", directory_ / L"ready", "ready", false);
}

bool HandoffStorage::IsCommitted() const {
  try {
    auto file = OpenRegularFile(directory_ / L"commit", GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING);
    LARGE_INTEGER size{};
    if (!GetFileSizeEx(file.get(), &size) || size.QuadPart != 5) return false;
    std::array<char, 5> bytes{};
    DWORD read = 0;
    return ReadFile(file.get(), bytes.data(), static_cast<DWORD>(bytes.size()), &read, nullptr) &&
        read == bytes.size() && std::string(bytes.data(), bytes.size()) == "apply";
  } catch (const std::exception&) {
    return false;
  }
}

void HandoffStorage::WriteResult(const std::wstring& version, DWORD code) const {
  std::string ascii_version;
  for (const auto character : version) {
    if (character > 127) throw std::runtime_error("Update version is not ASCII");
    ascii_version.push_back(static_cast<char>(character));
  }
  const auto content = "{\"schema\":1,\"version\":\"" + ascii_version +
      "\",\"code\":" + std::to_string(code) + "}";
  Publish(directory_ / L"result.tmp", cache_ / L"result.json", content, true);
}

}  // namespace ame::updater
