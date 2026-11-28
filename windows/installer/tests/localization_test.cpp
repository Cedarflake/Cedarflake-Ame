#include "../localization.h"

#include <stdexcept>

int RunLocalizationTests() {
  using ame::installer::InstallerLanguage;
  struct Scenario {
    std::vector<std::wstring> preferences;
    InstallerLanguage expected;
  };
  const Scenario scenarios[] = {
      {{L"ja-JP", L"en-US"}, InstallerLanguage::Japanese},
      {{L"en-GB", L"zh-CN"}, InstallerLanguage::English},
      {{L"zh-Hans-SG", L"en-US"}, InstallerLanguage::Chinese},
      {{L"zh-Hant-CN", L"ja-JP"}, InstallerLanguage::Japanese},
      {{L"zh-TW", L"en-US"}, InstallerLanguage::English},
      {{L"fr-FR", L"ZH-cn"}, InstallerLanguage::Chinese},
      {{L"fr-FR"}, InstallerLanguage::English},
      {{}, InstallerLanguage::English},
  };
  for (const auto& scenario : scenarios) {
    if (ame::installer::ResolveInstallerLanguage(scenario.preferences) != scenario.expected) {
      throw std::runtime_error("Installer display-language preference order changed");
    }
  }
  return static_cast<int>(sizeof(scenarios) / sizeof(scenarios[0]));
}
