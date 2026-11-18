[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [string]$ArchivePath,
    [switch]$Accelerator,
    [string]$DirectMLArchivePath
)

$ErrorActionPreference = "Stop"
trap {
    $message = $_.Exception.Message -replace '\r?\n', ' '
    Write-Host "${PSCommandPath}: error AME_CLASSIFICATION_RUNTIME: $message"
    throw
}
. (Join-Path $PSScriptRoot "release_classification_runtime.ps1")
. (Join-Path $PSScriptRoot "release_download_classification_runtime.ps1")

$manifest = Get-AmeClassificationRuntimeManifest -Accelerator:$Accelerator
$output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
$build = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $PSScriptRoot) "build"))
if (-not $output.StartsWith("$build\", [StringComparison]::OrdinalIgnoreCase)) {
    throw "Classification runtime build storage must be below the repository build directory"
}
Assert-AmeClassificationRuntimeLocalPath $output
if (Test-Path -LiteralPath $output) {
    Assert-AmeClassificationRuntimeDirectory $output $manifest
    Write-Host "Verified bundled classification runtime: $($manifest.bundleDirectory)"
    return
}

$parent = [IO.Path]::GetDirectoryName($output)
[void][IO.Directory]::CreateDirectory($parent)
$scratch = Join-Path $parent (".classification-runtime-" + [Guid]::NewGuid().ToString("N"))
[void][IO.Directory]::CreateDirectory($scratch)
$prepared = Join-Path $scratch "prepared"
[void][IO.Directory]::CreateDirectory($prepared)
$parts = @(if ($Accelerator) {
    @($manifest.archives | ForEach-Object {
        $sourceArchive = $_
        [pscustomobject]@{
            archive = $sourceArchive
            files = @($manifest.files | Where-Object archive -CEQ $sourceArchive.id)
        }
    })
} else { $manifest })
$ownedFiles = [Collections.Generic.List[string]]::new()
$ownedDirectories = [Collections.Generic.List[string]]::new()
try {
    for ($index = 0; $index -lt $parts.Count; $index++) {
        $part = $parts[$index]
        $download = Join-Path $scratch "$index.zip"
        $extracted = Join-Path $scratch "$index"
        $ownedFiles.Add($download)
        $ownedDirectories.Add($extracted)
        foreach ($file in $part.files) { $ownedFiles.Add((Join-Path $extracted $file.name)) }
        $archive = if ($index -eq 0) { $ArchivePath } else { $DirectMLArchivePath }
        if ([string]::IsNullOrWhiteSpace($archive)) {
            Receive-AmeClassificationRuntimeArchive $part.archive $download
            $archive = $download
        }
        Expand-AmeClassificationRuntimeArchive $archive $extracted $part
        foreach ($file in $part.files) {
            [IO.File]::Move((Join-Path $extracted $file.name), (Join-Path $prepared $file.name))
        }
    }
    Assert-AmeClassificationRuntimeDirectory $prepared $manifest
    Assert-AmeClassificationRuntimeLocalPath $output
    [IO.Directory]::Move($prepared, $output)
    Write-Host "Prepared bundled classification runtime: $($manifest.bundleDirectory)"
} finally {
    # Only this invocation's verified flat extraction and downloads are disposable.
    foreach ($file in $manifest.files) { $ownedFiles.Add((Join-Path $prepared $file.name)) }
    foreach ($path in $ownedFiles) {
        if (Test-Path -LiteralPath $path) {
            Assert-AmeClassificationRuntimeLocalPath $path
            Remove-Item -LiteralPath $path -Force
        }
    }
    foreach ($path in $ownedDirectories.ToArray() + @($prepared, $scratch)) {
        if (Test-Path -LiteralPath $path) { [IO.Directory]::Delete($path, $false) }
    }
}
