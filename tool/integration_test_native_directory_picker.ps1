$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$support = Join-Path (Get-AmeRepositoryRoot) "integration_test/support"
$sources = @(
    "NativeDirectoryPicker.cs"
    "NativeDirectoryActionTests.cs"
) | ForEach-Object {
    [IO.File]::ReadAllText((Join-Path $support $_), [Text.Encoding]::UTF8)
}
Add-Type -TypeDefinition ($sources -join "`n")
[Ame.Integration.Tests.NativeDirectoryActionTests]::Run()
Invoke-AmePowerShellSyntaxCheck @((Join-Path $support "control_native_directory.ps1"))
Write-Output "native_directory_picker_lifecycle_passed"
