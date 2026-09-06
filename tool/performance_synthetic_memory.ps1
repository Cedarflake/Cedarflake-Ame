function Initialize-AmeSyntheticMemoryQuery {
  if ($null -ne ("AmeSyntheticMemoryQuery" -as [type])) { return }
  Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class AmeSyntheticMemoryQuery
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Counters
    {
        public uint Size, PageFaultCount;
        public UIntPtr PeakWorkingSet, WorkingSet, QuotaPeakPagedPool, QuotaPagedPool;
        public UIntPtr QuotaPeakNonPagedPool, QuotaNonPagedPool, Pagefile, PeakPagefile;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool K32GetProcessMemoryInfo(
        IntPtr process, out Counters counters, uint size);

    public static long ReadPeakWorkingSet(IntPtr retainedProcessHandle)
    {
        if (retainedProcessHandle == IntPtr.Zero || retainedProcessHandle == new IntPtr(-1))
            throw new ArgumentException("A retained child process handle is required");
        Counters counters;
        if (!K32GetProcessMemoryInfo(retainedProcessHandle, out counters,
                (uint)Marshal.SizeOf(typeof(Counters))))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Child memory query failed");
        return checked((long)counters.PeakWorkingSet.ToUInt64());
    }
}
'@
}
