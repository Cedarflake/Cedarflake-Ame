$ErrorActionPreference = "Stop"

function Receive-AmeClassificationRuntimeArchive {
    param(
        [Parameter(Mandatory = $true)][object]$Manifest,
        [Parameter(Mandatory = $true)][string]$Download
    )
    Add-Type -AssemblyName System.Net.Http
    $client = [Net.Http.HttpClient]::new()
    $client.DefaultRequestHeaders.UserAgent.ParseAdd("Cedarflake-Ame-build/1.0")
    $timeout = [Threading.CancellationTokenSource]::new([TimeSpan]::FromMinutes(5))
    $response = $null
    try {
        $archives = @($Manifest.archive)
        if ($Manifest.PSObject.Properties['bundledArchive']) { $archives += $Manifest.bundledArchive }
        foreach ($expected in $archives) {
            if ($null -ne $response) { $response.Dispose() }
            $response = $client.GetAsync(
                $expected.url, [Net.Http.HttpCompletionOption]::ResponseHeadersRead, $timeout.Token
            ).GetAwaiter().GetResult()
            if ([int]$response.StatusCode -notin @(404, 410)) { break }
        }
        [void]$response.EnsureSuccessStatusCode()
        if ($response.Content.Headers.ContentLength -gt $expected.length) {
            throw "Runtime download exceeds its admitted size"
        }
        $source = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
        $destination = [IO.File]::Open($download, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
        try {
            $buffer = [byte[]]::new(65536)
            $total = 0L
            while (($read = $source.ReadAsync($buffer, 0, $buffer.Length, $timeout.Token).GetAwaiter().GetResult()) -gt 0) {
                $total += $read
                if ($total -gt $expected.length) { throw "Runtime download exceeds its admitted size" }
                $destination.Write($buffer, 0, $read)
            }
        } finally {
            $destination.Dispose()
            $source.Dispose()
        }
        return $expected
    } finally {
        if ($null -ne $response) { $response.Dispose() }
        $timeout.Dispose()
        $client.Dispose()
    }
}
