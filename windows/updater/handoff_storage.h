#ifndef AME_UPDATE_HANDOFF_STORAGE_H_
#define AME_UPDATE_HANDOFF_STORAGE_H_

#include "package.h"

namespace ame::updater {

// Pins the managed directory tree while publishing signals and the update result.
class HandoffStorage {
 public:
  HandoffStorage(const std::filesystem::path& directory, const std::filesystem::path& cache);
  void Acknowledge() const;
  bool IsCommitted() const;
  void WriteResult(const std::wstring& version, DWORD code) const;

 private:
  std::filesystem::path directory_;
  std::filesystem::path cache_;
  std::vector<installer::FileHandle> directories_;
  installer::FileHandle lease_;
};

}  // namespace ame::updater

#endif
