$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_installer_payload.ps1")

function Assert-InstallerRejected {
    param([scriptblock]$Action)
    $rejected = $false
    try { & $Action } catch { $rejected = $true }
    if (-not $rejected) { throw "Unsafe installer fixture was admitted" }
}

foreach ($case in @(
        @('v0.1.0-alpha.1', '0.1.0-alpha.1+1', '0.1.1'),
        @('v0.1.0-alpha.2', '0.1.0-alpha.2+2', '0.1.2'),
        @('v0.1.0', '0.1.0+3', '0.1.3'),
        @('v0.1.1', '0.1.1+4', '0.1.4'),
        @('v255.255.0-rc.1', '255.255.0-rc.1+65535', '255.255.65535')
    )) {
    if ((Get-AmeInstallerVersion -Tag $case[0] -ApplicationVersion $case[1]) -cne $case[2]) {
        throw "MSI upgrade version changed for $($case[0])"
    }
}
foreach ($case in @(
        @('v0.1.0.1', '0.1.0.1+1'), @('v0.1.0-rc.01', '0.1.0-rc.01+1'),
        @('v0.1.0+2', '0.1.0+2'), @('v256.0.0', '256.0.0+1'),
        @('v0.256.0', '0.256.0+1'), @('v0.1.0', '0.1.0+65536'),
        @('v01.2.3', '01.2.3+1'), @('0.1.0', '0.1.0+1'),
        @('v0.1.0', '0.1.0'), @('v0.1.0', '0.1.0+0'),
        @('v0.1.0', '0.1.0+01'), @('v0.1.0', '0.1.0+build.1'),
        @('v0.1.0-alpha.1', '0.1.0+1')
    )) {
    Assert-InstallerRejected { Get-AmeInstallerVersion -Tag $case[0] -ApplicationVersion $case[1] }
}
$firstProduct = Get-AmeInstallerGuid "product/0.1.0"
if ($firstProduct -cne (Get-AmeInstallerGuid "product/0.1.0") -or
    $firstProduct -ceq (Get-AmeInstallerGuid "product/0.1.1")) {
    throw "MSI product identity does not separate repair from upgrade"
}

$names = @(
    "cedarflake_ame.exe", "cedarflake_ame_journal_broker.exe", "cedarflake_ame_classification_worker.exe", "rust_lib_cedarflake_ame.dll",
    "flutter_windows.dll", "data/app.so", "data/icudtl.dat", "data/flutter_assets/AssetManifest.bin"
) + @(Get-AmeClassificationRuntimePayloadNames) + @(Get-AmeClassificationModelsPayloadNames)
$files = @($names | ForEach-Object { [pscustomobject]@{ relativePath = $_; length = 1L; sha256 = "a" * 64 } })
Assert-AmeInstallerPayloadNames $files
foreach ($missing in (@(Get-AmeClassificationRuntimePayloadNames) + @(Get-AmeClassificationModelsPayloadNames))) {
    $failure = $null
    try { Assert-AmeInstallerPayloadNames @($files | Where-Object { $_.relativePath -cne $missing }) } catch { $failure = $_ }
    if ($null -eq $failure -or $failure.Exception.Message -cne "Installer payload is missing $missing") {
        throw "Installer payload did not reject missing classification file: $missing"
    }
}
foreach ($badPath in @("../outside.exe", "C:/outside.exe", "data/../outside.exe", "data//x", "data/a:stream", "data/filename.")) {
    $badFiles = $files + [pscustomobject]@{ relativePath = $badPath; length = 1L; sha256 = "a" * 64 }
    Assert-InstallerRejected { Assert-AmeInstallerPayloadNames $badFiles }
}
Assert-InstallerRejected { Assert-AmeInstallerPayloadNames ($files + $files[0]) }
Assert-InstallerRejected { Assert-AmeInstallerPayloadNames $files[1..6] }

$root = Join-Path (Split-Path -Parent $PSScriptRoot) ".build"
[void][IO.Directory]::CreateDirectory($root)
$output = Join-Path $root ("installer-payload-test-" + [guid]::NewGuid().ToString("N") + ".wxs")
try {
    New-AmeInstallerApplicationSource -Files $files -BundlePath "C:\bounded-fixture" `
        -ModelDirectory "C:\verified-model-fixture" -OutputPath $output
    [xml]$source = [IO.File]::ReadAllText($output, [Text.Encoding]::UTF8)
    $namespaces = [Xml.XmlNamespaceManager]::new($source.NameTable)
    $namespaces.AddNamespace("w", $script:AmeInstallerNamespace)
    $entries = @($source.SelectNodes("//w:File", $namespaces))
    foreach ($name in Get-AmeClassificationModelsPayloadNames) {
        $modelPath = Join-Path "C:\verified-model-fixture" ([IO.Path]::GetFileName($name))
        if (@($entries | Where-Object { $_.Source -ceq $modelPath }).Count -ne 1) {
            throw 'Model installer components must bind the separately verified model payload'
        }
    }
    if ($entries.Count -ne 19 -or
        @($entries | Where-Object { $_.Source -like "*cedarflake_ame_journal_broker.exe" }).Count -ne 0 -or
        $source.SelectNodes("//w:Shortcut", $namespaces).Count -ne 2 -or
        $source.SelectNodes("//w:ComponentRef", $namespaces).Count -ne 27 -or
        $source.SelectNodes("//w:PermissionEx", $namespaces).Count -ne 25) {
        throw "MSI generated component, broker exclusion, shortcut or ACL ownership is incomplete"
    }
    foreach ($name in @('AmeStartMenu', 'AmeDesktop')) {
        $shortcut = $source.SelectSingleNode("//w:Shortcut[@Id='$name']", $namespaces)
        if ($shortcut.Target -cne '[#AmeApplicationExecutable]' -or
            $shortcut.ParentNode.LocalName -cne 'Component' -or
            $source.SelectNodes("//w:ComponentGroup[@Id='${name}Files']/w:ComponentRef", $namespaces).Count -ne 1) {
            throw 'Shortcut choices must own independent components targeting the installed executable'
        }
    }
    $ids = @($source.SelectNodes("//w:Component", $namespaces) | ForEach-Object { $_.Guid })
    if (@($ids | Sort-Object -Unique).Count -ne $ids.Count) { throw "Duplicate MSI component identity" }
} finally {
    if (Test-Path -LiteralPath $output -PathType Leaf) { Remove-Item -LiteralPath $output -Force }
}
Write-Host "Installer version, identity, payload and component guardrails passed"
