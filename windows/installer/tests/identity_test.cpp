#include "../policy.h"

#include <functional>
#include <stdexcept>
#include <vector>

namespace {

void RequireRejected(const std::function<void()>& action) {
  try {
    action();
  } catch (const std::runtime_error&) {
    return;
  }
  throw std::runtime_error("Invalid broker identity was admitted");
}

}  // namespace

int RunIdentityTests() {
  using namespace ame::installer;
  const std::wstring identity = std::wstring(kBrokerIdentityPrefix) +
      L"BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
  const std::wstring binary = L"\"" +
      (ProductDirectory() / L"Journal Broker" / kBrokerName).native() + L"\"";
  const BrokerServiceRegistration valid{SERVICE_WIN32_OWN_PROCESS, SERVICE_DEMAND_START,
      SERVICE_SID_TYPE_RESTRICTED, L"LocalSystem", binary,
      {L"SeManageVolumePrivilege", L"SeImpersonatePrivilege"}, identity};
  ValidateBrokerServiceRegistration(valid, identity);
  const std::vector<std::function<void(BrokerServiceRegistration&)>> changes{
      [](auto& value) { value.binary_path = L"\"C:\\foreign.exe\""; },
      [](auto& value) { value.binary_path += L" --arbitrary-argument"; },
      [](auto& value) { value.account = L"LocalService"; },
      [](auto& value) { value.service_type = SERVICE_WIN32_SHARE_PROCESS; },
      [](auto& value) { value.start_type = SERVICE_AUTO_START; },
      [](auto& value) { value.sid_type = SERVICE_SID_TYPE_UNRESTRICTED; },
      [](auto& value) { value.privileges = {L"SeImpersonatePrivilege"}; },
      [](auto& value) { value.privileges = {L"SeManageVolumePrivilege"}; },
      [](auto& value) { value.privileges.push_back(L"SeDebugPrivilege"); },
      [](auto& value) { value.privileges.clear(); },
      [](auto& value) { value.description.back() = L'0'; }};
  for (const auto& change : changes) {
    auto invalid = valid;
    change(invalid);
    RequireRejected([&] { ValidateBrokerServiceRegistration(invalid, identity); });
  }
  if (!IsBrokerIdentity(identity) || IsBrokerIdentity(identity + L"0") ||
      IsBrokerIdentity(identity.substr(0, identity.size() - 1)) ||
      IsBrokerIdentity(std::wstring(kBrokerIdentityPrefix) + std::wstring(64, L'a'))) {
    throw std::runtime_error("Broker manifest syntax validation failed");
  }
  const auto fixture = std::filesystem::temp_directory_path() /
      (L"ame-installer-hash-" + std::to_wstring(GetCurrentProcessId()) + L".tmp");
  FileHandle file(CreateFileW(fixture.c_str(), GENERIC_READ | GENERIC_WRITE, 0, nullptr,
                             CREATE_NEW, FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_DELETE_ON_CLOSE, nullptr));
  if (file.get() == INVALID_HANDLE_VALUE) {
    file.release();
    throw std::runtime_error("Cannot create owned identity fixture");
  }
  RequireRejected([&] { BrokerIdentityForFile(file.get()); });
  DWORD written = 0;
  if (!WriteFile(file.get(), "abc", 3, &written, nullptr) || written != 3 ||
      BrokerIdentityForFile(file.get()) != identity) {
    throw std::runtime_error("Native broker SHA-256 does not match the fixed vector");
  }
  return 15;
}
