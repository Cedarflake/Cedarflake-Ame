#ifndef AME_INSTALLER_LOCALIZATION_H_
#define AME_INSTALLER_LOCALIZATION_H_

#include <windows.h>
#include <msi.h>

#include <string>
#include <vector>

namespace ame::installer {

enum class InstallerLanguage { English = 1033, Japanese = 1041, Chinese = 2052 };
InstallerLanguage ResolveInstallerLanguage(const std::vector<std::wstring>& preferences);
void LocalizeInstaller(MSIHANDLE session);

}  // namespace ame::installer

#endif
