function Get-AmeBrokerServicePlan {
    return [pscustomobject]@{
        ServiceName = $script:AmeBrokerServiceName
        DisplayName = $script:AmeBrokerDisplayName
        BinaryName = $script:AmeBrokerBinaryName
        ServiceType = "own"
        StartMode = "demand"
        ServiceAccount = "LocalSystem"
        ServiceSidType = "restricted"
        RequiredPrivileges = @(Get-AmeBrokerRequiredPrivileges)
        ServiceDacl = $script:AmeBrokerServiceDacl
    }
}

function Invoke-AmeBrokerSc {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,
        [int[]]$AllowedExitCodes = @(0)
    )

    $output = @(& "$env:SystemRoot\System32\sc.exe" @Arguments 2>&1)
    $exitCode = $LASTEXITCODE
    if ($AllowedExitCodes -notcontains $exitCode) {
        throw "sc.exe $($Arguments[0]) failed with exit code $exitCode"
    }
    return [pscustomobject]@{
        ExitCode = $exitCode
        Output = ($output -join "`n")
    }
}

function Test-AmeBrokerServiceExists {
    $result = Invoke-AmeBrokerSc `
        -Arguments @("query", $script:AmeBrokerServiceName) `
        -AllowedExitCodes @(0, 1060)
    return $result.ExitCode -eq 0
}

function Stop-AmeBrokerServiceBounded {
    param(
        [int]$TimeoutSeconds = 30
    )

    if (-not (Test-AmeBrokerServiceExists)) {
        return
    }
    $service = Get-Service -Name $script:AmeBrokerServiceName
    try {
        if ($service.Status -eq [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
            return
        }
        Invoke-AmeBrokerSc -Arguments @("stop", $script:AmeBrokerServiceName) | Out-Null
        $service.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Stopped,
            [TimeSpan]::FromSeconds($TimeoutSeconds)
        )
    } finally {
        $service.Dispose()
    }
}

function Start-AmeBrokerServiceBounded {
    param(
        [int]$TimeoutSeconds = 30
    )

    Invoke-AmeBrokerSc -Arguments @("start", $script:AmeBrokerServiceName) | Out-Null
    $service = Get-Service -Name $script:AmeBrokerServiceName
    try {
        $service.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Running,
            [TimeSpan]::FromSeconds($TimeoutSeconds)
        )
    } finally {
        $service.Dispose()
    }
}

function Wait-AmeBrokerServiceDeletion {
    param(
        [int]$TimeoutMilliseconds = 30000,
        [int]$PollMilliseconds = 100,
        [scriptblock]$ServiceExistsProbe = { Test-AmeBrokerServiceExists }
    )

    if ($TimeoutMilliseconds -le 0 -or $PollMilliseconds -le 0) {
        throw "The service deletion wait must be positively bounded"
    }
    $timer = [Diagnostics.Stopwatch]::StartNew()
    try {
        while (& $ServiceExistsProbe) {
            if ($timer.ElapsedMilliseconds -ge $TimeoutMilliseconds) {
                throw "The journal broker service object did not disappear before the deadline"
            }
            $remainingMilliseconds = [Math]::Max(
                1,
                $TimeoutMilliseconds - [int]$timer.ElapsedMilliseconds
            )
            Start-Sleep -Milliseconds ([Math]::Min(
                $PollMilliseconds,
                $remainingMilliseconds
            ))
        }
    } finally {
        $timer.Stop()
    }
}

function Remove-AmeBrokerServiceBounded {
    if (-not (Test-AmeBrokerServiceExists)) {
        return
    }
    Stop-AmeBrokerServiceBounded
    Invoke-AmeBrokerSc -Arguments @("delete", $script:AmeBrokerServiceName) | Out-Null
    Wait-AmeBrokerServiceDeletion
}

