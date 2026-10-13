$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

$script:aclFixtures = @{}

function Get-Acl {
    param([string]$LiteralPath)

    if (-not $script:aclFixtures.ContainsKey($LiteralPath)) {
        throw "The ACL fixture does not exist: $LiteralPath"
    }
    return $script:aclFixtures[$LiteralPath]
}

function Set-Acl {
    param(
        [string]$LiteralPath,
        [Security.AccessControl.FileSystemSecurity]$AclObject
    )

    $persisted = [Activator]::CreateInstance($AclObject.GetType())
    $persisted.SetSecurityDescriptorBinaryForm($AclObject.GetSecurityDescriptorBinaryForm())
    $script:aclFixtures[$LiteralPath] = $persisted
}

function Assert-AclRejected {
    param([scriptblock]$Action)

    $rejected = $false
    try {
        & $Action
    } catch {
        if ($_.Exception.Message -cne
            "The broker Directory ACL does not exactly match its admitted rules") {
            throw
        }
        $rejected = $true
    }
    if (-not $rejected) {
        throw "The exact ACL policy accepted an altered access mask"
    }
}

# Exercise Windows descriptors and rule constructors rather than handwritten access-mask facts.
foreach ($sddl in @(
    $script:AmeBrokerProtectedDirectorySddl,
    $script:AmeApplicationProtectedDirectorySddl
)) {
    $security = [Security.AccessControl.DirectorySecurity]::new()
    $security.SetSecurityDescriptorSddlForm($sddl)
    Set-Acl -LiteralPath "native-directory" -AclObject $security
    $facts = Get-AmeBrokerAclFacts -Path "native-directory"
    Assert-AmeBrokerExactAclFacts -Facts $facts -Kind Directory -IncludeServiceSid $false
    Assert-AmeApplicationExactAclFacts -Facts $facts -Kind Directory
}

foreach ($includeServiceSid in @($false, $true)) {
    Set-AmeBrokerDirectoryAcl -Path "broker-directory" -IncludeServiceSid $includeServiceSid
    Set-AmeBrokerFileAcl -Path "broker-file" -IncludeServiceSid $includeServiceSid
    foreach ($path in @("broker-directory", "broker-file")) {
        $facts = Get-AmeBrokerAclFacts -Path $path
        foreach ($rule in $facts.Rules) {
            if ($rule.Sid -in @($script:AmeBrokerUsersSid, (Get-AmeBrokerServiceSid))) {
                if ($rule.Rights -ne [uint64]0x1200A9) {
                    throw "The installed payload did not retain exactly read/execute plus synchronize"
                }
            }
        }
    }
}

foreach ($kind in @("Directory", "File")) {
    Set-AmeApplicationPathAcl -Path "application-$kind" -Kind $kind
    $facts = Get-AmeBrokerAclFacts -Path "application-$kind"
    $usersRule = @($facts.Rules | Where-Object { $_.Sid -ceq $script:AmeBrokerUsersSid })
    if ($usersRule.Count -ne 1 -or $usersRule[0].Rights -ne [uint64]0x1200A9) {
        throw "The Application ACL did not retain exactly read/execute plus synchronize"
    }
}

Set-AmeBrokerDirectoryAcl -Path "rejection-directory" -IncludeServiceSid $true
foreach ($sid in @($script:AmeBrokerUsersSid, (Get-AmeBrokerServiceSid))) {
    foreach ($extraRight in @(
        [Security.AccessControl.FileSystemRights]::WriteData,
        [Security.AccessControl.FileSystemRights]::Delete,
        [Security.AccessControl.FileSystemRights]::ChangePermissions,
        [Security.AccessControl.FileSystemRights]::TakeOwnership
    )) {
        $facts = Get-AmeBrokerAclFacts -Path "rejection-directory"
        $rule = @($facts.Rules | Where-Object { $_.Sid -ceq $sid })[0]
        $rule.Rights = $rule.Rights -bor [uint64]$extraRight
        Assert-AclRejected {
            Assert-AmeBrokerExactAclFacts -Facts $facts -Kind Directory -IncludeServiceSid $true
        }
    }
    $facts = Get-AmeBrokerAclFacts -Path "rejection-directory"
    $rule = @($facts.Rules | Where-Object { $_.Sid -ceq $sid })[0]
    $rule.Rights = [uint64][Security.AccessControl.FileSystemRights]::ReadAndExecute
    Assert-AclRejected {
        Assert-AmeBrokerExactAclFacts -Facts $facts -Kind Directory -IncludeServiceSid $true
    }
}

Write-Output "journal_broker_acl_guardrails_passed"
