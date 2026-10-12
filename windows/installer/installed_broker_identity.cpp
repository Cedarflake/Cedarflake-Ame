#include "policy.h"

#include <bcrypt.h>

#include <algorithm>
#include <array>
#include <stdexcept>

namespace ame::installer {
namespace {

struct HashDeleter {
  void operator()(void* value) const noexcept { BCryptDestroyHash(value); }
};
using HashHandle = std::unique_ptr<void, HashDeleter>;

}  // namespace

bool IsBrokerIdentity(const std::wstring& identity) {
  const std::wstring prefix = kBrokerIdentityPrefix;
  return identity.size() == prefix.size() + 64 && identity.rfind(prefix, 0) == 0 &&
         std::all_of(identity.begin() + prefix.size(), identity.end(), [](wchar_t value) {
           return (value >= L'0' && value <= L'9') || (value >= L'A' && value <= L'F');
         });
}

std::wstring BrokerIdentityForFile(HANDLE file) {
  LARGE_INTEGER length{};
  if (!GetFileSizeEx(file, &length) || length.QuadPart <= 0 ||
      length.QuadPart > 128LL * 1024 * 1024) {
    throw std::runtime_error("Installed broker size is outside its bound");
  }
  LARGE_INTEGER beginning{};
  if (!SetFilePointerEx(file, beginning, nullptr, FILE_BEGIN)) {
    throw std::runtime_error("Cannot seek installed broker");
  }
  BCRYPT_HASH_HANDLE raw = nullptr;
  if (BCryptCreateHash(BCRYPT_SHA256_ALG_HANDLE, &raw, nullptr, 0, nullptr, 0, 0) < 0) {
    throw std::runtime_error("Cannot create broker identity hash");
  }
  const HashHandle hash(raw);
  std::array<BYTE, 64 * 1024> buffer{};
  LONGLONG total = 0;
  while (true) {
    DWORD read = 0;
    if (!ReadFile(file, buffer.data(), static_cast<DWORD>(buffer.size()), &read, nullptr)) {
      throw std::runtime_error("Cannot read installed broker identity");
    }
    if (!read) {
      break;
    }
    total += read;
    if (total > length.QuadPart || BCryptHashData(hash.get(), buffer.data(), read, 0) < 0) {
      throw std::runtime_error("Installed broker changed or could not be hashed");
    }
  }
  if (total != length.QuadPart) {
    throw std::runtime_error("Installed broker length changed during inspection");
  }
  std::array<BYTE, 32> digest{};
  if (BCryptFinishHash(hash.get(), digest.data(), static_cast<ULONG>(digest.size()), 0) < 0) {
    throw std::runtime_error("Cannot finish broker identity hash");
  }
  constexpr wchar_t alphabet[] = L"0123456789ABCDEF";
  std::wstring identity = kBrokerIdentityPrefix;
  for (const BYTE value : digest) {
    identity += alphabet[value >> 4];
    identity += alphabet[value & 15];
  }
  return identity;
}

std::optional<std::wstring> ReadInstalledBrokerIdentity() {
  const auto file = OpenProtectedFile(ProductDirectory() / L"Journal Broker" / kBrokerName);
  if (!file) {
    return std::nullopt;
  }
  return BrokerIdentityForFile(file.get());
}

}  // namespace ame::installer
