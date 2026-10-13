#include "policy.h"

#include <msi.h>
#include <msiquery.h>

#include <stdexcept>
#include <vector>

namespace ame::installer {
namespace {

class InstallerHandle {
 public:
  ~InstallerHandle() {
    if (value) {
      MsiCloseHandle(value);
    }
  }
  InstallerHandle() = default;
  InstallerHandle(const InstallerHandle&) = delete;
  InstallerHandle& operator=(const InstallerHandle&) = delete;
  MSIHANDLE value = 0;
};

std::wstring RegisteredProperty(const wchar_t* product, const wchar_t* property) {
  std::vector<wchar_t> value(32768);
  DWORD capacity = static_cast<DWORD>(value.size());
  if (MsiGetProductInfoW(product, property, value.data(), &capacity) != ERROR_SUCCESS) {
    throw std::runtime_error("Cannot inspect preceding product registration");
  }
  return std::wstring(value.data(), capacity);
}

std::wstring SingleString(MSIHANDLE database, const wchar_t* query) {
  InstallerHandle view;
  InstallerHandle record;
  if (MsiDatabaseOpenViewW(database, query, &view.value) != ERROR_SUCCESS ||
      MsiViewExecute(view.value, 0) != ERROR_SUCCESS ||
      MsiViewFetch(view.value, &record.value) != ERROR_SUCCESS) {
    throw std::runtime_error("Cached product does not contain its broker registration");
  }
  std::vector<wchar_t> value(1024);
  DWORD capacity = static_cast<DWORD>(value.size());
  if (MsiRecordGetFieldCount(record.value) != 1 ||
      MsiRecordGetStringW(record.value, 1, value.data(), &capacity) != ERROR_SUCCESS || !capacity) {
    throw std::runtime_error("Cached broker registration is malformed");
  }
  InstallerHandle excess;
  if (MsiViewFetch(view.value, &excess.value) != ERROR_NO_MORE_ITEMS) {
    throw std::runtime_error("Cached broker registration is ambiguous");
  }
  return std::wstring(value.data(), capacity);
}

RegisteredProduct CachedProduct(const wchar_t* product) {
  const std::filesystem::path cached = RegisteredProperty(product, INSTALLPROPERTY_LOCALPACKAGE);
  std::vector<wchar_t> system(32768);
  const UINT count = GetWindowsDirectoryW(system.data(), static_cast<UINT>(system.size()));
  if (!count || count >= system.size() ||
      !SamePath(cached.parent_path(), std::filesystem::path(system.data()) / L"Installer")) {
    throw std::runtime_error("Cached product is outside protected Windows Installer storage");
  }
  const auto lease = OpenProtectedFile(cached);
  if (!lease) {
    throw std::runtime_error("Cached product is missing");
  }
  InstallerHandle database;
  if (MsiOpenDatabaseW(cached.c_str(), MSIDBOPEN_READONLY, &database.value) != ERROR_SUCCESS) {
    throw std::runtime_error("Cannot read cached product identity");
  }
  if (SingleString(database.value, L"SELECT `Value` FROM `Property` WHERE `Property` = 'ProductCode'") != product ||
      SingleString(database.value, L"SELECT `Value` FROM `Property` WHERE `Property` = 'UpgradeCode'") != kUpgradeCode ||
      SingleString(database.value, L"SELECT `KeyPath` FROM `Component` WHERE `Component` = 'AmeBroker'") !=
          L"AmeBrokerExecutable") {
    throw std::runtime_error("Cached product identity differs from its registration");
  }
  const auto component = SingleString(database.value,
      L"SELECT `ComponentId` FROM `Component` WHERE `Component` = 'AmeBroker'");
  std::vector<wchar_t> component_path(32768);
  DWORD capacity = static_cast<DWORD>(component_path.size());
  const auto state = MsiGetComponentPathW(product, component.c_str(), component_path.data(), &capacity);
  if ((state != INSTALLSTATE_LOCAL && state != INSTALLSTATE_ABSENT) ||
      !SamePath(component_path.data(), ProductDirectory() / L"Journal Broker" / kBrokerName)) {
    throw std::runtime_error("Installed broker component has an unexpected destination");
  }
  const auto identity = SingleString(database.value,
      L"SELECT `Description` FROM `ServiceInstall` WHERE `Name` = 'CedarflakeAmeJournalBroker' AND `Component_` = 'AmeBroker'");
  if (!IsBrokerIdentity(identity)) {
    throw std::runtime_error("Cached broker identity is malformed");
  }
  const auto application_component = SingleString(database.value,
      L"SELECT `Component_` FROM `File` WHERE `File` = 'AmeApplicationExecutable'");
  if (application_component.find_first_not_of(L"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.") !=
      std::wstring::npos) {
    throw std::runtime_error("Cached application component is malformed");
  }
  const auto query = L"SELECT `ComponentId` FROM `Component` WHERE `Component` = '" + application_component + L"'";
  const auto application_id = SingleString(database.value, query.c_str());
  capacity = static_cast<DWORD>(component_path.size());
  const auto application_state = MsiGetComponentPathW(product, application_id.c_str(), component_path.data(), &capacity);
  const std::filesystem::path directory = RegisteredProperty(product, INSTALLPROPERTY_INSTALLLOCATION);
  if ((application_state != INSTALLSTATE_LOCAL && application_state != INSTALLSTATE_ABSENT) ||
      !SamePath(component_path.data(), directory / L"cedarflake_ame.exe")) {
    throw std::runtime_error("Registered application directory differs from its component");
  }
  ValidateApplicationPath(directory, ProductDirectory());
  return RegisteredProduct{identity, directory};
}

}  // namespace

std::optional<RegisteredProduct> ReadRegisteredProduct() {
  std::optional<RegisteredProduct> registered;
  for (DWORD index = 0; index < 32; ++index) {
    wchar_t product[39]{};
    const UINT status = MsiEnumRelatedProductsW(kUpgradeCode, 0, index, product);
    if (status == ERROR_NO_MORE_ITEMS) {
      return registered;
    }
    if (status != ERROR_SUCCESS) {
      throw std::runtime_error("Cannot inspect preceding product registration");
    }
    if (MsiQueryProductStateW(product) != INSTALLSTATE_DEFAULT) {
      continue;
    }
    if (registered || RegisteredProperty(product, INSTALLPROPERTY_ASSIGNMENTTYPE) != L"1") {
      throw std::runtime_error("Preceding product has ambiguous registration or an unexpected destination");
    }
    registered = CachedProduct(product);
  }
  throw std::runtime_error("Related product registrations exceed their bound");
}

}  // namespace ame::installer
