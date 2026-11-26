#include "application_update_startup.h"

#include <windows.h>

#include <filesystem>
#include <vector>

bool RunApplicationUpdateStartup() {
  std::vector<wchar_t> path(32768);
  const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
  if (!size || size >= path.size()) return false;
  const auto library = std::filesystem::path(path.data()).parent_path() / L"rust_lib_cedarflake_ame.dll";
  const HMODULE module = LoadLibraryExW(library.c_str(), nullptr,
      LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
  if (!module) return false;
  using Startup = int (*)();
  const auto startup = reinterpret_cast<Startup>(GetProcAddress(module, "ame_application_update_startup"));
  // The bridge uses the same module for the rest of the process lifetime.
  return startup && startup() == 0;
}
