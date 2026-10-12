$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Get-AmeBrokerRequiredPrivileges {
    return @("SeManageVolumePrivilege", "SeImpersonatePrivilege")
}

function Assert-AmeBrokerRequiredPrivileges {
    param([AllowEmptyCollection()][string[]]$Privileges)

    $expected = @(Get-AmeBrokerRequiredPrivileges)
    if ($Privileges.Count -ne $expected.Count) {
        throw "The broker service must retain exactly its journal and caller-impersonation privileges"
    }
    foreach ($name in $expected) {
        if (@($Privileges | Where-Object { $_ -ceq $name }).Count -ne 1) {
            throw "The broker service privilege allowlist differs from its required profile"
        }
    }
}
