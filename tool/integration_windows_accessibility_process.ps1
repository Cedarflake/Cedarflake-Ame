. (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1")

function Initialize-AmeWindowsAccessibilityProcessJob {
    if ($null -eq ("AmeWindowsAccessibilityProcessJob" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public sealed class AmeWindowsAccessibilityProcessJob : IDisposable
{
    private const uint JobObjectExtendedLimitInformation = 9;
    private const uint JobObjectLimitKillOnJobClose = 0x00002000;
    private const uint CreateSuspended = 0x00000004;
    private const uint CreateNoWindow = 0x08000000;

    private SafeFileHandle jobHandle;
    private SafeFileHandle processHandle;

    public AmeWindowsAccessibilityProcessJob()
    {
        IntPtr rawHandle = CreateJobObject(IntPtr.Zero, null);
        if (rawHandle == IntPtr.Zero || rawHandle == new IntPtr(-1))
        {
            throw new Win32Exception(
                Marshal.GetLastWin32Error(),
                "Could not create the Windows accessibility process job");
        }
        jobHandle = new SafeFileHandle(rawHandle, true);
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits =
            new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
        limits.BasicLimitInformation.LimitFlags = JobObjectLimitKillOnJobClose;
        int length = Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
        IntPtr buffer = Marshal.AllocHGlobal(length);
        try
        {
            Marshal.StructureToPtr(limits, buffer, false);
            if (!SetInformationJobObject(
                rawHandle,
                JobObjectExtendedLimitInformation,
                buffer,
                (uint)length))
            {
                throw new Win32Exception(
                    Marshal.GetLastWin32Error(),
                    "Could not configure the Windows accessibility process job");
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
            throw new ObjectDisposedException("AmeWindowsAccessibilityProcessJob");
        }
        if (processHandle != null)
        {
            throw new InvalidOperationException(
                "The Windows accessibility process job already owns a process");
        }
        if (fileName.IndexOf('"') >= 0)
        {
            throw new ArgumentException(
                "The process filename must not contain a quote",
                "fileName");
        }
        STARTUPINFO startup = new STARTUPINFO();
        startup.cb = (uint)Marshal.SizeOf(typeof(STARTUPINFO));
        PROCESS_INFORMATION process;
        StringBuilder commandLine =
            new StringBuilder("\"" + fileName + "\" " + arguments);
        if (!CreateProcess(
            fileName,
            commandLine,
            IntPtr.Zero,
            IntPtr.Zero,
            false,
            CreateSuspended | CreateNoWindow,
            IntPtr.Zero,
            workingDirectory,
            ref startup,
            out process))
        {
            throw new Win32Exception(
                Marshal.GetLastWin32Error(),
                "Could not start the Windows accessibility integration process");
        }
        try
        {
            if (!AssignProcessToJobObject(jobHandle.DangerousGetHandle(), process.hProcess))
            {
                int error = Marshal.GetLastWin32Error();
                TerminateProcess(process.hProcess, 1);
                throw new Win32Exception(
                    error,
                    "Could not assign the Windows accessibility process tree to its job");
            }
            if (ResumeThread(process.hThread) == UInt32.MaxValue)
            {
                int error = Marshal.GetLastWin32Error();
                TerminateProcess(process.hProcess, 1);
                throw new Win32Exception(
                    error,
                    "Could not resume the Windows accessibility integration process");
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

    public int PrimaryExitCode
    {
        get
        {
            if (processHandle == null || processHandle.IsClosed)
            {
                throw new InvalidOperationException(
                    "The Windows accessibility process job has no owned process");
            }
            uint exitCode;
            if (!GetExitCodeProcess(processHandle.DangerousGetHandle(), out exitCode))
            {
                throw new Win32Exception(
                    Marshal.GetLastWin32Error(),
                    "Could not read the Windows accessibility process exit code");
            }
            if (exitCode == 259)
            {
                throw new InvalidOperationException(
                    "The Windows accessibility integration process is still active");
            }
            return unchecked((int)exitCode);
        }
    }

    public void Dispose()
    {
        SafeFileHandle ownedJob = jobHandle;
        SafeFileHandle ownedProcess = processHandle;
        jobHandle = null;
        processHandle = null;
        Exception jobFailure = null;
        try
        {
            if (ownedJob != null) { ownedJob.Dispose(); }
        }
        catch (Exception error)
        {
            jobFailure = error;
        }
        try
        {
            if (ownedProcess != null) { ownedProcess.Dispose(); }
        }
        catch (Exception error)
        {
            if (jobFailure != null)
            {
                throw new AggregateException(
                    "Windows accessibility handle cleanup failed", jobFailure, error);
            }
            throw;
        }
        if (jobFailure != null) { throw jobFailure; }
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
    private static extern IntPtr CreateJobObject(
        IntPtr securityAttributes,
        string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetInformationJobObject(
        IntPtr job,
        uint informationClass,
        IntPtr information,
        uint informationLength);

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
}
'@
    }
}

function Invoke-AmeWindowsAccessibilityProbe {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ProbeScriptPath,
        [Parameter(Mandatory = $true)]
        [int]$TargetProcessId,
        [Parameter(Mandatory = $true)]
        [string]$Phase,
        [Parameter(Mandatory = $true)]
        [string]$ResultPath,
        [Parameter(Mandatory = $true)]
        [string]$Token,
        [Parameter(Mandatory = $true)]
        [TimeSpan]$Timeout
    )

    if ($Timeout -le [TimeSpan]::Zero) {
        throw "Windows UIA probe has no remaining parent deadline"
    }
    foreach ($argument in @($ProbeScriptPath, $Phase, $ResultPath, $Token)) {
        if ($argument.Contains('"') -or $argument.IndexOf([char]0) -ge 0) {
            throw "Windows UIA probe argument cannot be quoted safely"
        }
    }
    if (
        (Test-Path -LiteralPath $ResultPath) -or
        (Test-Path -LiteralPath ([System.IO.Path]::ChangeExtension($ResultPath, "tmp")))
    ) {
        throw "Windows UIA probe result must use fresh scratch storage"
    }

    Initialize-AmeWindowsAccessibilityProcessJob
    $hostExecutable = (Get-Process -Id $PID).Path
    $arguments = (
        '-NoLogo -NoProfile -NonInteractive -Mta -ExecutionPolicy Bypass ' +
        '-File "{0}" -TargetProcessId {1} -Phase "{2}" -ResultPath "{3}" -Token "{4}"'
    ) -f $ProbeScriptPath, $TargetProcessId, $Phase, $ResultPath, $Token
    $job = $null
    $process = $null
    $probeProcessId = 0
    $exitCode = $null
    $probeFailure = $null
    $cleanup = New-AmeWindowsAccessibilityCleanup
    $elapsed = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $job = [AmeWindowsAccessibilityProcessJob]::new()
        $process = $job.Start(
            $hostExecutable,
            $arguments,
            (Split-Path -Parent $ProbeScriptPath)
        )
        $probeProcessId = $process.Id
        while (-not $process.HasExited) {
            $remaining = $Timeout - $elapsed.Elapsed
            if ($remaining -le [TimeSpan]::Zero) {
                throw "Windows UIA probe '$Phase' exceeded its parent deadline"
            }
            $waitMilliseconds = [int][Math]::Min(
                50,
                [Math]::Max(1, [Math]::Ceiling($remaining.TotalMilliseconds))
            )
            $null = $process.WaitForExit($waitMilliseconds)
        }
        if ($elapsed.Elapsed -gt $Timeout) {
            throw "Windows UIA probe '$Phase' exceeded its parent deadline"
        }
        $exitCode = $job.PrimaryExitCode
    } catch {
        $probeFailure = $_.Exception
    } finally {
        Close-AmeWindowsAccessibilityProcess -Cleanup $cleanup -Job $job -Process $process
        $elapsed.Stop()
    }
    Complete-AmeWindowsUiaProbe `
        -ResultPath $ResultPath -Token $Token -Phase $Phase `
        -TargetProcessId $TargetProcessId -ProbeProcessId $probeProcessId `
        -ExitCode $exitCode -ProbeFailure $probeFailure -Cleanup $cleanup
}
