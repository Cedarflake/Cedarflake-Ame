#include "policy.h"

#include <msi.h>
#include <msiquery.h>

#include <stdexcept>
#include <vector>

namespace {

std::wstring Property(MSIHANDLE session, const wchar_t* name) {
  DWORD required = 0;
  const UINT sizing = MsiGetPropertyW(session, name, L"", &required);
  if ((sizing != ERROR_MORE_DATA && sizing != ERROR_SUCCESS) || required > 32768) {
    throw std::runtime_error("Installer property is unavailable or exceeds its bound");
  }
  std::vector<wchar_t> value(static_cast<size_t>(required) + 1);
  DWORD capacity = static_cast<DWORD>(value.size());
  if (MsiGetPropertyW(session, name, value.data(), &capacity) != ERROR_SUCCESS) {
    throw std::runtime_error("Cannot read installer property");
  }
  return std::wstring(value.data(), capacity);
}

std::filesystem::path TargetPath(MSIHANDLE session, const wchar_t* name) {
  std::vector<wchar_t> value(32768);
  DWORD capacity = static_cast<DWORD>(value.size());
  if (MsiGetTargetPathW(session, name, value.data(), &capacity) != ERROR_SUCCESS) {
    throw std::runtime_error("Cannot resolve fixed installation directory");
  }
  return std::filesystem::path(value.data());
}

UINT ReportFailure(MSIHANDLE session, const std::exception& error) noexcept {
  const MSIHANDLE record = MsiCreateRecord(1);
  if (record) {
    MsiRecordSetStringW(record, 0, L"Cedarflake Ame installation cannot continue: [1]");
    MsiRecordSetStringA(record, 1, error.what());
    MsiProcessMessage(session, INSTALLMESSAGE_ERROR, record);
    MsiCloseHandle(record);
  }
  return ERROR_INSTALL_FAILURE;
}

}  // namespace

extern "C" __declspec(dllexport) UINT __stdcall AmeValidateInstallation(MSIHANDLE session) {
  try {
    using namespace ame::installer;
    const auto product = ProductDirectory();
    if (Property(session, L"ALLUSERS") != L"1" ||
        !SamePath(TargetPath(session, L"AmeProductFolder"), product) ||
        !SamePath(TargetPath(session, L"AmeBrokerFolder"), product / L"Journal Broker")) {
      throw std::runtime_error("The journal broker must use its protected per-machine location");
    }
    ValidateProtectedTree(product);
    const auto registered = ReadRegisteredProduct();
    ValidateApplicationDestination(TargetPath(session, L"AMEAPPDIR"), registered);
    const auto service = ReadBrokerServiceRegistration();
    if (!registered && (service ||
        (std::filesystem::exists(product) && !std::filesystem::is_empty(product)))) {
      throw std::runtime_error("An unmanaged Ame installation must be removed before MSI installation");
    }
    if (registered && service) {
      ValidateBrokerServiceRegistration(*service, registered->broker_identity);
      const auto installed_identity = ReadInstalledBrokerIdentity();
      if (installed_identity && *installed_identity != registered->broker_identity) {
        throw std::runtime_error("Existing broker bytes differ from the registered product");
      }
    }
    return ERROR_SUCCESS;
  } catch (const std::exception& error) {
    return ReportFailure(session, error);
  } catch (...) {
    return ERROR_INSTALL_FAILURE;
  }
}

extern "C" __declspec(dllexport) UINT __stdcall AmeVerifyInstalledService(MSIHANDLE session) {
  try {
    const auto manifest = Property(session, L"CustomActionData");
    if (!ame::installer::IsBrokerIdentity(manifest)) {
      throw std::runtime_error("Installer broker identity is malformed");
    }
    ame::installer::ValidateProtectedTree(ame::installer::ProductDirectory());
    ame::installer::VerifyServiceConfiguration(manifest);
    return ERROR_SUCCESS;
  } catch (const std::exception& error) {
    return ReportFailure(session, error);
  } catch (...) {
    return ERROR_INSTALL_FAILURE;
  }
}

extern "C" __declspec(dllexport) UINT __stdcall AmeStartInstalledService(MSIHANDLE session) {
  try {
    return ame::installer::StartVerifiedBroker(session, Property(session, L"CustomActionData"));
  } catch (const std::exception& error) {
    return ReportFailure(session, error);
  } catch (...) {
    return ERROR_INSTALL_FAILURE;
  }
}

extern "C" __declspec(dllexport) UINT __stdcall AmeStopInstalledService(MSIHANDLE session) {
  try {
    ame::installer::StopVerifiedBroker(Property(session, L"CustomActionData"));
    return ERROR_SUCCESS;
  } catch (const std::exception& error) {
    return ReportFailure(session, error);
  } catch (...) {
    return ERROR_INSTALL_FAILURE;
  }
}
