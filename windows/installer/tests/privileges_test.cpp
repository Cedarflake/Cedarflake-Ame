#include "../policy.h"

#include <cstring>
#include <functional>
#include <stdexcept>

namespace {

void RequireRejected(const std::function<void()>& action) {
  try {
    action();
  } catch (const std::runtime_error&) {
    return;
  }
  throw std::runtime_error("Malformed service privilege buffer was admitted");
}

}  // namespace

int RunPrivilegeTests() {
  using namespace ame::installer;
  constexpr wchar_t names[] = L"SeManageVolumePrivilege\0SeImpersonatePrivilege\0";
  std::vector<BYTE> bytes(sizeof(SERVICE_REQUIRED_PRIVILEGES_INFOW) + sizeof(names));
  auto text = reinterpret_cast<wchar_t*>(bytes.data() + sizeof(SERVICE_REQUIRED_PRIVILEGES_INFOW));
  std::memcpy(text, names, sizeof(names));
  const auto parsed = ReadBrokerPrivileges(bytes, text);
  if (parsed != std::vector<std::wstring>{L"SeManageVolumePrivilege", L"SeImpersonatePrivilege"} ||
      !HasRequiredBrokerPrivileges(parsed)) {
    throw std::runtime_error("SCM MULTI_SZ did not preserve both required privileges");
  }
  if (!HasRequiredBrokerPrivileges({L"SeImpersonatePrivilege", L"SeManageVolumePrivilege"})) {
    throw std::runtime_error("SCM privilege order changed the required set");
  }
  RequireRejected([&] { ReadBrokerPrivileges(bytes, nullptr); });
  RequireRejected([&] {
    ReadBrokerPrivileges(bytes, reinterpret_cast<const wchar_t*>(bytes.data() + bytes.size()));
  });
  RequireRejected([&] {
    ReadBrokerPrivileges(bytes, reinterpret_cast<const wchar_t*>(bytes.data() + 1));
  });
  bytes.resize(bytes.size() - sizeof(wchar_t));
  text = reinterpret_cast<wchar_t*>(bytes.data() + sizeof(SERVICE_REQUIRED_PRIVILEGES_INFOW));
  RequireRejected([&] { ReadBrokerPrivileges(bytes, text); });
  if (HasRequiredBrokerPrivileges({L"SeManageVolumePrivilege", L"SeManageVolumePrivilege"}) ||
      HasRequiredBrokerPrivileges({L"SeImpersonatePrivilege", L"SeImpersonatePrivilege"}) ||
      HasRequiredBrokerPrivileges({L"SeManageVolumePrivilege", L"SeDebugPrivilege"}) ||
      HasRequiredBrokerPrivileges({})) {
    throw std::runtime_error("Duplicate, missing or foreign privileges were admitted");
  }
  return 10;
}
