function Get-AmeR2cRTestBuildArguments {
    return @(
        "test", "--locked", "--manifest-path", "rust\Cargo.toml",
        "--lib", "--no-run", "--jobs", "1"
    )
}

function Assert-AmeR2cRCargoInvocation {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("CargoExactTest", "CargoTestBuild")][string]$Kind,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [switch]$Detached
    )

    if ($Detached) {
        throw "R2c-R verified process boundary cannot detach a Cargo test or build"
    }
    $cargoFallback = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if ([IO.File]::Exists($cargoFallback)) {
        $expectedExecutable = [IO.Path]::GetFullPath($cargoFallback)
    } else {
        $cargoCommand = Microsoft.PowerShell.Core\Get-Command `
            "cargo" -CommandType Application -ErrorAction SilentlyContinue |
            Microsoft.PowerShell.Utility\Select-Object -First 1
        if ($null -eq $cargoCommand) {
            throw "R2c-R verified process boundary could not resolve Cargo"
        }
        $expectedExecutable = [IO.Path]::GetFullPath($cargoCommand.Source)
    }
    $actualExecutable = [IO.Path]::GetFullPath($FileName)
    if (-not $actualExecutable.Equals($expectedExecutable, [StringComparison]::OrdinalIgnoreCase)) {
        throw "R2c-R verified process boundary rejected the Cargo executable"
    }

    $commands = [Collections.Generic.List[object]]::new()
    if ($Kind -ceq "CargoTestBuild") {
        $commands.Add(@(Get-AmeR2cRTestBuildArguments))
    } else {
        foreach ($case in @(Get-AmeR2cRCaseDefinitions)) {
            $commands.Add(@(Get-AmeR2cRCaseArguments -Case $case))
        }
    }
    $commandMatches = 0
    foreach ($expectedArguments in $commands) {
        if ($expectedArguments.Count -ne $Arguments.Count) { continue }
        $matches = $true
        for ($index = 0; $index -lt $Arguments.Count; $index += 1) {
            if ([string]$Arguments[$index] -cne [string]$expectedArguments[$index]) {
                $matches = $false
                break
            }
        }
        if ($matches) { $commandMatches += 1 }
    }
    if ($commandMatches -ne 1) {
        throw "R2c-R verified process boundary rejected a non-matrix Cargo command"
    }
}

function Assert-AmeR2cRCargoBoundaryContract {
    param([Parameter(Mandatory = $true)][psobject]$State)

    $digests = @{
        "Get-AmeR2cRTestBuildArguments" = "97f990b69a92da24ffb5f19429cea4835bb028bf4afefa96a26ccdb27bde1f29"
        "Assert-AmeR2cRCargoInvocation" = "f23693c381c8a1333974b9be23788dcb87f8e7195066d7a9c3b0f531f1967293"
    }
    foreach ($name in $digests.Keys) {
        if (-not $State.Functions.ContainsKey($name) -or $State.Functions[$name].Count -ne 1) {
            throw "R2c-R Cargo boundary item $name was missing or ambiguous"
        }
        $actual = Get-AmeR2cRTextSha256 -Text $State.Functions[$name][0].Ast.Extent.Text
        if ($actual -cne $digests[$name]) {
            throw "R2c-R Cargo boundary item $name changed; expected=$($digests[$name]) actual=$actual"
        }
    }
}
