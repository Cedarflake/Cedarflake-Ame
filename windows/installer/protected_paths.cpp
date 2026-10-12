#include "policy.h"

#include <aclapi.h>
#include <sddl.h>
#include <shlobj.h>

#include <stdexcept>
#include <vector>

namespace ame::installer {
namespace {

bool TrustedSid(PSID sid) {
  wchar_t* text = nullptr;
  if (!sid || !ConvertSidToStringSidW(sid, &text)) {
    throw std::runtime_error("Cannot read installation security identity");
  }
  const LocalAllocation owner(text);
  return wcscmp(text, L"S-1-5-18") == 0 || wcscmp(text, L"S-1-5-32-544") == 0 ||
         wcscmp(text, L"S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464") == 0;
}

void ValidateOpenPath(HANDLE handle, const std::filesystem::path& path, bool volume_root) {
  BY_HANDLE_FILE_INFORMATION facts{};
  if (!GetFileInformationByHandle(handle, &facts) ||
      (facts.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT)) {
    throw std::runtime_error("Installation path is redirected or unverifiable");
  }
  std::vector<wchar_t> final_path(32768);
  const DWORD length = GetFinalPathNameByHandleW(handle, final_path.data(),
                                                static_cast<DWORD>(final_path.size()), 0);
  if (!length || length >= final_path.size()) {
    throw std::runtime_error("Cannot resolve physical installation path");
  }
  if (!SamePath(path, std::filesystem::path(final_path.data()))) {
    throw std::runtime_error("Installation path is not its fixed physical destination");
  }
  PSECURITY_DESCRIPTOR raw = nullptr;
  const DWORD result = GetSecurityInfo(handle, SE_FILE_OBJECT,
      OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION, nullptr, nullptr,
      nullptr, nullptr, &raw);
  const LocalAllocation descriptor(raw);
  if (result != ERROR_SUCCESS || !descriptor) {
    throw std::runtime_error("Cannot inspect installation permissions");
  }
  ValidateDescriptor(descriptor.get(), volume_root);
}

bool ValidatePath(const std::filesystem::path& path, bool volume_root) {
  FileHandle handle(CreateFileW(path.c_str(), READ_CONTROL | FILE_READ_ATTRIBUTES,
                               FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                               nullptr, OPEN_EXISTING,
                               FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                               nullptr));
  if (handle.get() == INVALID_HANDLE_VALUE) {
    handle.release();
    const DWORD error = GetLastError();
    if (error == ERROR_FILE_NOT_FOUND || error == ERROR_PATH_NOT_FOUND) {
      return false;
    }
    throw std::runtime_error("Cannot inspect fixed installation path");
  }
  ValidateOpenPath(handle.get(), path, volume_root);
  return true;
}

std::wstring ComparablePath(const std::filesystem::path& path) {
  std::wstring value = path.native();
  if (value.rfind(L"\\\\?\\", 0) == 0) {
    value.erase(0, 4);
  }
  while (value.size() > 3 && value.back() == L'\\') {
    value.pop_back();
  }
  return value;
}

}  // namespace

std::filesystem::path ProgramFiles() {
  wchar_t* raw = nullptr;
  const HRESULT result = SHGetKnownFolderPath(FOLDERID_ProgramFilesX64, 0, nullptr, &raw);
  if (FAILED(result) || !raw) {
    CoTaskMemFree(raw);
    throw std::runtime_error("Native Program Files is unavailable");
  }
  std::filesystem::path path;
  try {
    path = raw;
  } catch (...) {
    CoTaskMemFree(raw);
    throw;
  }
  CoTaskMemFree(raw);
  if (!path.is_absolute() || path.native().rfind(L"\\\\", 0) == 0) {
    throw std::runtime_error("Installation requires a local native Program Files path");
  }
  return path;
}

std::filesystem::path ProductDirectory() {
  return ProgramFiles() / L"Cedarflake Ame";
}

bool SamePath(const std::filesystem::path& first, const std::filesystem::path& second) {
  const auto left = ComparablePath(first);
  const auto right = ComparablePath(second);
  return CompareStringOrdinal(left.c_str(), static_cast<int>(left.size()),
                              right.c_str(), static_cast<int>(right.size()), TRUE) == CSTR_EQUAL;
}

void ValidateDescriptor(PSECURITY_DESCRIPTOR descriptor, bool volume_root) {
  PSID owner = nullptr;
  BOOL defaulted = FALSE;
  if (!IsValidSecurityDescriptor(descriptor) ||
      !GetSecurityDescriptorOwner(descriptor, &owner, &defaulted) || !TrustedSid(owner)) {
    throw std::runtime_error("Installation path has an untrusted owner");
  }
  PACL acl = nullptr;
  BOOL present = FALSE;
  if (!GetSecurityDescriptorDacl(descriptor, &present, &acl, &defaulted) ||
      !present || !acl || !IsValidAcl(acl)) {
    throw std::runtime_error("Installation path has no restrictive access list");
  }
  const DWORD dangerous = volume_root ? 0x000D0040 : 0x500D0156;
  for (DWORD index = 0; index < acl->AceCount; ++index) {
    void* raw = nullptr;
    if (!GetAce(acl, index, &raw)) {
      throw std::runtime_error("Installation access list is malformed");
    }
    const auto header = static_cast<const ACE_HEADER*>(raw);
    if (header->AceFlags & INHERIT_ONLY_ACE) {
      continue;
    }
    if (header->AceType == ACCESS_DENIED_ACE_TYPE || header->AceType == SYSTEM_AUDIT_ACE_TYPE) {
      continue;
    }
    if (header->AceType != ACCESS_ALLOWED_ACE_TYPE || header->AceSize < sizeof(ACCESS_ALLOWED_ACE)) {
      throw std::runtime_error("Installation access list has an unsupported grant");
    }
    const auto entry = static_cast<const ACCESS_ALLOWED_ACE*>(raw);
    constexpr size_t sid_offset = offsetof(ACCESS_ALLOWED_ACE, SidStart);
    constexpr size_t sid_header = offsetof(SID, SubAuthority);
    const auto sid = reinterpret_cast<const SID*>(&entry->SidStart);
    if (header->AceSize < sid_offset + sid_header ||
        sid->SubAuthorityCount > SID_MAX_SUB_AUTHORITIES ||
        sid_offset + sid_header + static_cast<size_t>(sid->SubAuthorityCount) * sizeof(DWORD) >
            header->AceSize || !IsValidSid(const_cast<SID*>(sid))) {
      throw std::runtime_error("Installation access list has an invalid grant identity");
    }
    if (!TrustedSid(const_cast<DWORD*>(&entry->SidStart)) && (entry->Mask & dangerous)) {
      throw std::runtime_error("Installation path grants untrusted write or replacement rights");
    }
  }
}

FileHandle OpenProtectedFile(const std::filesystem::path& path) {
  auto current = path.root_path();
  if (current.empty() || !ValidatePath(current, true)) {
    throw std::runtime_error("Protected file volume is unavailable");
  }
  for (const auto& part : path.parent_path().relative_path()) {
    current /= part;
    if (!ValidatePath(current, false)) {
      return {};
    }
  }
  FileHandle file(CreateFileW(path.c_str(), GENERIC_READ | READ_CONTROL, FILE_SHARE_READ,
                             nullptr, OPEN_EXISTING, FILE_FLAG_OPEN_REPARSE_POINT, nullptr));
  if (file.get() == INVALID_HANDLE_VALUE) {
    file.release();
    const DWORD error = GetLastError();
    if (error == ERROR_FILE_NOT_FOUND || error == ERROR_PATH_NOT_FOUND) {
      return {};
    }
    throw std::runtime_error("Cannot pin protected installed file");
  }
  ValidateOpenPath(file.get(), path, false);
  BY_HANDLE_FILE_INFORMATION facts{};
  if (!GetFileInformationByHandle(file.get(), &facts) ||
      (facts.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY)) {
    throw std::runtime_error("Installed file is not a regular file");
  }
  return file;
}

void ValidateProtectedTree(const std::filesystem::path& path) {
  auto current = path.root_path();
  if (current.empty() || !ValidatePath(current, true)) {
    throw std::runtime_error("Installation volume is unavailable");
  }
  for (const auto& part : path.relative_path()) {
    current /= part;
    if (!ValidatePath(current, false)) {
      return;
    }
  }
  size_t count = 0;
  uint64_t bytes = 0;
  for (const auto& entry : std::filesystem::recursive_directory_iterator(path)) {
    if (++count > 4096 || !ValidatePath(entry.path(), false)) {
      throw std::runtime_error("Installation tree exceeds its bound or changed during inspection");
    }
    if (entry.is_regular_file()) {
      bytes += entry.file_size();
      if (bytes > 2ULL * 1024 * 1024 * 1024) {
        throw std::runtime_error("Installation tree exceeds 2 GiB");
      }
    }
  }
}

}  // namespace ame::installer
