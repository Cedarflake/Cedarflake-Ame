#ifndef AME_INSTALLER_USER_DATA_CLEANUP_H_
#define AME_INSTALLER_USER_DATA_CLEANUP_H_

#include "policy.h"

namespace ame::installer {

struct UserDataRemovalRequest {
  bool installed;
  std::wstring remove;
  bool upgrading;
  std::wstring choice;
};

struct CurrentUserData {
  std::wstring sid;
  std::filesystem::path local_app_data;
};

bool ShouldRemoveUserData(const UserDataRemovalRequest& request);
CurrentUserData ResolveCurrentUserData(const std::wstring& expected_sid);
FileHandle AcquireUserDataCleanupLease(const std::wstring& sid);
std::filesystem::path AmeUserDataDirectory(const std::filesystem::path& local_app_data);
bool RemoveAmeUserData(const std::filesystem::path& local_app_data);

}  // namespace ame::installer

#endif
