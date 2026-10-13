$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_privileges.ps1")

Assert-AmeBrokerRequiredPrivileges @("SeManageVolumePrivilege", "SeImpersonatePrivilege")
Assert-AmeBrokerRequiredPrivileges @("SeImpersonatePrivilege", "SeManageVolumePrivilege")
foreach ($invalid in @(
    @{ Names = @() },
    @{ Names = @("SeManageVolumePrivilege") },
    @{ Names = @("SeImpersonatePrivilege") },
    @{ Names = @("SeManageVolumePrivilege", "SeManageVolumePrivilege") },
    @{ Names = @("SeManageVolumePrivilege", "SeImpersonatePrivilege", "SeDebugPrivilege") },
    @{ Names = @("SeManageVolumePrivilege", "SeDebugPrivilege") }
)) {
    $rejected = $false
    try { Assert-AmeBrokerRequiredPrivileges -Privileges $invalid.Names } catch { $rejected = $true }
    if (-not $rejected) { throw "An invalid broker privilege profile was admitted" }
}
Write-Host "Broker required privileges: two valid orders and six rejecting profiles passed"
