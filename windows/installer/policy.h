#ifndef AME_INSTALLER_POLICY_H_
#define AME_INSTALLER_POLICY_H_

#include <windows.h>
#include <msi.h>

#include <filesystem>
#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace ame::installer {

inline constexpr wchar_t kUpgradeCode[] = L"{C284E6B0-1484-4926-A94B-C3EC3840DBBD}";
inline constexpr wchar_t kServiceName[] = L"CedarflakeAmeJournalBroker";
inline constexpr wchar_t kBrokerName[] = L"cedarflake_ame_journal_broker.exe";
inline constexpr wchar_t kBrokerIdentityPrefix[] = L"AMEJBID3|protocol=5|max_frame=1048576|sha256=";
inline constexpr wchar_t kServiceSddl[] =
    L"O:BAG:BAD:(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;SY)"
    L"(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCRPLO;;;IU)";

struct LocalFreeDeleter {
  void operator()(void* value) const noexcept { LocalFree(value); }
};
using LocalAllocation = std::unique_ptr<void, LocalFreeDeleter>;

struct HandleDeleter {
  void operator()(void* value) const noexcept { CloseHandle(value); }
};
using FileHandle = std::unique_ptr<void, HandleDeleter>;

std::filesystem::path ProgramFiles();
std::filesystem::path ProductDirectory();
bool SamePath(const std::filesystem::path& first, const std::filesystem::path& second);
void ValidateDescriptor(PSECURITY_DESCRIPTOR descriptor, bool volume_root);
void ValidateProtectedTree(const std::filesystem::path& path);
FileHandle OpenProtectedFile(const std::filesystem::path& path);
bool IsBrokerIdentity(const std::wstring& identity);
std::wstring BrokerIdentityForFile(HANDLE file);
std::optional<std::wstring> ReadInstalledBrokerIdentity();

struct BrokerServiceRegistration {
  DWORD service_type;
  DWORD start_type;
  DWORD sid_type;
  std::wstring account;
  std::wstring binary_path;
  std::vector<std::wstring> privileges;
  std::wstring description;
};

std::vector<std::wstring> ReadBrokerPrivileges(const std::vector<BYTE>& bytes, const wchar_t* text);
bool HasRequiredBrokerPrivileges(const std::vector<std::wstring>& privileges);
std::optional<BrokerServiceRegistration> ReadBrokerServiceRegistration();
void ValidateBrokerServiceRegistration(const BrokerServiceRegistration& registration,
                                       const std::wstring& identity_manifest);
struct RegisteredProduct {
  std::wstring broker_identity;
  std::filesystem::path application_directory;
};
std::optional<RegisteredProduct> ReadRegisteredProduct();
void ValidateApplicationPath(const std::filesystem::path& destination,
                             const std::filesystem::path& product);
void ValidateApplicationDestination(const std::filesystem::path& destination,
                                    const std::optional<RegisteredProduct>& registered);
void VerifyServiceConfiguration(const std::wstring& identity_manifest);

enum class BrokerStartState { Pending, Ready, Failed, Cancelled, TimedOut };
BrokerStartState ClassifyBrokerStart(const SERVICE_STATUS_PROCESS& status,
                                    bool cancelled, bool deadline_reached);
UINT StartVerifiedBroker(MSIHANDLE session, const std::wstring& identity);
void StopVerifiedBroker(const std::wstring& identity);

}  // namespace ame::installer

#endif
