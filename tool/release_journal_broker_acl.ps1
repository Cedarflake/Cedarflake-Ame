function ConvertTo-AmeBrokerAccessMask {
    param(
        [Parameter(Mandatory = $true)]
        [Security.AccessControl.FileSystemRights]$Rights
    )

    $bytes = [BitConverter]::GetBytes([int32]$Rights)
    return [uint64]([BitConverter]::ToUInt32($bytes, 0))
}

function Get-AmeBrokerAclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $acl = Get-Acl -LiteralPath $Path
    try {
        $ownerSid = ([Security.Principal.NTAccount]::new(
            [string]$acl.Owner
        )).Translate([Security.Principal.SecurityIdentifier]).Value
    } catch {
        if ([string]$acl.Owner -match '^S-\d-') {
            $ownerSid = [string]$acl.Owner
        } else {
            throw "The owner of $Path cannot be resolved to a SID"
        }
    }
    try {
        $groupSid = ([Security.Principal.NTAccount]::new(
            [string]$acl.Group
        )).Translate([Security.Principal.SecurityIdentifier]).Value
    } catch {
        if ([string]$acl.Group -match '^S-\d-') {
            $groupSid = [string]$acl.Group
        } else {
            throw "The group of $Path cannot be resolved to a SID"
        }
    }
    $rules = @(
        foreach ($rule in $acl.Access) {
            $identityText = [string]$rule.IdentityReference
            $sid = $null
            try {
                $sid = $rule.IdentityReference.Translate(
                    [Security.Principal.SecurityIdentifier]
                ).Value
            } catch {
                $sid = $null
            }
            [pscustomobject]@{
                Sid = $sid
                Identity = $identityText
                Rights = ConvertTo-AmeBrokerAccessMask -Rights $rule.FileSystemRights
                AccessControlType = [string]$rule.AccessControlType
                InheritanceFlags = [int]$rule.InheritanceFlags
                PropagationFlags = [int]$rule.PropagationFlags
                IsInherited = [bool]$rule.IsInherited
            }
        }
    )
    return [pscustomobject]@{
        OwnerSid = $ownerSid
        GroupSid = $groupSid
        AreAccessRulesProtected = [bool]$acl.AreAccessRulesProtected
        Rules = $rules
    }
}

function Get-AmeBrokerExpectedAclRules {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid
    )

    $inheritanceFlags = if ($Kind -ceq "Directory") { 3 } else { 0 }
    $readAndExecuteRights = [uint64](
        [Security.AccessControl.FileSystemRights]::ReadAndExecute -bor
        [Security.AccessControl.FileSystemRights]::Synchronize
    )
    $rules = @(
        [pscustomobject]@{
            Sid = $script:AmeBrokerSystemSid
            Rights = [uint64]2032127
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerAdministratorsSid
            Rights = [uint64]2032127
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerUsersSid
            Rights = $readAndExecuteRights
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        }
    )
    if ($IncludeServiceSid) {
        $rules += [pscustomobject]@{
            Sid = Get-AmeBrokerServiceSid
            Rights = $readAndExecuteRights
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        }
    }
    return $rules
}

function Assert-AmeBrokerExactAclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts,
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid
    )

    if ([string]$Facts.OwnerSid -cne $script:AmeBrokerAdministratorsSid) {
        throw "The broker $Kind ACL owner must be BUILTIN Administrators"
    }
    if ([string]$Facts.GroupSid -cne $script:AmeBrokerAdministratorsSid) {
        throw "The broker $Kind ACL group must be BUILTIN Administrators"
    }
    if (-not $Facts.AreAccessRulesProtected) {
        throw "The broker $Kind ACL must be protected from inheritance"
    }
    $actualRules = @($Facts.Rules)
    $expectedRules = @(Get-AmeBrokerExpectedAclRules `
        -Kind $Kind `
        -IncludeServiceSid $IncludeServiceSid)
    if ($actualRules.Count -ne $expectedRules.Count) {
        throw "The broker $Kind ACL contains an unexpected access rule"
    }
    foreach ($expected in $expectedRules) {
        $matches = @(
            $actualRules | Where-Object {
                [string]$_.Sid -ceq [string]$expected.Sid -and
                [uint64]$_.Rights -eq [uint64]$expected.Rights -and
                $_.AccessControlType -ceq "Allow" -and
                [int]$_.InheritanceFlags -eq [int]$expected.InheritanceFlags -and
                [int]$_.PropagationFlags -eq [int]$expected.PropagationFlags -and
                -not $_.IsInherited
            }
        )
        if ($matches.Count -ne 1) {
            throw "The broker $Kind ACL does not exactly match its admitted rules"
        }
    }
}

function Get-AmeApplicationExpectedAclRules {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind
    )

    $inheritanceFlags = if ($Kind -ceq "Directory") { 3 } else { 0 }
    $readAndExecuteRights = [uint64](
        [Security.AccessControl.FileSystemRights]::ReadAndExecute -bor
        [Security.AccessControl.FileSystemRights]::Synchronize
    )
    return @(
        [pscustomobject]@{
            Sid = $script:AmeBrokerSystemSid
            Rights = [uint64]2032127
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerAdministratorsSid
            Rights = [uint64]2032127
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerUsersSid
            Rights = $readAndExecuteRights
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        }
    )
}

