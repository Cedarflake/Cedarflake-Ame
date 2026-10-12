#include "../policy.h"

#include <stdexcept>

int RunLifecycleTests() {
  using namespace ame::installer;
  const std::filesystem::path product = L"C:\\Program Files\\Cedarflake Ame";
  int count = 0;
  for (const auto& path : {product / L"Application", std::filesystem::path(L"D:\\Apps\\Ame")}) {
    ValidateApplicationPath(path, product);
    ++count;
  }
  for (const auto& path : {product, product.parent_path(), product / L"Journal Broker",
                          product / L"Journal Broker\\Nested", std::filesystem::path(L"D:\\"),
                          std::filesystem::path(L"relative\\Ame"),
                          std::filesystem::path(L"\\\\server\\share\\Ame"),
                          std::filesystem::path(L"D:\\Apps\\..\\Ame")}) {
    bool rejected = false;
    try {
      ValidateApplicationPath(path, product);
    } catch (const std::runtime_error&) {
      rejected = true;
    }
    if (!rejected) {
      throw std::runtime_error("Unsafe application destination was accepted");
    }
    ++count;
  }
  struct StartupCase {
    DWORD state;
    DWORD process;
    bool cancelled;
    bool expired;
    BrokerStartState expected;
  };
  const StartupCase cases[] = {
      {SERVICE_START_PENDING, 17, false, false, BrokerStartState::Pending},
      {SERVICE_RUNNING, 17, false, false, BrokerStartState::Ready},
      {SERVICE_RUNNING, 0, false, false, BrokerStartState::Failed},
      {SERVICE_STOPPED, 0, false, false, BrokerStartState::Failed},
      {SERVICE_STOP_PENDING, 17, false, false, BrokerStartState::Failed},
      {SERVICE_START_PENDING, 17, false, true, BrokerStartState::TimedOut},
      {SERVICE_RUNNING, 17, false, true, BrokerStartState::TimedOut},
      {SERVICE_RUNNING, 17, true, false, BrokerStartState::Cancelled},
      {SERVICE_START_PENDING, 17, true, true, BrokerStartState::Cancelled},
  };
  for (const auto& fixture : cases) {
    SERVICE_STATUS_PROCESS status{};
    status.dwCurrentState = fixture.state;
    status.dwProcessId = fixture.process;
    if (ClassifyBrokerStart(status, fixture.cancelled, fixture.expired) != fixture.expected) {
      throw std::runtime_error("Service startup did not preserve failure, cancellation or deadline precedence");
    }
    ++count;
  }
  return count;
}
