$ErrorActionPreference = 'Stop'
if ([Security.Principal.WindowsIdentity]::GetCurrent().Name.Split('\')[-1] -cne 'WDAGUtilityAccount') {
    throw 'Desktop admission is restricted to the disposable Windows Sandbox controller'
}

Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Text;

public static class AmeGuestDesktop {
    delegate bool WindowVisitor(IntPtr window, IntPtr context);
    [DllImport("user32.dll")]
    static extern bool EnumWindows(WindowVisitor visitor, IntPtr context);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetClassName(IntPtr window, StringBuilder text, int size);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetWindowText(IntPtr window, StringBuilder text, int size);
    [DllImport("user32.dll")]
    static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")]
    static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr OpenWindowStation(string name, bool inherit, uint access);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr OpenDesktop(string name, uint flags, bool inherit, uint access);
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool CloseWindowStation(IntPtr handle);
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool CloseDesktop(IntPtr handle);
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool GetUserObjectSecurity(IntPtr handle, ref uint information, byte[] value, uint size, out uint needed);
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool SetUserObjectSecurity(IntPtr handle, ref uint information, byte[] value);

    static RawSecurityDescriptor Read(IntPtr handle) {
        uint information = 4, needed;
        GetUserObjectSecurity(handle, ref information, null, 0, out needed);
        if (needed == 0 || needed > 65536) throw new Win32Exception(Marshal.GetLastWin32Error());
        byte[] bytes = new byte[needed];
        if (!GetUserObjectSecurity(handle, ref information, bytes, needed, out needed))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        return new RawSecurityDescriptor(bytes, 0);
    }

    public static string Inspect(bool desktop) {
        return Access(desktop, null);
    }

    public static string[] DescribeWindows() {
        List<string> windows = new List<string>();
        EnumWindows(delegate(IntPtr window, IntPtr context) {
            if (!IsWindowVisible(window)) return true;
            uint process;
            GetWindowThreadProcessId(window, out process);
            StringBuilder name = new StringBuilder(256), title = new StringBuilder(1024);
            GetClassName(window, name, name.Capacity);
            GetWindowText(window, title, title.Capacity);
            windows.Add(process + "|" + name + "|" + title);
            return true;
        }, IntPtr.Zero);
        return windows.ToArray();
    }

    public static string Admit(bool desktop, string sid) {
        return Access(desktop, new SecurityIdentifier(sid));
    }

    static string Access(bool desktop, SecurityIdentifier sid) {
        uint access = sid == null ? 0x20000u : 0x60000u;
        IntPtr handle = desktop ? OpenDesktop("Default", 0, false, access)
                                : OpenWindowStation("WinSta0", false, access);
        if (handle == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        try {
            RawSecurityDescriptor descriptor = Read(handle);
            if (sid != null) {
                int required = desktop ? 0xF01FF : 0xF037F;
                bool present = false;
                foreach (GenericAce entry in descriptor.DiscretionaryAcl) {
                    CommonAce ace = entry as CommonAce;
                    if (ace != null && ace.AceQualifier == AceQualifier.AccessAllowed &&
                        ace.SecurityIdentifier == sid && (ace.AccessMask & required) == required) present = true;
                }
                if (!present) {
                    descriptor.DiscretionaryAcl.InsertAce(descriptor.DiscretionaryAcl.Count,
                        new CommonAce(AceFlags.None, AceQualifier.AccessAllowed, required, sid, false, null));
                    byte[] bytes = new byte[descriptor.BinaryLength];
                    descriptor.GetBinaryForm(bytes, 0);
                    uint information = 4;
                    if (!SetUserObjectSecurity(handle, ref information, bytes))
                        throw new Win32Exception(Marshal.GetLastWin32Error());
                }
            }
            return Read(handle).GetSddlForm(AccessControlSections.Access);
        } finally {
            if (desktop) CloseDesktop(handle); else CloseWindowStation(handle);
        }
    }
}
'@
