[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)] [int]$TargetProcessId,
    [Parameter(Mandatory = $true)]
    [ValidateSet("runtime-preparation")] [string]$Phase,
    [Parameter(Mandatory = $true)] [string]$ResultPath,
    [Parameter(Mandatory = $true)] [string]$Token
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_assemblies.ps1")

$elapsed = [Diagnostics.Stopwatch]::StartNew()
$record = @{
    ResultPath = $ResultPath; Token = $Token; Phase = $Phase
    TargetProcessId = $TargetProcessId; ProbeProcessId = $PID
}
$progress = $null
$failure = $null
$stageMilliseconds = [ordered]@{}
try {
    $progress = New-AmeWindowsUiaAssemblyProgress -ResultPath $ResultPath -Token $Token `
        -Phase $Phase -TargetProcessId $TargetProcessId -Mode "Open"
    if ([Threading.Thread]::CurrentThread.GetApartmentState() -ne "MTA") {
        throw "Windows UIA preparation requires an MTA thread"
    }
    foreach ($assembly in @("UIAutomationTypes", "UIAutomationClient")) {
        $stage = if ($assembly -ceq "UIAutomationTypes") { "loading-uia-types" } else { "loading-uia-client" }
        Write-AmeWindowsUiaProbeRecord @record -Stage $stage `
            -ElapsedMilliseconds $elapsed.ElapsedMilliseconds
        $startedMilliseconds = $elapsed.ElapsedMilliseconds
        Import-AmeWindowsUiaAssembly -Name $assembly -Progress $progress
        $stageMilliseconds[$stage] = $elapsed.ElapsedMilliseconds - $startedMilliseconds
    }
} catch {
    $failure = $_.Exception.Message
}
try {
    Write-AmeWindowsUiaProbeRecord @record -Status "complete" -Stage "complete" `
        -ElapsedMilliseconds $elapsed.ElapsedMilliseconds -Failure $failure `
        -StageMilliseconds $stageMilliseconds
} finally {
    Close-AmeWindowsUiaAssemblyProgress $progress
}
