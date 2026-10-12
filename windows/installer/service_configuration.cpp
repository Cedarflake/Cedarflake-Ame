#include "policy.h"

#include <sddl.h>

#include <cstring>
#include <algorithm>
#include <cstdint>
#include <stdexcept>
#include <vector>

namespace ame::installer {
namespace {

struct ServiceDeleter {
  void operator()(SC_HANDLE value) const noexcept { CloseServiceHandle(value); }
};
using ServiceHandle = std::unique_ptr<SC_HANDLE__, ServiceDeleter>;

ServiceHandle OpenBroker(DWORD access) {
  ServiceHandle manager(OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT));
  if (!manager) {
    throw std::runtime_error("Cannot inspect Windows service manager");
  }
  ServiceHandle service(OpenServiceW(manager.get(), kServiceName, access));
  if (!service && GetLastError() != ERROR_SERVICE_DOES_NOT_EXIST) {
    throw std::runtime_error("Cannot inspect journal broker registration");
  }
  return service;
}

std::vector<BYTE> Configuration(SC_HANDLE service, DWORD level) {
  DWORD required = 0;
  QueryServiceConfig2W(service, level, nullptr, 0, &required);
  if (!required || required > 64 * 1024) {
    throw std::runtime_error("Journal broker configuration size is invalid");
  }
  std::vector<BYTE> bytes(required);
  if (!QueryServiceConfig2W(service, level, bytes.data(), required, &required)) {
    throw std::runtime_error("Cannot read journal broker configuration");
  }
  return bytes;
}

std::wstring ConfigurationString(const std::vector<BYTE>& bytes, const wchar_t* text) {
  const auto begin = reinterpret_cast<uintptr_t>(bytes.data());
  const auto end = begin + bytes.size();
  const auto address = reinterpret_cast<uintptr_t>(text);
  if (!text || address < begin || address >= end || address % alignof(wchar_t) != 0) {
    throw std::runtime_error("Journal broker configuration string is outside its buffer");
  }
  const size_t capacity = (end - address) / sizeof(wchar_t);
  const auto terminator = std::find(text, text + capacity, L'\0');
  if (terminator == text + capacity) {
    throw std::runtime_error("Journal broker configuration string is not bounded");
  }
  return std::wstring(text, terminator);
}

bool EqualSidField(PSECURITY_DESCRIPTOR left, PSECURITY_DESCRIPTOR right, bool owner) {
  PSID first = nullptr;
  PSID second = nullptr;
  BOOL defaulted = FALSE;
  const BOOL first_ok = owner ? GetSecurityDescriptorOwner(left, &first, &defaulted)
                              : GetSecurityDescriptorGroup(left, &first, &defaulted);
  const BOOL second_ok = owner ? GetSecurityDescriptorOwner(right, &second, &defaulted)
                               : GetSecurityDescriptorGroup(right, &second, &defaulted);
  return first_ok && second_ok && first && second && EqualSid(first, second);
}

void VerifyServiceAcl(SC_HANDLE service) {
  DWORD required = 0;
  constexpr DWORD parts = OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION |
                          DACL_SECURITY_INFORMATION;
  QueryServiceObjectSecurity(service, parts, nullptr, 0, &required);
  if (!required || required > 64 * 1024) {
    throw std::runtime_error("Journal broker security descriptor size is invalid");
  }
  std::vector<BYTE> bytes(required);
  if (!QueryServiceObjectSecurity(service, parts, bytes.data(), required, &required)) {
    throw std::runtime_error("Cannot read journal broker permissions");
  }
  PSECURITY_DESCRIPTOR expected = nullptr;
  if (!ConvertStringSecurityDescriptorToSecurityDescriptorW(kServiceSddl, SDDL_REVISION_1,
                                                           &expected, nullptr)) {
    throw std::runtime_error("Cannot construct journal broker permissions");
  }
  const LocalAllocation expected_owner(expected);
  PACL actual_acl = nullptr;
  PACL expected_acl = nullptr;
  BOOL actual_present = FALSE;
  BOOL expected_present = FALSE;
  BOOL defaulted = FALSE;
  if (!EqualSidField(bytes.data(), expected, true) ||
      !EqualSidField(bytes.data(), expected, false) ||
      !GetSecurityDescriptorDacl(bytes.data(), &actual_present, &actual_acl, &defaulted) ||
      !GetSecurityDescriptorDacl(expected, &expected_present, &expected_acl, &defaulted) ||
      !actual_present || !expected_present || !actual_acl || !expected_acl ||
      actual_acl->AclSize != expected_acl->AclSize ||
      std::memcmp(actual_acl, expected_acl, actual_acl->AclSize) != 0) {
    throw std::runtime_error("Journal broker permissions differ from the admitted descriptor");
  }
}

}  // namespace

