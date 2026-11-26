#ifndef AME_UPDATE_PACKAGE_H_
#define AME_UPDATE_PACKAGE_H_

#include "../installer/policy.h"

namespace ame::updater {

std::filesystem::path UpdateCache();
void RejectReparse(const std::filesystem::path& path);
std::vector<installer::FileHandle> PinDirectoryTree(const std::filesystem::path& directory);
std::wstring QuoteArgument(const std::wstring& value);
struct VerifiedPackage {
  std::vector<installer::FileHandle> directories;
  installer::FileHandle file;
};
VerifiedPackage VerifyEngine(const std::filesystem::path& path,
                             const std::wstring& sha256, unsigned long long length);
VerifiedPackage VerifyPackage(const std::filesystem::path& path,
                                  const std::wstring& sha256, unsigned long long length,
                                  const std::filesystem::path& portable_root = {});

}  // namespace ame::updater

#endif
