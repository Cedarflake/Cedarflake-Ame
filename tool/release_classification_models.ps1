$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_classification_runtime.ps1')

function Get-AmeClassificationModelsManifest {
    $path = Join-Path (Split-Path -Parent $PSScriptRoot) 'rust/src/adapters/classification_runtime/model_distribution.json'
    [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8) | ConvertFrom-Json
}

function Assert-AmeClassificationModelsManifest {
    param([object]$Manifest)
    if ($Manifest.files.Count -ne 3 -or $Manifest.notices.Count -ne 2) { throw 'Model payload must contain three artifacts and two notices' }
    $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $total = 0L
    foreach ($file in @($Manifest.files) + @($Manifest.notices)) {
        if ($file.name -cnotmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$' -or !$names.Add($file.name) -or
            $file.sha256 -cnotmatch '^[a-f0-9]{64}$' -or $file.length -le 0) { throw 'Model payload has an invalid artifact declaration' }
        $total += [long]$file.length
    }
    if ($total -gt 1GB) { throw 'Model payload exceeds its storage bound' }
}

function Get-AmeClassificationModelsPayloadNames {
    $manifest = Get-AmeClassificationModelsManifest
    foreach ($file in @($manifest.files) + @($manifest.notices)) { "$($manifest.bundleDirectory)/$($file.name)" }
}

function Assert-AmeClassificationModelsDirectory {
    param([string]$Path, [object]$Manifest)
    Assert-AmeClassificationModelsManifest $Manifest
    Assert-AmeClassificationRuntimeLocalPath $Path
    $files = @($Manifest.files) + @($Manifest.notices)
    if (@(Get-ChildItem -LiteralPath $Path -Force).Count -ne $files.Count) {
        throw 'Model bundle must contain exactly the admitted artifacts and notices'
    }
    foreach ($file in $files) { Assert-AmeClassificationRuntimeArtifact (Join-Path $Path $file.name) $file }
}

function Get-AmeClassificationModelsPayload {
    param([Parameter(Mandatory)][string]$Directory)
    $manifest = Get-AmeClassificationModelsManifest
    Assert-AmeClassificationModelsDirectory $Directory $manifest
    foreach ($file in @($manifest.files) + @($manifest.notices)) {
        [pscustomobject]@{ relativePath = "$($manifest.bundleDirectory)/$($file.name)"; length = [long]$file.length; sha256 = [string]$file.sha256 }
    }
}

