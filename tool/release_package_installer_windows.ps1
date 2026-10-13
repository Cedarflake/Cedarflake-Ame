[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [Parameter(Mandatory = $true)][string]$BundlePath,
    [Parameter(Mandatory = $true)][string]$UnsignedEvidencePath,
    [Parameter(Mandatory = $true)][string]$WixSdkPackagePath,
    [Parameter(Mandatory = $true)][string]$WixSdkDirectory,
    [Parameter(Mandatory = $true)][string]$WixUiPackagePath,
    [Parameter(Mandatory = $true)][string]$WixUiDirectory,
    [Parameter(Mandatory = $true)][string]$WixBootstrapperPackagePath,
    [Parameter(Mandatory = $true)][string]$WixBootstrapperDirectory,
    [Parameter(Mandatory = $true)][string]$CMakePath,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [switch]$AcceptWixBinaryEula
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "quality_unsigned_windows_payload.ps1")
. (Join-Path $PSScriptRoot "release_installer_payload.ps1")
. (Join-Path $PSScriptRoot "release_installer_toolchain.ps1")
. (Join-Path $PSScriptRoot "release_installer_database.ps1")
. (Join-Path $PSScriptRoot "release_installer_policy.ps1")
. (Join-Path $PSScriptRoot "release_test_installer_policy.ps1")
. (Join-Path $PSScriptRoot "release_installer_bundle.ps1")

