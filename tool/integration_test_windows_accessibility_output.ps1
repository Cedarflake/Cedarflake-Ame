$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_output.ps1")

if ($null -eq ("AmeAccessibilityOutputWriter" -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
public static class AmeAccessibilityOutputWriter
{
    public static Task CompleteLater(string path)
    {
        var stream = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.Read);
        return Task.Run(() => {
            using (stream) {
                Thread.Sleep(150);
                byte[] bytes = Encoding.UTF8.GetBytes("complete engine output");
                stream.Write(bytes, 0, bytes.Length);
            }
        });
    }
}
'@
}
$buildRoot = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $PSScriptRoot) "build"))
$path = [IO.Path]::GetFullPath((Join-Path $buildRoot ("uia-output-" + [guid]::NewGuid().ToString("N") + ".log")))
if (-not $path.StartsWith($buildRoot + "\", [StringComparison]::OrdinalIgnoreCase) -or
    ((Get-Item -LiteralPath $buildRoot).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Windows UIA output fixture escaped owned storage"
}
$writer = $null
$lockStream = $null
try {
    if ((Read-AmeWindowsAccessibilityOutput -Path $path -Timeout ([TimeSpan]::Zero)) -cne "") {
        throw "A missing UIA log must stay empty"
    }
    $writer = [AmeAccessibilityOutputWriter]::CompleteLater($path)
    $output = Read-AmeWindowsAccessibilityOutput -Path $path -Timeout ([TimeSpan]::FromSeconds(5))
    $writer.GetAwaiter().GetResult()
    if ($output -cne "complete engine output") { throw "UIA log capture read before its writer released the file" }

    $lockStream = [IO.File]::Open($path, [IO.FileMode]::Open, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    $failure = $null
    try { Read-AmeWindowsAccessibilityOutput -Path $path -Timeout ([TimeSpan]::FromMilliseconds(100)) | Out-Null } catch {
        $failure = $_.Exception
    }
    if ($failure -isnot [TimeoutException]) { throw "A locked UIA log did not preserve the cleanup deadline: $failure" }
    $lockStream.Dispose()
    $lockStream = $null
    if ([IO.File]::ReadAllText($path) -cne "complete engine output") { throw "A failed output capture changed its evidence" }
    Write-Host "Windows UIA output passed: completed writer, bounded sharing wait and unchanged failure evidence"
} finally {
    if ($null -ne $lockStream) { $lockStream.Dispose() }
    if ($null -ne $writer) { $writer.GetAwaiter().GetResult() }
    if ([IO.File]::Exists($path)) { Remove-Item -LiteralPath $path -Force }
}
