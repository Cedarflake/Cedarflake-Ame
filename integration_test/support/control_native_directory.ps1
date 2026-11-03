param(
    [Parameter(Mandatory = $true)]
    [ValidateRange(1, [int]::MaxValue)]
    [int]$TargetProcessId,
    [ValidateSet("Confirm", "Cancel")]
    [string]$Action = "Confirm"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Add-Type -TypeDefinition ([IO.File]::ReadAllText(
    (Join-Path $PSScriptRoot "NativeDirectoryPicker.cs"), [Text.Encoding]::UTF8
))

$controlId = if ($Action -eq "Confirm") { 1 } else { 2 }
$successMarker = if ($Action -eq "Confirm") {
    "NATIVE_PICKER_CONFIRMED"
} else {
    "NATIVE_PICKER_CANCELLED"
}

Write-Output "NATIVE_PICKER_READY"
$result = [Ame.Integration.NativeDirectoryAction]::Run(
    [Ame.Integration.Win32DirectoryDialog]::new(), $TargetProcessId, $controlId, 12000, 2000
)
Write-Output (
    "NATIVE_PICKER_RESULT action={0} stage={1} elapsed_ms={2} cleanup={3}" -f
    $Action, $result.Stage, $result.ElapsedMilliseconds, $result.Cleanup
)
if (-not $result.Succeeded) {
    [Console]::Error.WriteLine("Native directory picker $Action failed: $($result.Stage)")
    exit 2
}
Write-Output $successMarker
