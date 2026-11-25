#include "handoff_session.h"

#include <algorithm>
#include <stdexcept>
#include <vector>

namespace ame::updater {
namespace {
std::filesystem::path ProcessImage(HANDLE process) {
  std::vector<wchar_t> path(32768);
  DWORD size = static_cast<DWORD>(path.size());
  if (!QueryFullProcessImageNameW(process, 0, path.data(), &size)) {
    throw std::runtime_error("Cannot inspect application process");
  }
  return std::wstring(path.data(), size);
}

installer::FileHandle BindParent(DWORD id, const std::filesystem::path& application) {
  HANDLE raw = nullptr;
  if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw)) {
    throw std::runtime_error("Cannot inspect update handoff privileges");
  }
  const installer::FileHandle token(raw);
  TOKEN_ELEVATION elevation{};
  DWORD returned = 0;
  if (!GetTokenInformation(raw, TokenElevation, &elevation, sizeof(elevation), &returned) ||
      elevation.TokenIsElevated) {
    throw std::runtime_error("Application updates require an ordinary desktop session");
  }
  RejectReparse(application);
  installer::FileHandle parent(OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, id));
  if (!parent || !installer::SamePath(ProcessImage(parent.get()), application)) {
    throw std::runtime_error("Application update parent differs");
  }
  return parent;
}

std::filesystem::path BindDirectory(const std::filesystem::path& ready,
                                    const std::filesystem::path& result) {
  const auto self = ProcessImage(GetCurrentProcess());
  const auto cache = UpdateCache();
  RejectReparse(self);
  if (!installer::SamePath(self.parent_path().parent_path(), cache) ||
      self.parent_path().filename().wstring().rfind(L"handoff-", 0) != 0 ||
      !installer::SamePath(ready, self.parent_path() / L"ready") ||
      !installer::SamePath(result, cache / L"result.json")) {
    throw std::runtime_error("Unexpected application update handoff");
  }
  return self.parent_path();
}

std::wstring AdmittedVersion(const std::wstring& version) {
  if (version.empty() || version.size() > 100 || !std::all_of(version.begin(), version.end(), [](wchar_t c) {
    return (c >= L'0' && c <= L'9') || (c >= L'a' && c <= L'z') ||
           (c >= L'A' && c <= L'Z') || c == L'.' || c == L'-' || c == L'+';
  })) throw std::runtime_error("Invalid update version");
  return version;
}
}  // namespace

HandoffSession::HandoffSession(DWORD parent_id, const std::filesystem::path& application,
                             const std::filesystem::path& ready, const std::filesystem::path& result,
                             const std::wstring& version)
    : application_(application), version_(AdmittedVersion(version)),
      parent_(BindParent(parent_id, application)), directory_(BindDirectory(ready, result)),
      storage_(directory_, UpdateCache()) {
  if (!startup_guard_.RetainForHandoff()) {
    throw std::runtime_error("Cannot retain the application startup guard");
  }
}

void HandoffSession::Acknowledge() const { storage_.Acknowledge(); }
bool HandoffSession::WaitForParent() const { return WaitForSingleObject(parent_.get(), INFINITE) == WAIT_OBJECT_0; }
bool HandoffSession::IsCommitted() const { return storage_.IsCommitted(); }

int HandoffSession::Reopen(DWORD code) {
  bool recorded = false;
  try { storage_.WriteResult(version_, code); recorded = true; }
  catch (const std::exception&) {}
  auto arguments = QuoteArgument(application_.native());
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  startup_guard_.Release();
  if (!CreateProcessW(application_.c_str(), arguments.data(), nullptr, nullptr, FALSE,
      0, nullptr, application_.parent_path().c_str(), &startup, &process)) {
    throw std::runtime_error("Cannot reopen the application");
  }
  CloseHandle(process.hThread);
  CloseHandle(process.hProcess);
  return recorded ? 0 : 2;
}
}  // namespace ame::updater
