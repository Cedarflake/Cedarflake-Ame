function Initialize-AmeWindowsAccessibilityActivation {
    if ("AmeWindowsAccessibilityActivation" -as [type]) {
        return
    }

    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

public sealed class AmeWindowsAccessibilityActivationTargets
{
    public int WindowCount { get; private set; }
    public IntPtr[] ViewHandles { get; private set; }

    internal AmeWindowsAccessibilityActivationTargets(
        int windowCount,
        IntPtr[] viewHandles)
    {
        WindowCount = windowCount;
        ViewHandles = viewHandles;
    }
}

public static class AmeWindowsAccessibilityActivation
{
    private const string FlutterViewClass = "FLUTTERVIEW";
    private const uint ObjectIdClient = unchecked((uint)-4);
    private static readonly Guid AccessibleInterfaceId =
        new Guid("618736E0-3C3D-11CF-810C-00AA00389B71");

    public static AmeWindowsAccessibilityActivationTargets FindTargets(
        int targetProcessId)
    {
        if (targetProcessId <= 0)
        {
            throw new ArgumentOutOfRangeException("targetProcessId");
        }

        int windowCount = 0;
        var handles = new HashSet<IntPtr>();
        EnumWindowCallback childCallback = delegate(IntPtr child, IntPtr state)
        {
            if (IsTargetView(child, targetProcessId))
            {
                handles.Add(child);
            }
            return true;
        };
        EnumWindowCallback windowCallback = delegate(IntPtr window, IntPtr state)
        {
            uint processId;
            if (GetWindowThreadProcessId(window, out processId) != 0 &&
                processId == (uint)targetProcessId)
            {
                windowCount++;
                EnumChildWindows(window, childCallback, IntPtr.Zero);
            }
            return true;
        };
        if (!EnumWindows(windowCallback, IntPtr.Zero))
        {
            throw new Win32Exception(Marshal.GetLastWin32Error(),
                "Windows accessibility activation could not enumerate windows");
        }
        var viewHandles = new IntPtr[handles.Count];
        handles.CopyTo(viewHandles);
        return new AmeWindowsAccessibilityActivationTargets(
            windowCount, viewHandles);
    }

    public static void ActivateView(int targetProcessId, IntPtr viewHandle)
    {
        if (targetProcessId <= 0 || !IsTargetView(viewHandle, targetProcessId))
        {
            throw new InvalidOperationException(
                "Windows accessibility activation target no longer belongs " +
                "to the expected Flutter process and window class");
        }

        IntPtr accessible = IntPtr.Zero;
        Guid interfaceId = AccessibleInterfaceId;
        try
        {
            int result = AccessibleObjectFromWindow(
                viewHandle, ObjectIdClient, ref interfaceId, out accessible);
            if (result != 0 || accessible == IntPtr.Zero)
            {
                throw new InvalidOperationException(
                    "Windows accessibility activation MSAA request failed " +
                    "with HRESULT 0x" + result.ToString("X8") +
                    (accessible == IntPtr.Zero ? " and no interface" : ""));
            }
        }
        finally
        {
            if (accessible != IntPtr.Zero)
            {
                Marshal.Release(accessible);
            }
        }
    }

    private static bool IsTargetView(IntPtr window, int targetProcessId)
    {
        uint processId;
        if (window == IntPtr.Zero ||
            GetWindowThreadProcessId(window, out processId) == 0 ||
            processId != (uint)targetProcessId)
        {
            return false;
        }
        var windowClass = new StringBuilder(256);
        return GetClassName(window, windowClass, windowClass.Capacity) != 0 &&
            String.Equals(windowClass.ToString(), FlutterViewClass,
                StringComparison.Ordinal);
    }

    private delegate bool EnumWindowCallback(IntPtr window, IntPtr state);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(
        EnumWindowCallback callback,
        IntPtr state);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumChildWindows(
        IntPtr parent,
        EnumWindowCallback callback,
        IntPtr state);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint GetWindowThreadProcessId(
        IntPtr window,
        out uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr window,
        StringBuilder className,
        int maximumCount);

    [DllImport("oleacc.dll")]
    private static extern int AccessibleObjectFromWindow(
        IntPtr window,
        uint objectId,
        ref Guid interfaceId,
        out IntPtr accessible);
}
'@
}

function Get-AmeWindowsAccessibilityActivationTargets {
    param(
        [Parameter(Mandatory = $true)]
        [int]$TargetProcessId
    )

    Initialize-AmeWindowsAccessibilityActivation
    return [AmeWindowsAccessibilityActivation]::FindTargets($TargetProcessId)
}

function Enable-AmeWindowsAccessibilityNativeView {
    param(
        [Parameter(Mandatory = $true)]
        [int]$TargetProcessId,
        [Parameter(Mandatory = $true)]
        [IntPtr]$NativeViewHandle
    )

    [AmeWindowsAccessibilityActivation]::ActivateView(
        $TargetProcessId,
        $NativeViewHandle
    )
}