function Assert-AmeApplicationExactAclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts,
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind
    )

    if ([string]$Facts.OwnerSid -cne $script:AmeBrokerAdministratorsSid) {
        throw "The Ame application $Kind ACL owner must be BUILTIN Administrators"
    }
    if ([string]$Facts.GroupSid -cne $script:AmeBrokerAdministratorsSid) {
        throw "The Ame application $Kind ACL group must be BUILTIN Administrators"
    }
    if (-not $Facts.AreAccessRulesProtected) {
        throw "The Ame application $Kind ACL must be protected from inheritance"
    }
    $actualRules = @($Facts.Rules)
    $expectedRules = @(Get-AmeApplicationExpectedAclRules -Kind $Kind)
    if ($actualRules.Count -ne $expectedRules.Count) {
        throw "The Ame application $Kind ACL contains an unexpected access rule"
    }
    foreach ($expected in $expectedRules) {
        $matches = @(
            $actualRules | Where-Object {
                [string]$_.Sid -ceq [string]$expected.Sid -and
                [uint64]$_.Rights -eq [uint64]$expected.Rights -and
                $_.AccessControlType -ceq "Allow" -and
                [int]$_.InheritanceFlags -eq [int]$expected.InheritanceFlags -and
                [int]$_.PropagationFlags -eq [int]$expected.PropagationFlags -and
                -not $_.IsInherited
            }
        )
        if ($matches.Count -ne 1) {
            throw "The Ame application $Kind ACL does not exactly match its admitted rules"
        }
    }
}

function Set-AmeApplicationPathAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind
    )

    $administratorsSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinAdministratorsSid,
        $null
    )
    $systemSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::LocalSystemSid,
        $null
    )
    $usersSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinUsersSid,
        $null
    )
    if ($Kind -ceq "Directory") {
        $security = [Security.AccessControl.DirectorySecurity]::new()
        $inheritance = [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor `
            [Security.AccessControl.InheritanceFlags]::ObjectInherit
    } else {
        $security = [Security.AccessControl.FileSecurity]::new()
        $inheritance = [Security.AccessControl.InheritanceFlags]::None
    }
    $security.SetAccessRuleProtection($true, $false)
    $security.SetOwner($administratorsSid)
    $security.SetGroup($administratorsSid)
    foreach ($entry in @(
        @($systemSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($administratorsSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($usersSid, [Security.AccessControl.FileSystemRights]::ReadAndExecute)
    )) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            $entry[0],
            $entry[1],
            $inheritance,
            [Security.AccessControl.PropagationFlags]::None,
            [Security.AccessControl.AccessControlType]::Allow
        )
        $security.AddAccessRule($rule) | Out-Null
    }
    Set-Acl -LiteralPath $Path -AclObject $security
    Assert-AmeApplicationExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $Path) `
        -Kind $Kind
}

function Set-AmeBrokerDirectoryAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid
    )

    $systemSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::LocalSystemSid,
        $null
    )
    $administratorsSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinAdministratorsSid,
        $null
    )
    $usersSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinUsersSid,
        $null
    )
    $directorySecurity = [Security.AccessControl.DirectorySecurity]::new()
    $directorySecurity.SetAccessRuleProtection($true, $false)
    $directorySecurity.SetOwner($administratorsSid)
    $directorySecurity.SetGroup($administratorsSid)
    $inheritance = [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor `
        [Security.AccessControl.InheritanceFlags]::ObjectInherit
    $propagation = [Security.AccessControl.PropagationFlags]::None
    $entries = @(
        @($systemSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($administratorsSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($usersSid, [Security.AccessControl.FileSystemRights]::ReadAndExecute)
    )
    if ($IncludeServiceSid) {
        $serviceSid = [Security.Principal.SecurityIdentifier]::new(
            (Get-AmeBrokerServiceSid)
        )
        $entries += ,@(
            $serviceSid,
            [Security.AccessControl.FileSystemRights]::ReadAndExecute
        )
    }
    foreach ($entry in $entries) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            $entry[0],
            $entry[1],
            $inheritance,
            $propagation,
            [Security.AccessControl.AccessControlType]::Allow
        )
        $directorySecurity.AddAccessRule($rule) | Out-Null
    }
    Set-Acl -LiteralPath $Path -AclObject $directorySecurity
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $Path) `
        -Kind "Directory" `
        -IncludeServiceSid $IncludeServiceSid
}

function Set-AmeBrokerFileAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid
    )

    $systemSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::LocalSystemSid,
        $null
    )
    $administratorsSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinAdministratorsSid,
        $null
    )
    $usersSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinUsersSid,
        $null
    )

    $fileSecurity = [Security.AccessControl.FileSecurity]::new()
    $fileSecurity.SetAccessRuleProtection($true, $false)
    $fileSecurity.SetOwner($administratorsSid)
    $fileSecurity.SetGroup($administratorsSid)
    $entries = @(
        @($systemSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($administratorsSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($usersSid, [Security.AccessControl.FileSystemRights]::ReadAndExecute)
    )
    if ($IncludeServiceSid) {
        $serviceSid = [Security.Principal.SecurityIdentifier]::new(
            (Get-AmeBrokerServiceSid)
        )
        $entries += ,@(
            $serviceSid,
            [Security.AccessControl.FileSystemRights]::ReadAndExecute
        )
    }
    foreach ($entry in $entries) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            $entry[0],
            $entry[1],
            [Security.AccessControl.AccessControlType]::Allow
        )
        $fileSecurity.AddAccessRule($rule) | Out-Null
    }
    Set-Acl -LiteralPath $Path -AclObject $fileSecurity
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $Path) `
        -Kind "File" `
        -IncludeServiceSid $IncludeServiceSid
}

function Set-AmeBrokerBinaryAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath
    )

    Set-AmeBrokerDirectoryAcl `
        -Path $InstallDirectory `
        -IncludeServiceSid $true
    Set-AmeBrokerFileAcl -Path $BinaryPath -IncludeServiceSid $true
}

function Assert-AmeBrokerBinaryAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath
    )

    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $InstallDirectory) `
        -Kind "Directory" `
        -IncludeServiceSid $true
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $BinaryPath) `
        -Kind "File" `
        -IncludeServiceSid $true
}
