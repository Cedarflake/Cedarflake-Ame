#include "policy.h"

#include <msi.h>
#include <msiquery.h>

#include <chrono>
#include <stdexcept>
#include <thread>
#include <type_traits>

namespace ame::installer {
namespace {

struct ServiceDeleter {
  void operator()(SC_HANDLE value) const noexcept { CloseServiceHandle(value); }
};
using ServiceHandle = std::unique_ptr<std::remove_pointer_t<SC_HANDLE>, ServiceDeleter>;

SERVICE_STATUS_PROCESS ReadStatus(SC_HANDLE service) {
  SERVICE_STATUS_PROCESS status{};
  DWORD written = 0;
  if (!QueryServiceStatusEx(service, SC_STATUS_PROCESS_INFO, reinterpret_cast<BYTE*>(&status),
                           sizeof(status), &written)) {
    throw std::runtime_error("Cannot query journal broker startup");
  }
  return status;
}

ServiceHandle OpenBroker(DWORD access) {
  ServiceHandle manager(OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT));
  if (!manager) {
    throw std::runtime_error("Cannot open service manager");
  }
  ServiceHandle service(OpenServiceW(manager.get(), kServiceName, access | SERVICE_QUERY_STATUS));
  if (!service) {
    throw std::runtime_error("Cannot open installed journal broker");
  }
  return service;
}

bool Cancelled(MSIHANDLE session) {
  const MSIHANDLE record = MsiCreateRecord(4);
  if (!record) {
    throw std::runtime_error("Cannot report journal broker startup progress");
  }
  MsiRecordSetInteger(record, 1, 2);
  MsiRecordSetInteger(record, 2, 0);
  MsiRecordSetInteger(record, 3, 0);
  MsiRecordSetInteger(record, 4, 0);
  const int result = MsiProcessMessage(session, INSTALLMESSAGE_PROGRESS, record);
  MsiCloseHandle(record);
  if (result == -1) {
    throw std::runtime_error("Cannot deliver journal broker startup progress");
  }
  return result == IDCANCEL;
}

}  // namespace

BrokerStartState ClassifyBrokerStart(const SERVICE_STATUS_PROCESS& status,
                                    bool cancelled, bool deadline_reached) {
  if (cancelled) {
    return BrokerStartState::Cancelled;
  }
  if (deadline_reached) {
    return BrokerStartState::TimedOut;
  }
  switch (status.dwCurrentState) {
    case SERVICE_RUNNING:
      return status.dwProcessId ? BrokerStartState::Ready : BrokerStartState::Failed;
    case SERVICE_START_PENDING:
      return BrokerStartState::Pending;
    default:
      return BrokerStartState::Failed;
  }
}

UINT StartVerifiedBroker(MSIHANDLE session, const std::wstring& identity) {
  VerifyServiceConfiguration(identity);
  auto service = OpenBroker(SERVICE_START);
  if (Cancelled(session)) {
    return ERROR_INSTALL_USEREXIT;
  }
  const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(10);
  if (!StartServiceW(service.get(), 0, nullptr)) {
    const DWORD error = GetLastError();
    if (error != ERROR_SERVICE_ALREADY_RUNNING) {
      throw std::runtime_error("Journal broker start request failed: " + std::to_string(error));
    }
  }
  for (;;) {
    const bool cancelled = Cancelled(session);
    const auto status = ReadStatus(service.get());
    switch (ClassifyBrokerStart(status, cancelled, std::chrono::steady_clock::now() >= deadline)) {
      case BrokerStartState::Ready:
        return ERROR_SUCCESS;
      case BrokerStartState::Failed:
        throw std::runtime_error("Journal broker stopped during startup (Windows " +
            std::to_string(status.dwWin32ExitCode) + ", service " +
            std::to_string(status.dwServiceSpecificExitCode) + ")");
      case BrokerStartState::Pending:
        break;
      case BrokerStartState::Cancelled:
        return ERROR_INSTALL_USEREXIT;
      case BrokerStartState::TimedOut:
        throw std::runtime_error("Journal broker did not become ready within 10 seconds");
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }
}

void StopVerifiedBroker(const std::wstring& identity) {
  VerifyServiceConfiguration(identity);
  auto service = OpenBroker(SERVICE_STOP);
  const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(10);
  auto status = ReadStatus(service.get());
  while (status.dwCurrentState == SERVICE_START_PENDING) {
    if (std::chrono::steady_clock::now() >= deadline) {
      throw std::runtime_error("Journal broker remained starting during rollback");
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    status = ReadStatus(service.get());
  }
  if (status.dwCurrentState == SERVICE_STOPPED) {
    return;
  }
  SERVICE_STATUS stopped{};
  if (!ControlService(service.get(), SERVICE_CONTROL_STOP, &stopped) &&
      GetLastError() != ERROR_SERVICE_NOT_ACTIVE) {
    throw std::runtime_error("Cannot stop journal broker during rollback");
  }
  while (ReadStatus(service.get()).dwCurrentState != SERVICE_STOPPED) {
    if (std::chrono::steady_clock::now() >= deadline) {
      throw std::runtime_error("Journal broker did not stop within the rollback deadline");
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }
}

}  // namespace ame::installer
