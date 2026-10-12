$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_installer_payload.ps1")

function Assert-InstallerRejected {
    param([scriptblock]$Action)
    $rejected = $false
    try { & $Action } catch { $rejected = $true }
    if (-not $rejected) { throw "Unsafe installer fixture was admitted" }
}

foreach ($tag in @("v0.1.0", "v255.255.65535")) {
    if ((Get-AmeInstallerVersion $tag) -cne $tag.Substring(1)) { throw "MSI version changed" }
}
foreach ($tag in @("v0.1.0.1", "v0.1.0-rc.1", "v0.1.0+2", "v256.0.0", "v0.0.65536", "v01.2.3", "0.1.0")) {
    Assert-InstallerRejected { Get-AmeInstallerVersion $tag }
}
$firstProduct = Get-AmeInstallerGuid "product/0.1.0"
if ($firstProduct -cne (Get-AmeInstallerGuid "product/0.1.0") -or
    $firstProduct -ceq (Get-AmeInstallerGuid "product/0.1.1")) {
    throw "MSI product identity does not separate repair from upgrade"
}

$files = @(
    "cedarflake_ame.exe", "cedarflake_ame_journal_broker.exe", "rust_lib_cedarflake_ame.dll",
    "flutter_windows.dll", "data/app.so", "data/icudtl.dat", "data/flutter_assets/AssetManifest.bin"
) | ForEach-Object { [pscustomobject]@{ relativePath = $_; length = 1L; sha256 = "a" * 64 } }
Assert-AmeInstallerPayloadNames $files
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
    New-AmeInstallerApplicationSource -Files $files -BundlePath "C:\bounded-fixture" -OutputPath $output
    [xml]$source = [IO.File]::ReadAllText($output, [Text.Encoding]::UTF8)
    $namespaces = [Xml.XmlNamespaceManager]::new($source.NameTable)
    $namespaces.AddNamespace("w", $script:AmeInstallerNamespace)
    $entries = @($source.SelectNodes("//w:File", $namespaces))
    if ($entries.Count -ne 6 -or
        @($entries | Where-Object { $_.Source -like "*cedarflake_ame_journal_broker.exe" }).Count -ne 0 -or
        $source.SelectNodes("//w:Shortcut", $namespaces).Count -ne 2 -or
        $source.SelectNodes("//w:ComponentRef", $namespaces).Count -ne 11 -or
        $source.SelectNodes("//w:PermissionEx", $namespaces).Count -ne 9) {
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
