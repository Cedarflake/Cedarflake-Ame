#include "../user_data_cleanup.h"

#include <winioctl.h>

#include <algorithm>
#include <fstream>
#include <functional>
#include <stdexcept>

namespace {

void Require(bool condition, const char* message) {
  if (!condition) throw std::runtime_error(message);
}

void RequireRejected(const std::function<void()>& action) {
  try {
    action();
  } catch (const std::runtime_error&) {
    return;
  }
  throw std::runtime_error("Unsafe user-data removal was admitted");
}

struct Fixture {
  std::filesystem::path root = std::filesystem::temp_directory_path() /
      (L"ame-installer-user-data-" + std::to_wstring(GetCurrentProcessId()));

  Fixture() {
    Require(std::filesystem::create_directory(root), "User-data fixture already exists");
  }
  ~Fixture() {
    std::error_code ignored;
    std::filesystem::remove_all(root, ignored);
  }

  std::filesystem::path LocalData(const wchar_t* name) const { return root / name; }
};

void Write(const std::filesystem::path& path) {
  std::filesystem::create_directories(path.parent_path());
  std::ofstream file(path, std::ios::binary);
  file << "retained-original-or-app-data";
  Require(file.good(), "Cannot create the owned data fixture");
}

bool Retained(const std::filesystem::path& path) {
  std::ifstream file(path, std::ios::binary);
  std::string text;
  std::getline(file, text);
  return text == "retained-original-or-app-data";
}

void Junction(const std::filesystem::path& link, const std::filesystem::path& target) {
  std::filesystem::create_directories(link);
  ame::installer::FileHandle directory(CreateFileW(link.c_str(), GENERIC_WRITE, 0, nullptr,
      OPEN_EXISTING, FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS, nullptr));
  if (directory.get() == INVALID_HANDLE_VALUE) {
    directory.release();
    throw std::runtime_error("Cannot open the owned junction fixture");
  }
  struct MountPoint {
    DWORD tag;
    WORD data_length;
    WORD reserved;
    WORD substitute_offset;
    WORD substitute_length;
    WORD print_offset;
    WORD print_length;
    wchar_t paths[2048];
  } data{};
  const std::wstring substitute = L"\\??\\" + target.native();
  const std::wstring print = target.native();
  Require(substitute.size() + print.size() + 2 < 2048, "Junction fixture path is too long");
  data.tag = IO_REPARSE_TAG_MOUNT_POINT;
  data.substitute_length = static_cast<WORD>(substitute.size() * sizeof(wchar_t));
  data.print_offset = static_cast<WORD>((substitute.size() + 1) * sizeof(wchar_t));
  data.print_length = static_cast<WORD>(print.size() * sizeof(wchar_t));
  data.data_length = static_cast<WORD>(8 + data.print_offset + data.print_length + sizeof(wchar_t));
  std::copy(substitute.begin(), substitute.end(), data.paths);
  std::copy(print.begin(), print.end(), data.paths + substitute.size() + 1);
  DWORD written = 0;
  Require(DeviceIoControl(directory.get(), FSCTL_SET_REPARSE_POINT, &data,
      static_cast<DWORD>(8 + data.data_length), nullptr, 0, &written, nullptr) != FALSE,
      "Cannot create the owned junction fixture");
}

struct JunctionFixture {
  std::filesystem::path link;

