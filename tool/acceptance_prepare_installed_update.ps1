[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ParentPath,
    [Parameter(Mandatory)][string]$HelperPath,
    [Parameter(Mandatory)][string]$BaselineInstallerPath,
    [Parameter(Mandatory)][string]$InstallerPath,
    [Parameter(Mandatory)][string]$InstallerReceiptPath,
    [Parameter(Mandatory)][string]$MsvcRuntimePath,
    [Parameter(Mandatory)][string]$VolumePreparationPath,
    [string]$OutputDirectory = 'build/test/fixtures/application-update-installed'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'quality_windows_pe_image.ps1')
$output = Assert-AmeAcceptanceScratchPath $OutputDirectory
if (-not $output.StartsWith((Join-Path (Get-AmeRepositoryRoot) 'build/test/fixtures') + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Reusable update fixtures must remain beneath build/test/fixtures'
}
if (Test-Path -LiteralPath $output) { throw 'Installed fixture directory must be unused' }
if ((Get-FileHash -LiteralPath $BaselineInstallerPath -Algorithm SHA256).Hash -ine
    '7232636a4d5a3588ea18179a1e08c0cdeda771d997b4d28414456db36fd735c1') { throw 'Baseline is not the retained Alpha 3 installer' }
$receipt = Get-Content -LiteralPath $InstallerReceiptPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($receipt.format -cne 'cedarflake-ame-msi-v1' -or
    (Get-FileHash -LiteralPath $InstallerPath -Algorithm SHA256).Hash -ine $receipt.packageSha256) { throw 'Target MSI differs from its release receipt' }
$application = @($receipt.payload.files | Where-Object relativePath -CEQ 'cedarflake_ame.exe')
$helper = @($receipt.payload.files | Where-Object relativePath -CEQ 'cedarflake_ame_update.exe')
if ($application.Count -ne 1 -or $helper.Count -ne 1 -or
    (Get-FileHash -LiteralPath $HelperPath -Algorithm SHA256).Hash -ine $helper[0].sha256) { throw 'Shipping helper differs from the target MSI payload' }
Assert-AmeWindowsPeImage $ParentPath x64
Assert-AmeWindowsPeImage $HelperPath x64
$files = [ordered]@{
    'baseline.msi' = $BaselineInstallerPath
    'target.msi' = $InstallerPath
    'vc_redist.exe' = $MsvcRuntimePath
    'parent.exe' = $ParentPath
    'helper.exe' = $HelperPath
    'volume.ps1' = $VolumePreparationPath
    'guest.ps1' = Join-Path $PSScriptRoot 'acceptance_installed_update_guest.ps1'
    'worker.ps1' = Join-Path $PSScriptRoot 'acceptance_installed_update_worker.ps1'
    'user.ps1' = Join-Path $PSScriptRoot 'acceptance_guest_user.ps1'
    'desktop.ps1' = Join-Path $PSScriptRoot 'acceptance_guest_desktop.ps1'
}
$lock = Enter-AmeRepositoryToolLock
try {
    [void][IO.Directory]::CreateDirectory($output)
    foreach ($file in $files.GetEnumerator()) { Copy-Item -LiteralPath $file.Value -Destination (Join-Path $output $file.Key) }
    $inputs = @(Get-ChildItem -LiteralPath $output -File | ForEach-Object {
        @{ name = $_.Name; length = $_.Length; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    })
    $plan = [ordered]@{ format = 'ame-installed-update-fixture-v1'; files = $inputs
        packageSha256 = $receipt.packageSha256.ToLowerInvariant(); packageLength = (Get-Item -LiteralPath $InstallerPath).Length
        targetVersion = $receipt.releaseTag.Substring(1); applicationVersion = $receipt.applicationVersion
        applicationSha256 = $application[0].sha256; productCode = $receipt.productCode }
    [IO.File]::WriteAllText((Join-Path $output 'plan.json'), ($plan | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
} finally { Exit-AmeRepositoryToolLock $lock }
Write-Output $output
