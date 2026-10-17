$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_process.ps1")

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$buildRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
if (((Get-Item -LiteralPath $buildRoot).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Windows UIA preparation fixture cannot traverse a reparse point"
}
$scratchRoot = [IO.Path]::GetFullPath(
    (Join-Path $buildRoot ("uia-preparation-" + [guid]::NewGuid().ToString("N")))
)
if (-not $scratchRoot.StartsWith($buildRoot + "\", [StringComparison]::OrdinalIgnoreCase)) {
    throw "Windows UIA preparation fixture escaped build"
}
New-Item -ItemType Directory -Path $scratchRoot | Out-Null
$probe = Join-Path $PSScriptRoot "integration_windows_accessibility_preparation_probe.ps1"
$token = [guid]::NewGuid().ToString("N")
$canRetireStorage = $false
try {
    $result = Invoke-AmeWindowsAccessibilityProbe -ProbeScriptPath $probe `
        -TargetProcessId $PID -Phase "runtime-preparation" -Token $token `
        -ResultPath (Join-Path $scratchRoot "prepared.json") -Timeout ([TimeSpan]::FromSeconds(8))
    foreach ($assembly in @("UIAutomationTypes", "UIAutomationClient")) {
        foreach ($stage in @("entered", "loaded", "verified")) {
            if ($result.assemblyProgress.$assembly.$stage -ne $true) {
                throw "Windows UIA preparation omitted its verified assembly contract"
            }
        }
    }
    if ($result.attempt -ne 0 -or $result.windowCount -ne 0 -or $result.elementCount -ne 0) {
        throw "Runtime preparation cannot substitute for an application assertion"
    }
    $failure = $null
    try {
        Invoke-AmeWindowsAccessibilityProbe -ProbeScriptPath $probe `
            -TargetProcessId $PID -Phase "application-ready" -Token $token `
            -ResultPath (Join-Path $scratchRoot "wrong-phase.json") -Timeout ([TimeSpan]::FromSeconds(8))
    } catch {
        $failure = $_.Exception
    }
    if ($null -eq $failure -or
        $failure.Message -cne "Windows UIA probe 'application-ready' failed with a nonzero process exit") {
        throw "Runtime preparation accepted an application phase"
    }
    if ($null -ne $failure.Data["ameWindowsUiaProbeCleanupFailure"]) {
        throw "Rejected preparation retained an owned process or handle"
    }
    $canRetireStorage = $true
    Write-Host ("Windows UIA preparation passed in {0} ms; application phases remain separate" -f $result.elapsedMilliseconds)
} finally {
    if ($canRetireStorage) {
        foreach ($name in @("prepared.json", "prepared.tmp", "wrong-phase.json", "wrong-phase.tmp")) {
            $path = Join-Path $scratchRoot $name
            if (Test-Path -LiteralPath $path) {
                if (((Get-Item -LiteralPath $path).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                    throw "Windows UIA preparation fixture contains an unexpected reparse point"
                }
                Remove-Item -LiteralPath $path -Force
            }
        }
        Remove-Item -LiteralPath $scratchRoot
    }
}