std::optional<BrokerServiceRegistration> ReadBrokerServiceRegistration() {
  const auto service = OpenBroker(SERVICE_QUERY_CONFIG);
  if (!service) {
    return std::nullopt;
  }
  DWORD required = 0;
  QueryServiceConfigW(service.get(), nullptr, 0, &required);
  if (required < sizeof(QUERY_SERVICE_CONFIGW) || required > 64 * 1024) {
    throw std::runtime_error("Journal broker service configuration size is invalid");
  }
  std::vector<BYTE> bytes(required);
  auto config = reinterpret_cast<QUERY_SERVICE_CONFIGW*>(bytes.data());
  if (!QueryServiceConfigW(service.get(), config, required, &required)) {
    throw std::runtime_error("Cannot read journal broker service configuration");
  }
  const auto sid_bytes = Configuration(service.get(), SERVICE_CONFIG_SERVICE_SID_INFO);
  if (sid_bytes.size() < sizeof(SERVICE_SID_INFO)) {
    throw std::runtime_error("Journal broker service SID configuration is truncated");
  }
  const auto privilege_bytes = Configuration(service.get(), SERVICE_CONFIG_REQUIRED_PRIVILEGES_INFO);
  if (privilege_bytes.size() < sizeof(SERVICE_REQUIRED_PRIVILEGES_INFOW)) {
    throw std::runtime_error("Journal broker privilege configuration is truncated");
  }
  const auto privileges = reinterpret_cast<const SERVICE_REQUIRED_PRIVILEGES_INFOW*>(
      privilege_bytes.data())->pmszRequiredPrivileges;
  const auto description_bytes = Configuration(service.get(), SERVICE_CONFIG_DESCRIPTION);
  if (description_bytes.size() < sizeof(SERVICE_DESCRIPTIONW)) {
    throw std::runtime_error("Journal broker description is truncated");
  }
  const auto description = reinterpret_cast<const SERVICE_DESCRIPTIONW*>(
      description_bytes.data())->lpDescription;
  return BrokerServiceRegistration{
      config->dwServiceType, config->dwStartType,
      reinterpret_cast<const SERVICE_SID_INFO*>(sid_bytes.data())->dwServiceSidType,
      ConfigurationString(bytes, config->lpServiceStartName),
      ConfigurationString(bytes, config->lpBinaryPathName),
      ReadBrokerPrivileges(privilege_bytes, privileges),
      ConfigurationString(description_bytes, description)};
}

void ValidateBrokerServiceRegistration(const BrokerServiceRegistration& registration,
                                       const std::wstring& identity_manifest) {
  const auto binary = ProductDirectory() / L"Journal Broker" / kBrokerName;
  const std::wstring quoted_binary = L"\"" + binary.native() + L"\"";
  if (registration.service_type != SERVICE_WIN32_OWN_PROCESS ||
      registration.start_type != SERVICE_DEMAND_START || registration.account != L"LocalSystem" ||
      CompareStringOrdinal(registration.binary_path.c_str(), -1, quoted_binary.c_str(), -1, TRUE) !=
          CSTR_EQUAL) {
    throw std::runtime_error("Journal broker account, type, start or binary path is incorrect");
  }
  if (registration.sid_type != SERVICE_SID_TYPE_RESTRICTED ||
      !HasRequiredBrokerPrivileges(registration.privileges)) {
    throw std::runtime_error("Journal broker restricted SID or required privileges are incorrect");
  }
  if (!IsBrokerIdentity(identity_manifest) || registration.description != identity_manifest) {
    throw std::runtime_error("Journal broker identity manifest differs from this package");
  }
}

void VerifyServiceConfiguration(const std::wstring& identity_manifest) {
  const auto registration = ReadBrokerServiceRegistration();
  if (!registration) {
    throw std::runtime_error("Installed journal broker is missing");
  }
  ValidateBrokerServiceRegistration(*registration, identity_manifest);
  const auto file_identity = ReadInstalledBrokerIdentity();
  if (!file_identity || *file_identity != identity_manifest) {
    throw std::runtime_error("Installed broker bytes differ from this package; restart-delayed replacement is not accepted");
  }
  const auto service = OpenBroker(READ_CONTROL);
  if (!service) {
    throw std::runtime_error("Installed journal broker disappeared during verification");
  }
  VerifyServiceAcl(service.get());
}

}  // namespace ame::installer
