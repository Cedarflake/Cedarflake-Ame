#include "../policy.h"

#include <sddl.h>

#include <iostream>
#include <stdexcept>

int RunIdentityTests();
int RunLifecycleTests();
int RunPrivilegeTests();

namespace {

void CheckDescriptor(const wchar_t* sddl, bool volume_root, bool admitted) {
  PSECURITY_DESCRIPTOR raw = nullptr;
  if (!ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl, SDDL_REVISION_1, &raw, nullptr)) {
    throw std::runtime_error("Invalid descriptor fixture");
  }
  const ame::installer::LocalAllocation descriptor(raw);
  bool accepted = true;
  try {
    ame::installer::ValidateDescriptor(raw, volume_root);
  } catch (const std::exception&) {
    accepted = false;
  }
  if (accepted != admitted) {
    throw std::runtime_error("Protected path policy did not match fixture expectation");
  }
}

void RejectTruncatedGrantSid(BYTE subauthorities) {
  PSECURITY_DESCRIPTOR raw = nullptr;
  if (!ConvertStringSecurityDescriptorToSecurityDescriptorW(
          L"O:BAG:BAD:P(A;;0x1200A9;;;BU)", SDDL_REVISION_1, &raw, nullptr)) {
    throw std::runtime_error("Cannot construct malformed grant fixture");
  }
  const ame::installer::LocalAllocation descriptor(raw);
  PACL acl = nullptr;
  BOOL present = FALSE;
  BOOL defaulted = FALSE;
  void* grant = nullptr;
  if (!GetSecurityDescriptorDacl(raw, &present, &acl, &defaulted) || !present ||
      !GetAce(acl, 0, &grant)) {
    throw std::runtime_error("Cannot access owned malformed grant fixture");
  }
  auto entry = static_cast<ACCESS_ALLOWED_ACE*>(grant);
  reinterpret_cast<SID*>(&entry->SidStart)->SubAuthorityCount = subauthorities;
  try {
    ame::installer::ValidateDescriptor(raw, false);
  } catch (const std::runtime_error&) {
    return;
  }
  throw std::runtime_error("Truncated grant identity was admitted");
}

}  // namespace

int main() {
  try {
    CheckDescriptor(L"O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200A9;;;BU)", false, true);
    CheckDescriptor(L"O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FW;;;BU)", false, false);
    CheckDescriptor(L"O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;;SD;;;BU)", false, false);
    CheckDescriptor(L"O:BUG:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)", false, false);
    CheckDescriptor(L"O:BAG:BAD:NO_ACCESS_CONTROL", false, false);
    CheckDescriptor(L"O:BAG:BAD:P(A;;FA;;;BA)(A;;0x4;;;BU)", true, true);
    CheckDescriptor(L"O:BAG:BAD:P(A;;FA;;;BA)(A;;0x40;;;BU)", true, false);
    CheckDescriptor(L"O:BAG:BAD:P(A;;FA;;;BA)(A;OICIIO;FA;;;BU)", false, true);
    RejectTruncatedGrantSid(15);
    RejectTruncatedGrantSid(255);
    if (!ame::installer::SamePath(L"C:\\Program Files\\Cedarflake Ame\\",
                                  L"\\\\?\\C:\\Program Files\\Cedarflake Ame") ||
        ame::installer::SamePath(L"C:\\Program Files\\Ame", L"C:\\Program Files\\Ame-other")) {
      throw std::runtime_error("Fixed path comparison failed");
    }
    const auto product = ame::installer::ProductDirectory();
    if (!product.is_absolute() || product.filename() != L"Cedarflake Ame") {
      throw std::runtime_error("Known-folder destination failed");
    }
    const int identity_cases = RunIdentityTests();
    const int lifecycle_cases = RunLifecycleTests();
    const int privilege_cases = RunPrivilegeTests();
    std::cout << "installer_policy: " << 13 + identity_cases + lifecycle_cases + privilege_cases
              << " cases passed\n";
    return 0;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
