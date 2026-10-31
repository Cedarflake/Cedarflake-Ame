[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)] [string]$ScratchRoot,
    [Parameter(Mandatory = $true)] [string]$Token
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_session_protocol.ps1")
if ([Threading.Thread]::CurrentThread.GetApartmentState() -ne "MTA") {
    throw "Windows UIA session requires an MTA thread"
}

$sequence = 0
while ($true) {
    $requestPath = Join-Path $ScratchRoot ("uia-command-{0:D2}.json" -f $sequence)
    if (-not [IO.File]::Exists($requestPath)) {
        Start-Sleep -Milliseconds 10
        continue
    }
    $request = Read-AmeWindowsUiaSessionRequest -Path $requestPath -Token $Token -Sequence $sequence
    $resultPath = Join-Path $ScratchRoot $request.resultFile
    $completionName = Get-AmeWindowsUiaSessionCompletionName -ResultPath $resultPath -Token $Token `
        -Phase $request.phase -TargetProcessId $request.targetProcessId
    $completion = [Threading.EventWaitHandle]::OpenExisting($completionName)
    try {
        $probeName = if ($request.phase -ceq "runtime-preparation") {
            "integration_windows_accessibility_preparation_probe.ps1"
        } else {
            "integration_windows_accessibility_probe.ps1"
        }
        & (Join-Path $PSScriptRoot $probeName) -TargetProcessId $request.targetProcessId `
            -Phase $request.phase -ResultPath $resultPath -Token $Token
        $null = $completion.Set()
    } finally {
        $completion.Dispose()
    }
    $sequence += 1
}
