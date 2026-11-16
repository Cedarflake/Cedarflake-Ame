#ifndef AME_INSTALLER_SERVICE_ROLLBACK_H_
#define AME_INSTALLER_SERVICE_ROLLBACK_H_

#include "policy.h"

namespace ame::installer {

struct PreviousBrokerState {
  std::wstring identity;
  bool was_running;
};

std::wstring CapturePreviousBrokerState(const std::wstring& identity);
PreviousBrokerState DecodePreviousBrokerState(const std::wstring& encoded);
void ValidateRestorableBrokerRegistration(const BrokerServiceRegistration& registration,
                                          const std::wstring& identity);
void RestorePreviousBroker(MSIHANDLE session, const std::wstring& encoded);

}  // namespace ame::installer

#endif