function Copy-AmeModelArtifactStream {
    param([IO.Stream]$Source, [string]$Destination, [long]$Length, [Threading.CancellationToken]$Cancellation)
    $output = [IO.File]::Open($Destination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
    try {
        $buffer = [byte[]]::new(65536)
        $total = 0L
        while (($read = $Source.ReadAsync($buffer, 0, $buffer.Length, $Cancellation).GetAwaiter().GetResult()) -gt 0) {
            $total += $read
            if ($total -gt $Length) { throw 'Model artifact exceeds its pinned length' }
            $output.Write($buffer, 0, $read)
        }
        if ($total -ne $Length) { throw 'Model artifact is incomplete' }
        $output.Flush($true)
    } finally { $output.Dispose() }
}

function Assert-AmeModelDownloadUri {
    param([Uri]$Uri)
    if ($Uri.Scheme -cne 'https' -or $Uri.Port -ne 443 -or $Uri.UserInfo -ne '' -or
        ($Uri.Host -cne 'huggingface.co' -and !$Uri.Host.EndsWith('.huggingface.co') -and !$Uri.Host.EndsWith('.hf.co'))) {
        throw 'Model download destination is outside the admitted hosts'
    }
}

function Receive-AmeModelArtifact {
    param([object]$Manifest, [object]$Artifact, [string]$Destination)
    Add-Type -AssemblyName System.Net.Http
    $handler = [Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $false
    $client = [Net.Http.HttpClient]::new($handler)
    $client.Timeout = [Threading.Timeout]::InfiniteTimeSpan
    $client.DefaultRequestHeaders.UserAgent.ParseAdd('Cedarflake-Ame-build/1.0')
    $client.DefaultRequestHeaders.AcceptEncoding.ParseAdd('identity')
    $deadline = [Threading.CancellationTokenSource]::new([TimeSpan]::FromHours(1))
    $uri = [Uri]::new("$($Manifest.baseUrl)/$($Artifact.remote)")
    try {
        for ($redirect = 0; $redirect -le 5; $redirect++) {
            Assert-AmeModelDownloadUri $uri
            $response = $client.GetAsync($uri, [Net.Http.HttpCompletionOption]::ResponseHeadersRead, $deadline.Token).GetAwaiter().GetResult()
            try {
                if ([int]$response.StatusCode -in @(301, 302, 303, 307, 308)) {
                    if ($redirect -eq 5 -or $null -eq $response.Headers.Location) { throw 'Model download redirect was rejected' }
                    $uri = [Uri]::new($uri, $response.Headers.Location)
                    continue
                }
                if ([int]$response.StatusCode -ne 200 -or
                    ($null -ne $response.Content.Headers.ContentLength -and $response.Content.Headers.ContentLength -ne $Artifact.length) -or
                    @($response.Content.Headers.ContentEncoding | Where-Object { $_ -cne 'identity' }).Count -gt 0) {
                    throw 'Model download response differs from the admitted artifact'
                }
                $source = $response.Content.ReadAsStreamAsync($deadline.Token).GetAwaiter().GetResult()
                try { Copy-AmeModelArtifactStream $source $Destination $Artifact.length $deadline.Token } finally { $source.Dispose() }
                return
            } finally { $response.Dispose() }
        }
    } finally { $deadline.Dispose(); $client.Dispose(); $handler.Dispose() }
}

function New-AmeClassificationModelsDirectory {
    param([string]$OutputDirectory, [string]$SourceDirectory, [object]$Manifest)
    Assert-AmeClassificationModelsManifest $Manifest
    $output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
    $build = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $PSScriptRoot) 'build'))
    if (!$output.StartsWith("$build\", [StringComparison]::OrdinalIgnoreCase)) { throw 'Model package storage must be below repository build' }
    Assert-AmeClassificationRuntimeLocalPath $output
    if (Test-Path -LiteralPath $output) { Assert-AmeClassificationModelsDirectory $output $Manifest; return }
    $parent = [IO.Path]::GetDirectoryName($output)
    [void][IO.Directory]::CreateDirectory($parent)
    $scratch = Join-Path $parent ('.classification-models-' + [Guid]::NewGuid().ToString('N'))
    [void][IO.Directory]::CreateDirectory($scratch)
    try {
        foreach ($artifact in $Manifest.files) {
            $destination = Join-Path $scratch $artifact.name
            if ([string]::IsNullOrWhiteSpace($SourceDirectory)) {
                Receive-AmeModelArtifact $Manifest $artifact $destination
            } else {
                $sourcePath = Join-Path $SourceDirectory $artifact.name
                Assert-AmeClassificationRuntimeArtifact $sourcePath $artifact
                $source = [IO.File]::Open($sourcePath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
                try { Copy-AmeModelArtifactStream $source $destination $artifact.length ([Threading.CancellationToken]::None) } finally { $source.Dispose() }
            }
            Assert-AmeClassificationRuntimeArtifact $destination $artifact
        }
        foreach ($notice in $Manifest.notices) {
            $source = Join-Path (Split-Path -Parent $PSScriptRoot) "rust/src/adapters/classification_runtime/installation/$($notice.name)"
            Assert-AmeClassificationRuntimeArtifact $source $notice
            [IO.File]::Copy($source, (Join-Path $scratch $notice.name))
        }
        Assert-AmeClassificationModelsDirectory $scratch $Manifest
        [IO.Directory]::Move($scratch, $output)
    } finally {
        if (Test-Path -LiteralPath $scratch) {
            Assert-AmeClassificationRuntimeLocalPath $scratch
            foreach ($file in @($Manifest.files) + @($Manifest.notices)) {
                $path = Join-Path $scratch $file.name
                if (Test-Path -LiteralPath $path) { Assert-AmeClassificationRuntimeLocalPath $path; Remove-Item -LiteralPath $path }
            }
            [IO.Directory]::Delete($scratch, $false)
        }
    }
}
