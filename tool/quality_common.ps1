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
