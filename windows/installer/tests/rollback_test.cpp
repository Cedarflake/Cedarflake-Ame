#include "../service_rollback.h"

#include <functional>
#include <stdexcept>

namespace {

void RequireRejected(const std::function<void()>& action) {
  try {
    action();
  } catch (const std::runtime_error&) {
    return;
  }
  throw std::runtime_error("Unexpected preceding broker state was admitted for rollback");
}

}  // namespace

int RunRollbackTests() {
  using namespace ame::installer;
  const std::wstring identity = std::wstring(kBrokerIdentityPrefix) + std::wstring(64, L'A');
  for (const auto& encoded : {L"0:" + identity, L"1:" + identity}) {
    const auto state = DecodePreviousBrokerState(encoded);
    if (state.identity != identity || state.was_running != (encoded.front() == L'1')) {
      throw std::runtime_error("Previous broker identity or running state was lost");
    }
  }
  for (const auto& encoded : {std::wstring(), L"2:" + identity, L"1;" + identity,
                              L"1:" + identity + L"|extra", std::wstring(L"1:foreign")}) {
    RequireRejected([&] { DecodePreviousBrokerState(encoded); });
  }
  const std::wstring binary = L"\"" +
      (ProductDirectory() / L"Journal Broker" / kBrokerName).native() + L"\"";
  const BrokerServiceRegistration restored{SERVICE_WIN32_OWN_PROCESS, SERVICE_DEMAND_START,
      SERVICE_SID_TYPE_NONE, L"LocalSystem", binary, {}, identity};
  ValidateRestorableBrokerRegistration(restored, identity);
  RequireRejected([&] { ValidateBrokerServiceRegistration(restored, identity); });
  auto complete = restored;
  complete.sid_type = SERVICE_SID_TYPE_RESTRICTED;
  complete.privileges = {L"SeManageVolumePrivilege", L"SeImpersonatePrivilege"};
  ValidateRestorableBrokerRegistration(complete, identity);
  const std::vector<std::function<void(BrokerServiceRegistration&)>> changes{
      [](auto& value) { value.binary_path = L"\"C:\\foreign.exe\""; },
      [](auto& value) { value.account = L"LocalService"; },
      [](auto& value) { value.service_type = SERVICE_WIN32_SHARE_PROCESS; },
      [](auto& value) { value.start_type = SERVICE_AUTO_START; },
      [](auto& value) { value.sid_type = SERVICE_SID_TYPE_UNRESTRICTED; },
      [](auto& value) { value.privileges = {L"SeDebugPrivilege"}; },
      [](auto& value) { value.privileges = {L"SeImpersonatePrivilege"}; },
      [](auto& value) { value.description.back() = L'B'; }};
  for (const auto& change : changes) {
    auto invalid = restored;
    change(invalid);
    RequireRejected([&] { ValidateRestorableBrokerRegistration(invalid, identity); });
  }
  return 18;
}
