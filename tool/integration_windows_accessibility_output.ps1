function Read-AmeWindowsAccessibilityOutput {
    param([string]$Path, [TimeSpan]$Timeout)

    if (-not [IO.File]::Exists($Path)) { return "" }
    $clock = [Diagnostics.Stopwatch]::StartNew()
    while ($true) {
        $stream = $null
        try {
            $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
        } catch [IO.IOException] {
            $nativeError = $_.Exception.HResult -band 0xFFFF
            if ($nativeError -notin @(32, 33)) { throw }
            $remaining = $Timeout - $clock.Elapsed
            if ($remaining -le [TimeSpan]::Zero) {
                throw [TimeoutException]::new(
                    "Windows accessibility output remained in use after owned-process shutdown", $_.Exception
                )
            }
            Start-Sleep -Milliseconds ([int][Math]::Min(25, [Math]::Max(1, $remaining.TotalMilliseconds)))
            continue
        }
        try {
            $reader = [IO.StreamReader]::new($stream, [Text.Encoding]::UTF8, $true)
            try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
        } finally {
            $stream.Dispose()
        }
    }
}