function Set-AmeBrokerServiceConfiguration {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$IdentityManifest,
        [switch]$Create,
        [ref]$CreatedState,
        [scriptblock]$ScInvoker
    )

    Assert-AmeBrokerIdentityManifestFacts -Manifest $IdentityManifest

    $quotedBinaryPath = '"{0}"' -f ([System.IO.Path]::GetFullPath($BinaryPath))
    if ($Create) {
        if ($null -eq $CreatedState) {
            throw "Service creation requires an explicit transaction ownership state"
        }
        $CreatedState.Value = $false
        $arguments = @(
            "create",
            $script:AmeBrokerServiceName,
            "binPath= $quotedBinaryPath",
            "type= own",
            "start= demand",
            "error= normal",
            "obj= LocalSystem",
            "DisplayName= $script:AmeBrokerDisplayName"
        )
        if ($null -eq $ScInvoker) {
            Invoke-AmeBrokerSc -Arguments $arguments | Out-Null
        } else {
            & $ScInvoker $arguments | Out-Null
        }
        $CreatedState.Value = $true
    } else {
        $arguments = @(
            "config",
            $script:AmeBrokerServiceName,
            "binPath= $quotedBinaryPath",
            "type= own",
            "start= demand",
            "error= normal",
            "obj= LocalSystem",
            "DisplayName= $script:AmeBrokerDisplayName"
        )
        if ($null -eq $ScInvoker) {
            Invoke-AmeBrokerSc -Arguments $arguments | Out-Null
        } else {
            & $ScInvoker $arguments | Out-Null
        }
    }
    foreach ($command in @(
        [pscustomobject]@{
            Arguments = @("sidtype", $script:AmeBrokerServiceName, "restricted")
        },
        [pscustomobject]@{
            Arguments = @(
                "privs",
                $script:AmeBrokerServiceName,
                (@(Get-AmeBrokerRequiredPrivileges) -join "/")
            )
        },
        [pscustomobject]@{
            Arguments = @("sdset", $script:AmeBrokerServiceName, $script:AmeBrokerServiceDacl)
        },
        [pscustomobject]@{
            Arguments = @("description", $script:AmeBrokerServiceName, $IdentityManifest)
        }
    )) {
        $arguments = [string[]]$command.Arguments
        if ($null -eq $ScInvoker) {
            Invoke-AmeBrokerSc -Arguments $arguments | Out-Null
        } else {
            & $ScInvoker $arguments | Out-Null
        }
    }
}

function Get-AmeBrokerServiceSid {
    return $script:AmeBrokerServiceSid
}

function Assert-AmeBrokerServiceSidFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ActualSid
    )

    if ([string]$ActualSid -cne $script:AmeBrokerServiceSid) {
        throw "The fixed broker service SID does not match the service name"
    }
}

function Get-AmeBrokerCalculatedServiceSid {
    $result = Invoke-AmeBrokerSc `
        -Arguments @("showsid", $script:AmeBrokerServiceName)
    $match = [regex]::Match(
        $result.Output,
        'S-1-5-80-(?:\d+-){4}\d+',
        [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (-not $match.Success) {
        throw "sc.exe did not return the broker service SID"
    }
    return $match.Value
}

function Assert-AmeBrokerServiceDaclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ActualSddl
    )

    $compact = [regex]::Replace($ActualSddl, '\s', '')
    if (-not $compact.Equals(
        $script:AmeBrokerServiceDacl,
        [System.StringComparison]::Ordinal
    )) {
        throw "The broker service DACL does not exactly match the admitted descriptor"
    }
}

function Assert-AmeBrokerServiceConfiguration {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$IdentityManifest
    )

    Assert-AmeBrokerIdentityManifestFacts -Manifest $IdentityManifest

    $serviceKeyPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$script:AmeBrokerServiceName"
    $configuration = Get-ItemProperty -LiteralPath $serviceKeyPath
    $expectedImagePath = '"{0}"' -f ([System.IO.Path]::GetFullPath($BinaryPath))
    if ([string]$configuration.ImagePath -cne $expectedImagePath) {
        throw "The broker service binary path does not match the admitted installation"
    }
    if ([int]$configuration.Type -ne 0x10 -or [int]$configuration.Start -ne 3) {
        throw "The broker service must be own-process and demand-start"
    }
    if ([string]$configuration.ObjectName -cne "LocalSystem") {
        throw "The broker service account must be LocalSystem"
    }
    if ([int]$configuration.ServiceSidType -ne 3) {
        throw "The broker service SID must be restricted"
    }
    if ([string]$configuration.Description -cne $IdentityManifest) {
        throw "The broker service identity manifest does not match the installed binary"
    }
    $resolvedServiceSid = ([Security.Principal.NTAccount]::new(
        "NT SERVICE\$script:AmeBrokerServiceName"
    )).Translate([Security.Principal.SecurityIdentifier]).Value
    Assert-AmeBrokerServiceSidFacts -ActualSid $resolvedServiceSid
    Assert-AmeBrokerRequiredPrivileges -Privileges @($configuration.RequiredPrivileges)
    $serviceDacl = [Ame.JournalBrokerInstaller.NativeMethods]::GetServiceSecuritySddl(
        $script:AmeBrokerServiceName
    )
    Assert-AmeBrokerServiceDaclFacts -ActualSddl $serviceDacl
}
