#include "policy.h"

#include <algorithm>
#include <cstdint>
#include <stdexcept>

namespace ame::installer {

std::vector<std::wstring> ReadBrokerPrivileges(const std::vector<BYTE>& bytes,
                                              const wchar_t* text) {
  const auto begin = reinterpret_cast<uintptr_t>(bytes.data());
  const auto address = reinterpret_cast<uintptr_t>(text);
  if (!text || address < begin || address - begin >= bytes.size() ||
      address % alignof(wchar_t) != 0) {
    throw std::runtime_error("Journal broker privileges are outside their buffer");
  }
  const auto capacity = (bytes.size() - (address - begin)) / sizeof(wchar_t);
  std::vector<std::wstring> privileges;
  size_t offset = 0;
  while (offset < capacity) {
    const auto end = std::find(text + offset, text + capacity, L'\0');
    if (end == text + capacity || privileges.size() >= 16) {
      break;
    }
    if (end == text + offset) {
      return privileges;
    }
    privileges.emplace_back(text + offset, end);
    offset = static_cast<size_t>(end - text) + 1;
  }
  throw std::runtime_error("Journal broker privileges lack a bounded MULTI_SZ terminator");
}

bool HasRequiredBrokerPrivileges(const std::vector<std::wstring>& privileges) {
  if (privileges.size() != 2) {
    return false;
  }
  return std::count(privileges.begin(), privileges.end(), L"SeManageVolumePrivilege") == 1 &&
      std::count(privileges.begin(), privileges.end(), L"SeImpersonatePrivilege") == 1;
}

}  // namespace ame::installer
