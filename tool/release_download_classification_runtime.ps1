$ErrorActionPreference = "Stop"

function Receive-AmeClassificationRuntimeArchive {
    param(
        [Parameter(Mandatory = $true)][object]$Expected,
        [Parameter(Mandatory = $true)][string]$Download
    )
    Add-Type -AssemblyName System.Net.Http
    $client = [Net.Http.HttpClient]::new()
    $client.DefaultRequestHeaders.UserAgent.ParseAdd("Cedarflake-Ame-build/1.0")
    $timeout = [Threading.CancellationTokenSource]::new([TimeSpan]::FromMinutes(5))
    $response = $null
    try {
        $response = $client.GetAsync(
            $Expected.url, [Net.Http.HttpCompletionOption]::ResponseHeadersRead, $timeout.Token
        ).GetAwaiter().GetResult()
        [void]$response.EnsureSuccessStatusCode()
        if ($response.Content.Headers.ContentLength -gt $Expected.length) {
            throw "Runtime download exceeds its admitted size"
        }
        $source = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
        $destination = [IO.File]::Open($download, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
        try {
            $buffer = [byte[]]::new(65536)
            $total = 0L
            while (($read = $source.ReadAsync($buffer, 0, $buffer.Length, $timeout.Token).GetAwaiter().GetResult()) -gt 0) {
                $total += $read
                if ($total -gt $Expected.length) { throw "Runtime download exceeds its admitted size" }
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
}
