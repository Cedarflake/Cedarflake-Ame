[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$LocalRoot,
    [Parameter(Mandatory = $true)]
    [string]$CloudRoot,
    [Parameter(Mandatory = $true)]
    [string]$SourceCatalogPath,
    [Parameter(Mandatory = $true)]
    [string]$StorageRoot,
    [Parameter(Mandatory = $true)]
    [string]$AuthorizationToken,
    [switch]$AcknowledgeCloudReadOnly,
    [ValidateRange(60, 3600)]
    [int]$TimeLimitSeconds = 1800,
    [ValidateRange(268435456, 4294967296)]
    [UInt64]$MemoryLimitBytes = 2147483648,
    [switch]$ValidationOnly,
    [ValidateSet("Historical", "Replacement")]
    [string]$AcceptanceProfile = "Historical"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

if ($env:OS -ne "Windows_NT") {
    throw "R2c reliability acceptance requires Windows"
}

if ($null -eq ("AmeR2cProcessJob" -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public sealed class AmeR2cProcessJob : IDisposable
{
    private const uint JobObjectExtendedLimitInformation = 9;
    private const uint JobObjectLimitKillOnJobClose = 0x00002000;
    private const uint CreateSuspended = 0x00000004;
    private const uint FileFlagBackupSemantics = 0x02000000;
    private const uint OpenExisting = 3;
    private const uint ShareReadWriteDelete = 0x00000007;

    private SafeFileHandle jobHandle;
    private SafeFileHandle processHandle;

    public AmeR2cProcessJob()
    {
        IntPtr rawHandle = CreateJobObject(IntPtr.Zero, null);
        if (rawHandle == IntPtr.Zero || rawHandle == new IntPtr(-1))
        {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not create the R2c process job");
        }
        jobHandle = new SafeFileHandle(rawHandle, true);
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
        limits.BasicLimitInformation.LimitFlags = JobObjectLimitKillOnJobClose;
        int length = Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
        IntPtr buffer = Marshal.AllocHGlobal(length);
        try
        {
            Marshal.StructureToPtr(limits, buffer, false);
            if (!SetInformationJobObject(rawHandle, JobObjectExtendedLimitInformation, buffer, (uint)length))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not configure the R2c process job");
            }
        }
        catch
        {
            jobHandle.Dispose();
            throw;
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    public Process Start(string fileName, string arguments, string workingDirectory)
    {
        if (jobHandle == null || jobHandle.IsClosed)
        {
            throw new ObjectDisposedException("AmeR2cProcessJob");
        }
        if (fileName.IndexOf('"') >= 0)
        {
            throw new ArgumentException("The process filename must not contain a quote", "fileName");
        }
        if (processHandle != null)
        {
            throw new InvalidOperationException("The R2c process job already owns a process");
        }
        STARTUPINFO startup = new STARTUPINFO();
        startup.cb = (uint)Marshal.SizeOf(typeof(STARTUPINFO));
        PROCESS_INFORMATION process;
        StringBuilder commandLine = new StringBuilder("\"" + fileName + "\" " + arguments);
        if (!CreateProcess(
            fileName,
            commandLine,
            IntPtr.Zero,
            IntPtr.Zero,
            false,
            CreateSuspended,
            IntPtr.Zero,
            workingDirectory,
            ref startup,
            out process))
        {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not start the R2c acceptance process");
        }
        try
        {
            if (!AssignProcessToJobObject(jobHandle.DangerousGetHandle(), process.hProcess))
            {
                int error = Marshal.GetLastWin32Error();
                TerminateProcess(process.hProcess, 1);
                throw new Win32Exception(error, "Could not assign the R2c process to its job");
            }
            if (ResumeThread(process.hThread) == UInt32.MaxValue)
            {
                int error = Marshal.GetLastWin32Error();
                TerminateProcess(process.hProcess, 1);
                throw new Win32Exception(error, "Could not resume the R2c acceptance process");
            }
            processHandle = new SafeFileHandle(process.hProcess, true);
            process.hProcess = IntPtr.Zero;
            return Process.GetProcessById((int)process.dwProcessId);
        }
        finally
        {
            CloseHandle(process.hThread);
            if (process.hProcess != IntPtr.Zero)
            {
                CloseHandle(process.hProcess);
            }
        }
    }

    public uint PrimaryExitCode
    {
        get
        {
            if (processHandle == null || processHandle.IsClosed)
            {
                throw new InvalidOperationException("The R2c process job has no owned process");
            }
            uint exitCode;
            if (!GetExitCodeProcess(processHandle.DangerousGetHandle(), out exitCode))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the R2c process exit code");
            }
            if (exitCode == 259)
            {
                throw new InvalidOperationException("The R2c acceptance process is still active");
            }
            return exitCode;
        }
    }

    public ulong PeakMemoryBytes
    {
        get
        {
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION information;
            uint returnedLength;
            int length = Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
            if (!QueryInformationJobObject(
                jobHandle.DangerousGetHandle(),
                JobObjectExtendedLimitInformation,
                out information,
                (uint)length,
                out returnedLength))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not query R2c job memory");
            }
            return information.PeakJobMemoryUsed.ToUInt64();
        }
    }

    public static string ResourceLimitFailure(
        ulong observedMemoryBytes,
        ulong memoryLimitBytes,
        DateTime observedAtUtc,
        DateTime deadlineUtc)
    {
        if (observedMemoryBytes > memoryLimitBytes)
        {
            return "R2c reliability acceptance exceeded the memory limit of " +
                memoryLimitBytes + " bytes";
        }
        if (observedAtUtc >= deadlineUtc)
        {
            return "R2c reliability acceptance exceeded its time limit";
        }
        return null;
    }

    public static string ResolveExistingPath(string path)
    {
        using (SafeFileHandle handle = CreateFile(
            path,
            0,
            ShareReadWriteDelete,
            IntPtr.Zero,
            OpenExisting,
            FileFlagBackupSemantics,
            IntPtr.Zero))
        {
            if (handle.IsInvalid)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not resolve the R2c path");
            }
            StringBuilder buffer = new StringBuilder(32768);
            uint length = GetFinalPathNameByHandle(handle, buffer, (uint)buffer.Capacity, 0);
            if (length == 0 || length >= buffer.Capacity)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the resolved R2c path");
            }
            string resolved = buffer.ToString();
            if (resolved.StartsWith("\\\\?\\UNC\\", StringComparison.OrdinalIgnoreCase))
            {
                return "\\\\" + resolved.Substring(8);
            }
            if (resolved.StartsWith("\\\\?\\", StringComparison.OrdinalIgnoreCase))
            {
                return resolved.Substring(4);
            }
            return resolved;
        }
    }

    public void Dispose()
    {
        if (jobHandle != null)
        {
            jobHandle.Dispose();
            jobHandle = null;
        }
        if (processHandle != null)
        {
            processHandle.Dispose();
            processHandle = null;
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_BASIC_LIMIT_INFORMATION
    {
        public long PerProcessUserTimeLimit;
        public long PerJobUserTimeLimit;
        public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize;
        public UIntPtr MaximumWorkingSetSize;
        public uint ActiveProcessLimit;
        public UIntPtr Affinity;
        public uint PriorityClass;
        public uint SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct IO_COUNTERS
    {
        public ulong ReadOperationCount;
        public ulong WriteOperationCount;
        public ulong OtherOperationCount;
        public ulong ReadTransferCount;
        public ulong WriteTransferCount;
        public ulong OtherTransferCount;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION
    {
        public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation;
        public IO_COUNTERS IoInfo;
        public UIntPtr ProcessMemoryLimit;
        public UIntPtr JobMemoryLimit;
        public UIntPtr PeakProcessMemoryUsed;
        public UIntPtr PeakJobMemoryUsed;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct STARTUPINFO
    {
        public uint cb;
        public string lpReserved;
        public string lpDesktop;
        public string lpTitle;
        public uint dwX;
        public uint dwY;
        public uint dwXSize;
        public uint dwYSize;
        public uint dwXCountChars;
        public uint dwYCountChars;
        public uint dwFillAttribute;
        public uint dwFlags;
        public ushort wShowWindow;
        public ushort cbReserved2;
        public IntPtr lpReserved2;
        public IntPtr hStdInput;
        public IntPtr hStdOutput;
        public IntPtr hStdError;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PROCESS_INFORMATION
    {
        public IntPtr hProcess;
        public IntPtr hThread;
        public uint dwProcessId;
        public uint dwThreadId;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateJobObject(IntPtr securityAttributes, string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetInformationJobObject(
        IntPtr job,
        uint informationClass,
        IntPtr information,
        uint informationLength);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool QueryInformationJobObject(
        IntPtr job,
        uint informationClass,
        out JOBOBJECT_EXTENDED_LIMIT_INFORMATION information,
        uint informationLength,
        out uint returnedLength);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CreateProcess(
        string applicationName,
        StringBuilder commandLine,
        IntPtr processAttributes,
        IntPtr threadAttributes,
        bool inheritHandles,
        uint creationFlags,
        IntPtr environment,
        string currentDirectory,
        ref STARTUPINFO startupInformation,
        out PROCESS_INFORMATION processInformation);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint ResumeThread(IntPtr thread);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool TerminateProcess(IntPtr process, uint exitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr handle);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFile(
        string fileName,
        uint desiredAccess,
        uint shareMode,
        IntPtr securityAttributes,
        uint creationDisposition,
        uint flagsAndAttributes,
        IntPtr templateFile);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint GetFinalPathNameByHandle(
        SafeFileHandle file,
        StringBuilder filePath,
        uint filePathLength,
        uint flags);
}
'@
}

$isReplacement = $AcceptanceProfile -ceq "Replacement"
$acceptanceLabel = if ($isReplacement) { "R2c-M replacement" } else { "R2c-H reliability" }
$requiredToken = if ($isReplacement) {
    "CEDARFLAKE_AME_R2C_REPLACEMENT_ACCEPTANCE_V1"
} else {
    "CEDARFLAKE_AME_R2C_RELIABILITY_ACCEPTANCE_V1"
}
if ($AuthorizationToken -cne $requiredToken) {
    throw "The exact current $acceptanceLabel authorization token is required"
}
if (-not $AcknowledgeCloudReadOnly) {
    throw "The cloud root requires an explicit read-only acknowledgement"
}

$absoluteLocalRoot = [System.IO.Path]::GetFullPath($LocalRoot)
$absoluteCloudRoot = [System.IO.Path]::GetFullPath($CloudRoot)
$absoluteCatalog = [System.IO.Path]::GetFullPath($SourceCatalogPath)
$absoluteStorage = [System.IO.Path]::GetFullPath($StorageRoot)
if (-not (Test-Path -LiteralPath $absoluteLocalRoot -PathType Container)) {
    throw "The local-primary acceptance root is not an available directory"
}
if (-not (Test-Path -LiteralPath $absoluteCloudRoot -PathType Container)) {
    throw "The cloud-primary acceptance root is not an available directory"
}
if (-not (Test-Path -LiteralPath $absoluteCatalog -PathType Leaf)) {
    throw "The retained source catalog is not an available file"
}
if (-not (Test-Path -LiteralPath $absoluteStorage -PathType Container)) {
    throw "$acceptanceLabel acceptance storage must be a pre-created empty directory"
}

$resolvedLocalRoot = [AmeR2cProcessJob]::ResolveExistingPath($absoluteLocalRoot)
$resolvedCloudRoot = [AmeR2cProcessJob]::ResolveExistingPath($absoluteCloudRoot)
$resolvedCatalog = [AmeR2cProcessJob]::ResolveExistingPath($absoluteCatalog)
$resolvedStorage = [AmeR2cProcessJob]::ResolveExistingPath($absoluteStorage)

function ConvertTo-NormalizedAmePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    return $Path.Replace("/", "\").TrimEnd("\").ToLowerInvariant()
}

function Test-AmePathOverlap {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Left,
        [Parameter(Mandatory = $true)]
        [string]$Right
    )

    $normalizedLeft = ConvertTo-NormalizedAmePath $Left
    $normalizedRight = ConvertTo-NormalizedAmePath $Right
    return (
        $normalizedLeft -eq $normalizedRight -or
        $normalizedLeft.StartsWith("$normalizedRight\") -or
        $normalizedRight.StartsWith("$normalizedLeft\")
    )
}

if (Test-AmePathOverlap -Left $resolvedLocalRoot -Right $resolvedCloudRoot) {
    throw "The two authorized logical roots must not overlap"
}
foreach ($sourcePath in @($resolvedLocalRoot, $resolvedCloudRoot, $resolvedCatalog)) {
    if (Test-AmePathOverlap -Left $sourcePath -Right $resolvedStorage) {
        throw "$acceptanceLabel isolated storage must remain outside every source path"
    }
}
foreach ($rootPath in @($resolvedLocalRoot, $resolvedCloudRoot)) {
    if (Test-AmePathOverlap -Left $rootPath -Right $resolvedCatalog) {
        throw "The retained catalog must remain outside both source roots"
    }
}
$existingContent = Get-ChildItem -LiteralPath $resolvedStorage -Force |
    Select-Object -First 1
if ($null -ne $existingContent) {
    throw "$acceptanceLabel acceptance storage must be empty"
}

if ($ValidationOnly) {
    $validationPrefix = if ($isReplacement) { "AME_R2C_M" } else { "AME_R2C_H" }
    Write-Output "$validationPrefix`_VALIDATION status=passed"
    exit 0
}

$repositoryRoot = Get-AmeRepositoryRoot
$cargo = (Get-AmeToolchain).Cargo
$reportPrefix = if ($isReplacement) { "AME_R2C_M" } else { "AME_R2C_H" }
$reportName = if ($isReplacement) {
    "r2c-m-replacement-reliability.log"
} else {
    "r2c-h-large-library-reliability.log"
}
$reportPath = Join-Path $resolvedStorage $reportName
if ($isReplacement) {
    $environment = @{
        CEDARFLAKE_AME_R2C_M_CONSENT = $requiredToken
        CEDARFLAKE_AME_R2C_M_CLOUD_READ_ONLY_ACK = "true"
        CEDARFLAKE_AME_R2C_M_LOCAL_ROOT = $resolvedLocalRoot
        CEDARFLAKE_AME_R2C_M_CLOUD_ROOT = $resolvedCloudRoot
        CEDARFLAKE_AME_R2C_M_SOURCE_CATALOG = $resolvedCatalog
        CEDARFLAKE_AME_R2C_M_STORAGE_ROOT = $resolvedStorage
        CEDARFLAKE_AME_R2C_M_REPORT = $reportPath
    }
} else {
    $environment = @{
        CEDARFLAKE_AME_R2C_H_CONSENT = $requiredToken
        CEDARFLAKE_AME_R2C_H_CLOUD_READ_ONLY_ACK = "true"
        CEDARFLAKE_AME_R2C_H_LOCAL_ROOT = $resolvedLocalRoot
        CEDARFLAKE_AME_R2C_H_CLOUD_ROOT = $resolvedCloudRoot
        CEDARFLAKE_AME_R2C_H_SOURCE_CATALOG = $resolvedCatalog
        CEDARFLAKE_AME_R2C_H_STORAGE_ROOT = $resolvedStorage
        CEDARFLAKE_AME_R2C_H_REPORT = $reportPath
    }
}
$previousEnvironment = @{}
foreach ($name in $environment.Keys) {
    $previousEnvironment[$name] = [System.Environment]::GetEnvironmentVariable($name, "Process")
    [System.Environment]::SetEnvironmentVariable($name, $environment[$name], "Process")
}

$testFilter = if ($isReplacement) { "r2c_m_" } else { "r2c_h_" }
$ignoredMode = if ($isReplacement) { "--include-ignored" } else { "--ignored" }
$processArguments = (
    "test --release --locked --manifest-path rust\Cargo.toml $testFilter " +
    "-- $ignoredMode --nocapture --test-threads=1"
)
$deadlineUtc = [DateTime]::UtcNow.AddSeconds($TimeLimitSeconds)
$peakJobMemoryBytes = [UInt64]0
$failure = $null
$process = $null
$processJob = $null
$processExitCode = $null
$toolLock = Enter-AmeRepositoryToolLock
try {
    $processJob = [AmeR2cProcessJob]::new()
    $process = $processJob.Start($cargo, $processArguments, $repositoryRoot)
    while (-not $process.HasExited) {
        $observedJobMemoryBytes = [UInt64]$processJob.PeakMemoryBytes
        if ($observedJobMemoryBytes -gt $peakJobMemoryBytes) {
            $peakJobMemoryBytes = $observedJobMemoryBytes
        }
        $failure = [AmeR2cProcessJob]::ResourceLimitFailure(
            $observedJobMemoryBytes,
            $MemoryLimitBytes,
            [DateTime]::UtcNow,
            $deadlineUtc
        )
        if ($null -ne $failure) {
            $processJob.Dispose()
            $processJob = $null
            $process.WaitForExit()
            break
        }
        Start-Sleep -Milliseconds 20
    }
    if (-not $process.HasExited) {
        $process.WaitForExit()
    }
    if ($null -eq $failure) {
        $peakJobMemoryBytes = [UInt64]$processJob.PeakMemoryBytes
        $failure = [AmeR2cProcessJob]::ResourceLimitFailure(
            $peakJobMemoryBytes,
            $MemoryLimitBytes,
            [DateTime]::UtcNow,
            $deadlineUtc
        )
        if ($null -eq $failure) {
            $processExitCode = [int]$processJob.PrimaryExitCode
        }
    }
} finally {
    if ($null -ne $processJob) {
        $processJob.Dispose()
        $processJob = $null
    }
    if ($null -ne $process -and -not $process.HasExited) {
        $process.WaitForExit()
    }
    foreach ($name in $environment.Keys) {
        [System.Environment]::SetEnvironmentVariable(
            $name,
            $previousEnvironment[$name],
            "Process"
        )
    }
    Exit-AmeRepositoryToolLock $toolLock
}

if ($null -ne $failure) {
    throw $failure
}
if ($processExitCode -ne 0) {
    throw "$acceptanceLabel acceptance failed with exit code $processExitCode"
}
if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) {
    throw "$acceptanceLabel acceptance completed without a report"
}

$memoryLine = (
    "$reportPrefix`_MEMORY peak_job_memory_bytes=$peakJobMemoryBytes " +
    "limit_bytes=$MemoryLimitBytes"
)
[System.IO.File]::AppendAllText(
    $reportPath,
    "$memoryLine$([Environment]::NewLine)",
    [System.Text.UTF8Encoding]::new($false)
)
$report = [System.IO.File]::ReadAllText($reportPath)
Write-Output $report.TrimEnd()
Write-Output "$reportPrefix`_REPORT status=available"