  JunctionFixture(const std::filesystem::path& path, const std::filesystem::path& target)
      : link(path) {
    Junction(link, target);
  }
  ~JunctionFixture() { RemoveDirectoryW(link.c_str()); }
};

void CheckRemovalRequests() {
  using ame::installer::ShouldRemoveUserData;
  Require(ShouldRemoveUserData({true, L"ALL", false, L"1"}), "Explicit uninstall was rejected");
  const std::vector<ame::installer::UserDataRemovalRequest> retained{
      {false, L"ALL", false, L"1"},
      {true, L"ALL", true, L"1"},
      {true, L"", false, L"1"},
      {true, L"AmeDesktopFeature", false, L"1"},
      {true, L"ALL", false, L""},
      {true, L"ALL", false, L"0"},
      {true, L"ALL", false, L"true"}};
  for (const auto& request : retained) {
    Require(!ShouldRemoveUserData(request), "Install, repair, upgrade or absent consent deletes data");
  }
}

void CheckFixedDirectoryAndIdentity() {
  using namespace ame::installer;
  RequireRejected([] { AmeUserDataDirectory(L"C:\\"); });
  RequireRejected([] { AmeUserDataDirectory(L"relative"); });
  RequireRejected([] { AmeUserDataDirectory(L"C:\\Users\\example\\..\\other"); });
  RequireRejected([] { AmeUserDataDirectory(L"\\\\server\\share\\data"); });
  RequireRejected([] { ResolveCurrentUserData(L"S-1-5-18"); });
  RequireRejected([] { ResolveCurrentUserData(L"S-1-5-19"); });
  RequireRejected([] { ResolveCurrentUserData(L"S-1-5-20"); });
  RequireRejected([] { ResolveCurrentUserData(L"not-an-account"); });
  RequireRejected([] { ResolveCurrentUserData(L"S-1-5-21-0-0-0-987654"); });
}

void CheckLease() {
  const auto sid = L"S-1-5-21-0-0-0-" + std::to_wstring(GetCurrentProcessId());
  {
    const auto lease = ame::installer::AcquireUserDataCleanupLease(sid);
    RequireRejected([&] { ame::installer::AcquireUserDataCleanupLease(sid); });
  }
  const auto reacquired = ame::installer::AcquireUserDataCleanupLease(sid);
  Require(static_cast<bool>(reacquired), "Cleanup did not release the cross-distribution lease");
}

void CheckNormalRemoval(const Fixture& fixture) {
  using namespace ame::installer;
  const auto local = fixture.LocalData(L"normal");
  const auto data = AmeUserDataDirectory(local);
  const auto original = fixture.root / L"pictures" / L"原图片.jpg";
  const auto sibling = data.parent_path() / L"other-app-data" / L"retained.txt";
  Write(original);
  Write(sibling);
  Write(data / L"图库" / L"catalog.sqlite3");
  Write(data / L"cache" / L"preview.bin");
  Require(CreateHardLinkW((data / L"linked-original.jpg").c_str(), original.c_str(), nullptr) != FALSE,
          "Cannot create the owned hard-link fixture");
  Require(RemoveAmeUserData(local), "Existing Ame data was not removed");
  Require(!std::filesystem::exists(data), "Ame data remains after successful removal");
  Require(Retained(original) && Retained(sibling), "Cleanup changed source bytes or sibling data");
  Require(std::filesystem::is_directory(data.parent_path()), "Cleanup deleted the package data owner");
  Require(!RemoveAmeUserData(local), "Repeated cleanup should accept already absent data");
}

void CheckLockedData(const Fixture& fixture) {
  using namespace ame::installer;
  const auto local = fixture.LocalData(L"locked");
  const auto locked_path = AmeUserDataDirectory(local) / L"catalog.sqlite3";
  Write(locked_path);
  FileHandle locked(CreateFileW(locked_path.c_str(), GENERIC_READ, FILE_SHARE_READ,
      nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr));
  if (locked.get() == INVALID_HANDLE_VALUE) {
    locked.release();
    throw std::runtime_error("Cannot lock the owned data fixture");
  }
  RequireRejected([&] { RemoveAmeUserData(local); });
  Require(Retained(locked_path), "Locked data changed during a rejected cleanup");
  locked.reset();
  Require(RemoveAmeUserData(local), "Cleanup could not retry after the file was closed");
}

void CheckRedirectedData(const Fixture& fixture) {
  using namespace ame::installer;
  const auto original = fixture.root / L"pictures" / L"原图片.jpg";
  for (const auto* level : {L"root-link", L"child-link", L"ancestor-link"}) {
    const auto local = fixture.LocalData(level);
    const auto data = AmeUserDataDirectory(local);
    std::filesystem::path link = data;
    if (std::wstring(level) == L"child-link") link /= L"redirected";
    if (std::wstring(level) == L"ancestor-link") link = local / L"Packages";
    const JunctionFixture junction(link, original.parent_path());
    RequireRejected([&] { RemoveAmeUserData(local); });
    Require(Retained(original), "Redirected cleanup changed the original fixture");
    Require(RemoveDirectoryW(link.c_str()) != FALSE, "Cannot retire the owned junction fixture");
  }
}

}  // namespace

int RunUserDataCleanupTests() {
  CheckRemovalRequests();
  CheckFixedDirectoryAndIdentity();
  CheckLease();
  const Fixture fixture;
  CheckNormalRemoval(fixture);
  CheckLockedData(fixture);
  CheckRedirectedData(fixture);
  return 24;
}
