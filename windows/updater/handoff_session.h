#ifndef AME_UPDATE_HANDOFF_SESSION_H_
#define AME_UPDATE_HANDOFF_SESSION_H_

#include "handoff_storage.h"
#include "../runner/single_instance_lock.h"

namespace ame::updater {

class HandoffSession {
 public:
  HandoffSession(DWORD parent_id, const std::filesystem::path& application,
                 const std::filesystem::path& ready, const std::filesystem::path& result,
                 const std::wstring& version);
  const std::filesystem::path& directory() const { return directory_; }
  void Acknowledge() const;
  bool WaitForParent() const;
  bool IsCommitted() const;
  int Reopen(DWORD code);

 private:
  std::filesystem::path application_;
  std::wstring version_;
  installer::FileHandle parent_;
  std::filesystem::path directory_;
  HandoffStorage storage_;
  SingleInstanceLock startup_guard_;
};

}  // namespace ame::updater
#endif
