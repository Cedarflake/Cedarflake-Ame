#include "service_rollback.h"

#include <msiquery.h>

#include <stdexcept>
#include <type_traits>

namespace ame::installer {
namespace {

struct ServiceDeleter {
  void operator()(SC_HANDLE value) const noexcept { CloseServiceHandle(value); }
};
using ServiceHandle = std::unique_ptr<std::remove_pointer_t<SC_HANDLE>, ServiceDeleter>;

ServiceHandle OpenBroker(DWORD access) {
  ServiceHandle manager(OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT));
  if (!manager) {
    throw std::runtime_error("Cannot open service manager for upgrade recovery");
  }
  ServiceHandle service(OpenServiceW(manager.get(), kServiceName, access | SERVICE_QUERY_STATUS));
  if (!service) {
    throw std::runtime_error("Cannot open the preceding journal broker");
  }
  return service;
}

DWORD ReadState(SC_HANDLE service) {
  SERVICE_STATUS_PROCESS status{};
  DWORD written = 0;
  if (!QueryServiceStatusEx(service, SC_STATUS_PROCESS_INFO, reinterpret_cast<BYTE*>(&status),
                           sizeof(status), &written)) {
    throw std::runtime_error("Cannot read the preceding journal broker state");
  }
  return status.dwCurrentState;
}

}  // namespace

std::wstring CapturePreviousBrokerState(const std::wstring& identity) {
  VerifyServiceConfiguration(identity);
  const auto service = OpenBroker(SERVICE_QUERY_STATUS);
  const DWORD state = ReadState(service.get());
  const bool running = state != SERVICE_STOPPED && state != SERVICE_STOP_PENDING;
  return (running ? L"1:" : L"0:") + identity;
}

PreviousBrokerState DecodePreviousBrokerState(const std::wstring& encoded) {
  if (encoded.size() < 2 || (encoded[0] != L'0' && encoded[0] != L'1') ||
      encoded[1] != L':' || !IsBrokerIdentity(encoded.substr(2))) {
    throw std::runtime_error("The preceding journal broker snapshot is malformed");
  }
  return {encoded.substr(2), encoded[0] == L'1'};
}

void ValidateRestorableBrokerRegistration(const BrokerServiceRegistration& registration,
                                          const std::wstring& identity) {
  if ((registration.sid_type != SERVICE_SID_TYPE_NONE &&
       registration.sid_type != SERVICE_SID_TYPE_RESTRICTED) ||
      (!registration.privileges.empty() && !HasRequiredBrokerPrivileges(registration.privileges))) {
    throw std::runtime_error("The restored journal broker has an unexpected security configuration");
  }
  // MSI restores ServiceInstall fields but omits these two MsiServiceConfig fields.
  auto completed = registration;
  completed.sid_type = SERVICE_SID_TYPE_RESTRICTED;
  completed.privileges = {L"SeManageVolumePrivilege", L"SeImpersonatePrivilege"};
  ValidateBrokerServiceRegistration(completed, identity);
}

void RestorePreviousBroker(MSIHANDLE session, const std::wstring& encoded) {
  if (!MsiGetMode(session, MSIRUNMODE_ROLLBACK)) {
    throw std::runtime_error("Previous broker recovery requires installer rollback");
  }
  const auto previous = DecodePreviousBrokerState(encoded);
  const auto registration = ReadBrokerServiceRegistration();
  if (!registration) {
    throw std::runtime_error("The preceding journal broker registration was not restored");
  }
  ValidateRestorableBrokerRegistration(*registration, previous.identity);
  const auto file = OpenProtectedFile(ProductDirectory() / L"Journal Broker" / kBrokerName);
  if (!file || BrokerIdentityForFile(file.get()) != previous.identity) {
    throw std::runtime_error("The preceding journal broker bytes were not restored");
  }
  VerifyServicePermissions();
  if (registration->sid_type != SERVICE_SID_TYPE_RESTRICTED || registration->privileges.empty()) {
    const auto service = OpenBroker(SERVICE_CHANGE_CONFIG);
    if (ReadState(service.get()) != SERVICE_STOPPED) {
      throw std::runtime_error("The restored journal broker must be stopped before security recovery");
    }
    SERVICE_SID_INFO sid{SERVICE_SID_TYPE_RESTRICTED};
    wchar_t privileges[] = L"SeManageVolumePrivilege\0SeImpersonatePrivilege\0";
    SERVICE_REQUIRED_PRIVILEGES_INFOW required{privileges};
    if (!ChangeServiceConfig2W(service.get(), SERVICE_CONFIG_SERVICE_SID_INFO, &sid) ||
        !ChangeServiceConfig2W(service.get(), SERVICE_CONFIG_REQUIRED_PRIVILEGES_INFO, &required)) {
      throw std::runtime_error("Cannot restore the preceding journal broker security configuration");
    }
  }
  VerifyServiceConfiguration(previous.identity);
  if (previous.was_running && StartVerifiedBroker(session, previous.identity) != ERROR_SUCCESS) {
    throw std::runtime_error("Cannot restart the preceding journal broker");
  }
}

}  // namespace ame::installer
