$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_classification_runtime.ps1")

function Assert-RuntimeRejected {
    param([scriptblock]$Action, [string]$Expected)
    $failure = $null
    try { & $Action | Out-Null } catch { $failure = $_ }
    if ($null -eq $failure -or $failure.Exception.Message -notmatch $Expected) {
        throw "Classification runtime did not reject the expected violation: $Expected"
    }
}

function Set-FixtureArchiveDigest {
    $fixture.archive.length = (Get-Item -LiteralPath $zipPath).Length
    $fixture.archive.sha256 = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
}

$repository = Split-Path -Parent $PSScriptRoot
$scratch = Join-Path $repository ("build/runtime-distribution-test-" + [Guid]::NewGuid().ToString("N"))
$manifest = Get-AmeClassificationRuntimeManifest
$fixture = $manifest | ConvertTo-Json -Depth 10 | ConvertFrom-Json
$zipPath = Join-Path $scratch "fixture.zip"
$destination = Join-Path $scratch "prepared"
$rejected = Join-Path $scratch "rejected"
$linked = Join-Path $scratch "linked"
$utf8 = [Text.UTF8Encoding]::new($false)
$ownedFiles = [Collections.Generic.List[string]]::new()
$ownedDirectories = [Collections.Generic.List[string]]::new()
[void][IO.Directory]::CreateDirectory($scratch)
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
try {
    $archive = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in $fixture.files) {
            $bytes = $utf8.GetBytes("fixed runtime fixture: $($file.name)")
            $file.length = $bytes.Length
            $hash = [Security.Cryptography.SHA256]::Create()
            try { $file.sha256 = ([BitConverter]::ToString($hash.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant() }
            finally { $hash.Dispose() }
            $entry = $archive.CreateEntry($fixture.archive.prefix + $file.entry)
            $stream = $entry.Open()
            try { $stream.Write($bytes, 0, $bytes.Length) } finally { $stream.Dispose() }
        }
        [void]$archive.CreateEntry("../not-selected.pdb")
    } finally { $archive.Dispose() }
    Set-FixtureArchiveDigest
    foreach ($directory in @($destination, $rejected)) {
        $ownedDirectories.Add($directory)
        foreach ($file in $fixture.files) { $ownedFiles.Add((Join-Path $directory $file.name)) }
    }
    Expand-AmeClassificationRuntimeArchive $zipPath $destination $fixture
    Assert-AmeClassificationRuntimeDirectory $destination $fixture
    if (Test-Path -LiteralPath (Join-Path $scratch "not-selected.pdb")) {
        throw "Runtime preparation extracted an unselected archive entry"
    }
    Assert-RuntimeRejected { Expand-AmeClassificationRuntimeArchive $zipPath $destination $fixture } 'new directory'
    $runtime = Join-Path $destination $fixture.files[0].name
    $retained = [IO.File]::ReadAllBytes($runtime)
    $changed = [byte[]]$retained.Clone()
    $changed[0] = $changed[0] -bxor 1
    [IO.File]::WriteAllBytes($runtime, $changed)
    Assert-RuntimeRejected { Assert-AmeClassificationRuntimeDirectory $destination $fixture } 'pinned length or digest'
    [IO.File]::WriteAllBytes($runtime, $retained)
    [IO.File]::WriteAllBytes($runtime, $retained[0..($retained.Length - 2)])
    Assert-RuntimeRejected { Assert-AmeClassificationRuntimeDirectory $destination $fixture } 'pinned length or digest'
    [IO.File]::WriteAllBytes($runtime, $retained)
    $extra = Join-Path $destination 'unexpected.pdb'
    $ownedFiles.Add($extra)
    [IO.File]::WriteAllBytes($extra, $retained)
    Assert-RuntimeRejected { Assert-AmeClassificationRuntimeDirectory $destination $fixture } 'exactly the admitted payload'
    Remove-Item -LiteralPath $extra

    $fixture.archive.sha256 = '0' * 64
    Assert-RuntimeRejected { Expand-AmeClassificationRuntimeArchive $zipPath $rejected $fixture } 'pinned length or digest'
    if (Test-Path -LiteralPath $rejected) { throw "Rejected archive created extraction state" }
    Set-FixtureArchiveDigest
    $archive = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Update)
    try { $archive.GetEntry($fixture.archive.prefix + 'LICENSE').Delete() } finally { $archive.Dispose() }
    Set-FixtureArchiveDigest
    Assert-RuntimeRejected { Expand-AmeClassificationRuntimeArchive $zipPath $rejected $fixture } 'missing, duplicate or wrong-sized entry'

    New-Item -ItemType Junction -Path $linked -Target $destination | Out-Null
    Assert-RuntimeRejected { Assert-AmeClassificationRuntimeDirectory $linked $fixture } 'reparse points'
    Assert-RuntimeRejected {
        & (Join-Path $PSScriptRoot "release_prepare_classification_runtime.ps1") -OutputDirectory $repository
    } 'below the repository build directory'
    Write-Host "Classification runtime package guardrails passed"
} finally {
    if (Test-Path -LiteralPath $linked) { [IO.Directory]::Delete($linked, $false) }
    foreach ($path in $ownedFiles) {
        if (Test-Path -LiteralPath $path) {
            Assert-AmeClassificationRuntimeLocalPath $path
            Remove-Item -LiteralPath $path -Force
        }
    }
    foreach ($path in $ownedDirectories) {
        if (Test-Path -LiteralPath $path) { [IO.Directory]::Delete($path, $false) }
    }
    if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
    [IO.Directory]::Delete($scratch, $false)
}
