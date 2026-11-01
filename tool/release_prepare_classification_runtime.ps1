[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [string]$ArchivePath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_classification_runtime.ps1")

$manifest = Get-AmeClassificationRuntimeManifest
$output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
$build = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $PSScriptRoot) "build"))
if (-not $output.StartsWith("$build\", [StringComparison]::OrdinalIgnoreCase)) {
    throw "Classification runtime build storage must be below the repository build directory"
}
Assert-AmeClassificationRuntimeLocalPath $output
if (Test-Path -LiteralPath $output) {
    Assert-AmeClassificationRuntimeDirectory $output $manifest
    Write-Host "Verified bundled ONNX Runtime $($manifest.version)"
    return
}

$parent = [IO.Path]::GetDirectoryName($output)
[void][IO.Directory]::CreateDirectory($parent)
$scratch = Join-Path $parent (".classification-runtime-" + [Guid]::NewGuid().ToString("N"))
[void][IO.Directory]::CreateDirectory($scratch)
$download = Join-Path $scratch "runtime.zip"
$prepared = Join-Path $scratch "prepared"
try {
    if ([string]::IsNullOrWhiteSpace($ArchivePath)) {
        Add-Type -AssemblyName System.Net.Http
        $client = [Net.Http.HttpClient]::new()
        $client.DefaultRequestHeaders.UserAgent.ParseAdd("Cedarflake-Ame-build/1.0")
        $timeout = [Threading.CancellationTokenSource]::new([TimeSpan]::FromMinutes(5))
        $response = $null
        try {
            $response = $client.GetAsync(
                $manifest.archive.url, [Net.Http.HttpCompletionOption]::ResponseHeadersRead, $timeout.Token
            ).GetAwaiter().GetResult()
            [void]$response.EnsureSuccessStatusCode()
            if ($response.Content.Headers.ContentLength -gt $manifest.archive.length) {
                throw "Runtime download exceeds its admitted size"
            }
            $source = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
            $destination = [IO.File]::Open($download, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
            try {
                $buffer = [byte[]]::new(65536)
                $total = 0L
                while (($read = $source.ReadAsync($buffer, 0, $buffer.Length, $timeout.Token).GetAwaiter().GetResult()) -gt 0) {
                    $total += $read
                    if ($total -gt $manifest.archive.length) { throw "Runtime download exceeds its admitted size" }
                    $destination.Write($buffer, 0, $read)
                }
            } finally {
                $destination.Dispose()
                $source.Dispose()
            }
        } finally {
            if ($null -ne $response) { $response.Dispose() }
            $timeout.Dispose()
            $client.Dispose()
        }
        $ArchivePath = $download
    }
    Expand-AmeClassificationRuntimeArchive $ArchivePath $prepared $manifest
    Assert-AmeClassificationRuntimeLocalPath $output
    [IO.Directory]::Move($prepared, $output)
    Write-Host "Prepared bundled ONNX Runtime $($manifest.version)"
} finally {
    # Only this invocation's flat extraction and download are disposable.
    foreach ($file in $manifest.files) {
        $path = Join-Path $prepared $file.name
        if (Test-Path -LiteralPath $path) {
            Assert-AmeClassificationRuntimeLocalPath $path
            Remove-Item -LiteralPath $path -Force
        }
    }
    if (Test-Path -LiteralPath $prepared) { [IO.Directory]::Delete($prepared, $false) }
    if (Test-Path -LiteralPath $download) { Remove-Item -LiteralPath $download -Force }
    [IO.Directory]::Delete($scratch, $false)
}
