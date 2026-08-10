$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Get-AmeRepositoryRoot {
    return Split-Path -Parent $PSScriptRoot
}

function Resolve-AmeExecutable {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string]$FallbackPath
    )

    if (Test-Path -LiteralPath $FallbackPath -PathType Leaf) {
        return $FallbackPath
    }
    $command = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($null -ne $command) {
        return $command.Source
    }
    throw "Required executable '$Name' was not found on PATH or at '$FallbackPath'"
}

function Get-AmeToolchain {
    $flutterRoot = Join-Path $env:USERPROFILE "develop\flutter"
    return [pscustomobject]@{
        Cargo = Resolve-AmeExecutable `
            -Name "cargo" `
            -FallbackPath (Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe")
        Dart = Resolve-AmeExecutable `
            -Name "dart" `
            -FallbackPath (Join-Path $flutterRoot "bin\cache\dart-sdk\bin\dart.exe")
        Flutter = Resolve-AmeExecutable `
            -Name "flutter" `
            -FallbackPath (Join-Path $flutterRoot "bin\flutter.bat")
        Git = Resolve-AmeExecutable `
            -Name "git" `
            -FallbackPath (Join-Path $env:ProgramFiles "Git\cmd\git.exe")
    }
}

function Invoke-AmeChecked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE"
    }
}

function Invoke-AmePowerShellSyntaxCheck {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Paths
    )

    foreach ($path in $Paths) {
        $tokens = $null
        $errors = $null
        [System.Management.Automation.Language.Parser]::ParseFile(
            $path,
            [ref]$tokens,
            [ref]$errors
        ) | Out-Null
        if ($errors.Count -gt 0) {
            $messages = $errors | ForEach-Object { $_.Message }
            throw "PowerShell syntax check failed for '$path': $($messages -join '; ')"
        }
    }
}

function Invoke-AmeJsonSyntaxCheck {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Paths
    )

    foreach ($path in $Paths) {
        Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json | Out-Null
    }
}

function Assert-AmeToolScriptNames {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Names
    )

    $namePattern = "^(quality|integration|acceptance|performance|release|bridge)_[a-z0-9_]+\.ps1$"
    $invalidNames = @($Names | Where-Object { $_ -notmatch $namePattern })
    if ($invalidNames.Count -gt 0) {
        throw "Tool scripts require an approved category prefix: $($invalidNames -join ', ')"
    }
}

function Assert-AmeWorkflowNames {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Names
    )

    $namePattern = "^(quality|integration|acceptance|performance|release|bridge)_[a-z0-9_]+\.ya?ml$"
    $invalidNames = @($Names | Where-Object { $_ -notmatch $namePattern })
    if ($invalidNames.Count -gt 0) {
        throw "Workflow files require an approved category prefix: $($invalidNames -join ', ')"
    }
}

function Enter-AmeRepositoryToolLock {
    param(
        [int]$TimeoutMinutes = 30
    )

    $mutex = [System.Threading.Mutex]::new(
        $false,
        "Local\CedarflakeAmeRepositoryToolingV1"
    )
    try {
        if ($mutex.WaitOne(0)) {
            return $mutex
        }
        Write-Host "Waiting for another Cedarflake Ame tool command to finish..."
        if (-not $mutex.WaitOne([TimeSpan]::FromMinutes($TimeoutMinutes))) {
            throw "Timed out waiting for the Cedarflake Ame repository tool lock"
        }
        return $mutex
    } catch [System.Threading.AbandonedMutexException] {
        return $mutex
    } catch {
        $mutex.Dispose()
        throw
    }
}

function Exit-AmeRepositoryToolLock {
    param(
        [Parameter(Mandatory = $true)]
        [System.Threading.Mutex]$Mutex
    )

    try {
        $Mutex.ReleaseMutex()
    } finally {
        $Mutex.Dispose()
    }
}