if (-not $AcceptWixBinaryEula) {
    throw "Read WiX 7 OSMFEULA.txt and explicitly pass -AcceptWixBinaryEula before using its binary toolchain"
}
if (-not [Environment]::Is64BitProcess) { throw "MSI packaging requires a native x64 process" }
$repository = Get-AmeRepositoryRoot
$version = Get-AmeInstallerVersion $Tag
& (Join-Path $PSScriptRoot "release_validate_version.ps1") -Tag $Tag
$bundle = [IO.Path]::GetFullPath($BundlePath)
$output = [IO.Path]::GetFullPath($OutputDirectory)
$buildRoot = Join-Path $repository "build"
if (-not $output.StartsWith($buildRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw "Installer output must be a dedicated directory beneath repository build storage"
}
foreach ($path in @($bundle, $output, $UnsignedEvidencePath, $CMakePath)) {
    Assert-AmeUnsignedBuildPath $path
}
$msi = Join-Path $output "Cedarflake-Ame-$Tag-windows-x64.msi"
if (Test-Path -LiteralPath $msi) { throw "Installer output already exists; preserve the previous package" }
$evidence = Get-Content -LiteralPath $UnsignedEvidencePath -Raw -Encoding utf8 | ConvertFrom-Json
if ($evidence.format -cne "cedarflake-ame-unsigned-quality-v1" -or $evidence.status -cne "passed" -or
    $evidence.payload.machine -cne "windows-x64" -or $evidence.payload.configuration -cne "Release" -or
    $evidence.bridgeSmoke -cne "passed_without_catalog_access" -or
    @($evidence.payload.files).Count -ne $evidence.payload.fileCount) {
    throw "Installer requires complete unsigned Release payload verification"
}
Assert-AmeInstallerPayloadNames $evidence.payload.files
$toolLock = Enter-AmeRepositoryToolLock
$leases = [Collections.Generic.List[IDisposable]]::new()
try {
    [void][IO.Directory]::CreateDirectory($output)
    $wix = Get-AmeInstallerWixTool -PackagePath $WixSdkPackagePath -SdkDirectory $WixSdkDirectory
    $wixUi = Get-AmeInstallerWixUi -PackagePath $WixUiPackagePath -Directory $WixUiDirectory
    $wixBootstrapper = Get-AmeInstallerWixBootstrapper -PackagePath $WixBootstrapperPackagePath `
        -Directory $WixBootstrapperDirectory
    foreach ($entry in $evidence.payload.files) {
        $path = Join-Path $bundle $entry.relativePath
        Assert-AmeUnsignedBuildPath $path
        $stream = [IO.FileStream]::new($path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        $leases.Add($stream)
        if ($stream.Length -ne $entry.length -or
            (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $entry.sha256) {
            throw "Installer payload differs from its verified Release manifest"
        }
    }
    $nativeBuild = Join-Path $output "native"
    Invoke-AmeChecked $CMakePath @("-S", (Join-Path $repository "windows/installer"), "-B", $nativeBuild,
        "-G", "Visual Studio 17 2022", "-A", "x64")
    Invoke-AmeChecked $CMakePath @("--build", $nativeBuild, "--config", "Release", "--parallel", "1")
    & (Join-Path $nativeBuild "Release/ame_installer_policy_test.exe")
    if ($LASTEXITCODE -ne 0) { throw "Installer native policy tests failed" }
    $admission = Join-Path $nativeBuild "Release/ame_installer_admission.dll"
    Assert-AmeUnsignedX64Image $admission
    $leases.Add([IO.FileStream]::new($admission, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read))
    $generated = Join-Path $output "application_files.wxs"
    New-AmeInstallerApplicationSource -Files $evidence.payload.files -BundlePath $bundle -OutputPath $generated
    $broker = @($evidence.payload.files | Where-Object { $_.relativePath -ceq "cedarflake_ame_journal_broker.exe" })[0]
    $identity = "AMEJBID3|protocol=5|max_frame=1048576|sha256=" + ([string]$broker.sha256).ToUpperInvariant()
    $productCode = Get-AmeInstallerGuid "product/$version"
    $sources = @((Join-Path $repository "windows/installer/package.wxs"),
        (Join-Path $repository "windows/installer/ui.wxs"),
        (Join-Path $repository "windows/installer/options.wxs"), $generated)
    foreach ($source in $sources) {
        $leases.Add([IO.FileStream]::new($source, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read))
    }
    # WIX1149 also warns on SID/privilege configuration; its suggested Util replacement only
    # supports failure actions. ADR 0027 limits native rows to types 5/6 and requires SCM readback.
    $arguments = @("-acceptEula", "wix7", "build", "-arch", "x64", "-wx", "-sw1149", "-ct", "1",
        "-ext", $wixUi, "-culture", "zh-CN",
        "-d", "ProductVersion=$version", "-d", "ProductCode=$productCode",
        "-d", "BrokerPath=$(Join-Path $bundle 'cedarflake_ame_journal_broker.exe')",
        "-d", "BrokerIdentity=$identity", "-d", "AdmissionDll=$admission",
        "-d", "ApplicationIcon=$(Join-Path $repository 'windows/runner/resources/app_icon.ico')",
        "-intermediatefolder", (Join-Path $output "intermediate"), "-out", $msi) + $sources
    Invoke-AmeChecked $wix $arguments
    $receipt = [ordered]@{
        format = "cedarflake-ame-msi-v1"
        status = "built_not_installed"
        sourceCommit = $evidence.sourceCommit
        dirtyWorktree = $evidence.dirtyWorktree
        productVersion = $version
        productCode = $productCode
        upgradeCode = $script:AmeInstallerUpgradeCode
        packageSha256 = (Get-FileHash -LiteralPath $msi -Algorithm SHA256).Hash
        admissionSha256 = (Get-FileHash -LiteralPath $admission -Algorithm SHA256).Hash
        brokerIdentity = $identity
        payload = $evidence.payload
        createdUtc = [DateTime]::UtcNow.ToString("o")
    }
    $model = Read-AmeInstallerDatabase -PackagePath $msi `
        -DtfAssemblyPath (Join-Path $WixSdkDirectory "tools/net472/WixToolset.Dtf.WindowsInstaller.dll")
    Assert-AmeInstallerPolicy $model ([pscustomobject]$receipt)
    Test-AmeInstallerPolicyRegressions $model ([pscustomobject]$receipt)
    & (Join-Path $PSScriptRoot 'release_test_installer_ui.ps1') -PackagePath $msi `
        -DtfAssemblyPath (Join-Path $WixSdkDirectory 'tools/net472/WixToolset.Dtf.WindowsInstaller.dll')
    $receipt.databasePolicy = "passed"
    $setup = New-AmeInstallerBundle -Wix $wix -BootstrapperExtension $wixBootstrapper -PackagePath $msi `
        -Version $version -OutputDirectory $output -Repository $repository
    $receipt.setupFile = [IO.Path]::GetFileName($setup)
    $receipt.setupSha256 = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash
    [IO.File]::WriteAllText((Join-Path $output "package.json"), ($receipt | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
    Write-Host "Windows Installer package built: $msi"
} finally {
    foreach ($lease in $leases) { $lease.Dispose() }
    Exit-AmeRepositoryToolLock $toolLock
}
