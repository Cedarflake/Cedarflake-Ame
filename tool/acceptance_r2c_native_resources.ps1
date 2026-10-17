function Initialize-AmeR2cRNativeTypes {
    param(
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [switch]$ForceCompilerFailure
    )
    $nativeType = "AmeR2cRNative" -as [type]
    $jobType = "AmeR2cRProcessJob" -as [type]
    $rootType = "AmeR2cRDisposableRoot" -as [type]
    $snapshotType = "AmeR2cRAuditedScriptSnapshot" -as [type]
    if ($null -ne $nativeType -and $null -ne $jobType -and $null -ne $rootType -and
        $null -ne $snapshotType) { return }
    if ($null -ne $nativeType -or $null -ne $jobType -or $null -ne $rootType -or
        $null -ne $snapshotType) {
        throw "R2c-R native types were only partially initialized"
    }

    $bootstrap = $null
    $previousTemp = $null
    $previousTmp = $null
    $environmentCaptured = $false
    $environmentMutated = $false
    try {
        $bootstrap = New-AmeR2cRCompilerBootstrap -RepositoryRoot $RepositoryRoot
        Invoke-AmeR2cRCompilerFaultForGuardrail `
            -Point "bootstrap-post-create-pre-initialization" `
            -Owner $bootstrap
        $previousTemp = [Environment]::GetEnvironmentVariable("TEMP", "Process")
        $previousTmp = [Environment]::GetEnvironmentVariable("TMP", "Process")
        $environmentCaptured = $true
        Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $bootstrap
        $environmentMutated = $true
        [Environment]::SetEnvironmentVariable("TEMP", $bootstrap.Path, "Process")
        [Environment]::SetEnvironmentVariable("TMP", $bootstrap.Path, "Process")
        $nativeTypeDefinition = @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using Microsoft.Win32.SafeHandles;

public static class AmeR2cRNative
{
    public static Version NativeVersion()
    {
        RTL_OSVERSIONINFOEX version = new RTL_OSVERSIONINFOEX();
        version.dwOSVersionInfoSize = (uint)Marshal.SizeOf(typeof(RTL_OSVERSIONINFOEX));
        int status = RtlGetVersion(ref version);
        if (status != 0) throw new InvalidOperationException("Could not query the native Windows version; NTSTATUS=0x" + status.ToString("X8"));
        return new Version((int)version.dwMajorVersion, (int)version.dwMinorVersion, (int)version.dwBuildNumber);
    }

    public static string ResolveExistingPath(string path)
    {
        using (SafeFileHandle handle = CreateFile(path, 0x00000080, 7, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero))
        {
            if (handle.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not resolve the R2c-R path");
            StringBuilder buffer = new StringBuilder(32768);
            uint length = GetFinalPathNameByHandle(handle, buffer, (uint)buffer.Capacity, 0);
            if (length == 0 || length >= buffer.Capacity) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the resolved R2c-R path");
            string resolved = buffer.ToString();
            if (resolved.StartsWith("\\\\?\\UNC\\", StringComparison.OrdinalIgnoreCase)) return "\\\\" + resolved.Substring(8);
            return resolved.StartsWith("\\\\?\\", StringComparison.OrdinalIgnoreCase) ? resolved.Substring(4) : resolved;
        }
    }

    public static uint ProductSku(uint major, uint minor)
    {
        uint product;
        if (!GetProductInfo(major, minor, 0, 0, out product)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not query the Windows product SKU");
        return product;
    }

    public static void AssertDirectoryAnchor(string path)
    {
        AmeR2cRDisposableRoot.AssertDirectoryAnchor(path);
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFile(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, uint creationDisposition, uint flagsAndAttributes, IntPtr templateFile);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint GetFinalPathNameByHandle(SafeFileHandle file, StringBuilder filePath, uint filePathLength, uint flags);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetProductInfo(uint majorVersion, uint minorVersion, uint servicePackMajor, uint servicePackMinor, out uint productType);
    [DllImport("ntdll.dll", CharSet = CharSet.Unicode)]
    private static extern int RtlGetVersion(ref RTL_OSVERSIONINFOEX versionInformation);
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct RTL_OSVERSIONINFOEX
    {
        public uint dwOSVersionInfoSize;
        public uint dwMajorVersion;
        public uint dwMinorVersion;
        public uint dwBuildNumber;
        public uint dwPlatformId;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string szCSDVersion;
        public ushort wServicePackMajor;
        public ushort wServicePackMinor;
        public ushort wSuiteMask;
        public byte wProductType;
        public byte wReserved;
    }
}

public sealed class AmeR2cRDisposableRoot : IDisposable
{
    private const uint FILE_LIST_DIRECTORY = 0x00000001;
    private const uint FILE_TRAVERSE = 0x00000020;
    private const uint FILE_READ_ATTRIBUTES = 0x00000080;
    private const uint SYNCHRONIZE = 0x00100000;
    private const uint FILE_SHARE_READ = 0x00000001;
    private const uint FILE_SHARE_WRITE = 0x00000002;
    private const uint OPEN_EXISTING = 3;
    private const uint FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
    private const uint FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000;
    private const uint FILE_ATTRIBUTE_DIRECTORY = 0x00000010;
    private const uint FILE_ATTRIBUTE_REPARSE_POINT = 0x00000400;
    private const uint DRIVE_FIXED = 3;
    private static readonly Guid LocalApplicationData = new Guid("F1B32785-6FBA-4FCF-9D55-7B8E7F157091");
    private static string guardrailFaultPoint;
    private static SafeFileHandle guardrailFaultHandle;

    private SafeFileHandle anchorHandle;
    private SafeFileHandle rootHandle;
    private readonly List<SafeFileHandle> trustAnchorHandles = new List<SafeFileHandle>();
    private readonly List<DirectoryIdentity> trustAnchorIdentities = new List<DirectoryIdentity>();
    private DirectoryIdentity anchorIdentity;
    private DirectoryIdentity rootIdentity;

    public string Path { get; private set; }
    public string AnchorPath { get; private set; }
    public string KnownFolderPath { get; private set; }
    public string RootName { get; private set; }
    public string VolumeRoot { get; private set; }
    public string VolumeFormat { get; private set; }
    public string Nonce { get; private set; }
    public string RootIdentityToken { get { return rootIdentity.ToString(); } }
    public bool IsRootHandleHeld { get { return rootHandle != null && !rootHandle.IsClosed; } }

    private AmeR2cRDisposableRoot() { }

    public static AmeR2cRDisposableRoot Create(string nonce)
    {
        return CreateAtAnchor(ResolveKnownFolderPath(LocalApplicationData), nonce);
    }

    public static AmeR2cRDisposableRoot CreateForGuardrail(string anchorPath, string nonce)
    {
        if (String.IsNullOrWhiteSpace(anchorPath) || !System.IO.Path.IsPathRooted(anchorPath)) throw new ArgumentException("The R2c-R guardrail anchor must be an absolute path", "anchorPath");
        return CreateAtAnchor(System.IO.Path.GetFullPath(anchorPath).TrimEnd('\\'), nonce);
    }

    public static AmeR2cRDisposableRoot CreateForKnownFolderGuardrail(string anchorPath, string nonce)
    {
        if (String.IsNullOrWhiteSpace(anchorPath) || !System.IO.Path.IsPathRooted(anchorPath)) throw new ArgumentException("The R2c-R KnownFolder guardrail anchor must be an absolute path", "anchorPath");
        return CreateAtAnchor(System.IO.Path.GetFullPath(anchorPath).TrimEnd('\\'), nonce);
    }

    private static AmeR2cRDisposableRoot CreateAtAnchor(string requestedAnchor, string nonce)
    {
        if (nonce == null || nonce.Length != 32) throw new ArgumentException("The R2c-R nonce must contain 32 lowercase hexadecimal characters", "nonce");
        foreach (char value in nonce)
        {
            if (!((value >= '0' && value <= '9') || (value >= 'a' && value <= 'f'))) throw new ArgumentException("The R2c-R nonce must contain 32 lowercase hexadecimal characters", "nonce");
        }

        AmeR2cRDisposableRoot fixture = new AmeR2cRDisposableRoot();
        Exception cleanupFailure = null;
        try
        {
            fixture.KnownFolderPath = System.IO.Path.GetFullPath(requestedAnchor).TrimEnd('\\');
            SafeFileHandle logicalAnchor = fixture.BindDirectoryChain(fixture.KnownFolderPath, "logical KnownFolder anchor");
            string physicalPath = FinalPath(logicalAnchor);
            SafeFileHandle physicalAnchor = fixture.BindDirectoryChain(physicalPath, "physical KnownFolder anchor");
            if (!Identity(logicalAnchor).Equals(Identity(physicalAnchor))) throw new InvalidOperationException("R2c-R KnownFolder identity changed during read-only physical binding");
            fixture.anchorHandle = physicalAnchor;
            fixture.AnchorPath = FinalPath(fixture.anchorHandle);
            fixture.anchorIdentity = Identity(fixture.anchorHandle);
            fixture.VolumeRoot = System.IO.Path.GetPathRoot(fixture.AnchorPath);
            fixture.VolumeFormat = AssertFixedNtfs(fixture.anchorHandle, fixture.VolumeRoot);

            fixture.Nonce = nonce;
            fixture.RootName = "Cedarflake-Ame-R2c-R-" + nonce + "-" + Guid.NewGuid().ToString("N");
            fixture.rootHandle = OpenRelativeDirectory(fixture.anchorHandle, fixture.RootName, true, false);
            AssertNonReparseDirectory(fixture.rootHandle, "R2c-R disposable root");
            fixture.Path = FinalPath(fixture.rootHandle);
            fixture.rootIdentity = Identity(fixture.rootHandle);
            fixture.ValidateHeldIdentity();
            return fixture;
        }
        catch (Exception failure)
        {
            if (fixture.rootHandle != null)
            {
                try { fixture.DeleteFailedEmptyRoot(); }
                catch (Exception cleanup) { cleanupFailure = cleanup; }
            }
            fixture.DisposeTrustAnchors();
            if (cleanupFailure != null) throw new AggregateException("R2c-R root creation and local cleanup both failed", failure, cleanupFailure);
            throw;
        }
    }

    public static void AssertOwnedLeafName(string name)
    {
        ValidateOwnedLeafName(name);
    }

    public static void AssertDirectoryAnchor(string path)
    {
        AmeR2cRDisposableRoot probe = new AmeR2cRDisposableRoot();
        try
        {
            SafeFileHandle logical = probe.BindDirectoryChain(System.IO.Path.GetFullPath(path), "guardrail logical anchor");
            SafeFileHandle physical = probe.BindDirectoryChain(FinalPath(logical), "guardrail physical anchor");
            if (!Identity(logical).Equals(Identity(physical))) throw new InvalidOperationException("R2c-R guardrail anchor changed during physical binding");
        }
        finally { probe.DisposeTrustAnchors(); }
    }

    public static bool GuardrailFaultHandleIsClosed
    {
        get { return guardrailFaultHandle == null || guardrailFaultHandle.IsClosed; }
    }

    public static void ProbeBindDirectoryChainPostOpenFaultForGuardrail(string path)
    {
        AmeR2cRDisposableRoot probe = new AmeR2cRDisposableRoot();
        BeginGuardrailFault("bind-volume-post-open");
        try
        {
            probe.BindDirectoryChain(System.IO.Path.GetFullPath(path), "guardrail fault probe");
            throw new InvalidOperationException("R2c-R BindDirectoryChain guardrail fault was not injected");
        }
        finally
        {
            EndGuardrailFault();
            probe.DisposeTrustAnchors();
        }
    }

    public void ProbeDeleteFailedEmptyRootPostOpenFaultForGuardrail()
    {
        BeginGuardrailFault("delete-failed-root-post-open");
        try
        {
            DeleteFailedEmptyRoot();
            throw new InvalidOperationException("R2c-R DeleteFailedEmptyRoot guardrail fault was not injected");
        }
        finally { EndGuardrailFault(); }
    }

    public static void ReleaseGuardrailFaultHandleForCleanup()
    {
        SafeFileHandle captured = guardrailFaultHandle;
        guardrailFaultHandle = null;
        if (captured != null) captured.Dispose();
    }

    public void ReleaseDeleteFailedGuardrailFaultHandleForCleanup()
    {
        SafeFileHandle captured = guardrailFaultHandle;
        guardrailFaultHandle = null;
        if (captured != null)
        {
            if (Object.ReferenceEquals(rootHandle, captured)) rootHandle = null;
            captured.Dispose();
        }
    }

    public void ValidateHeldIdentity()
    {
        if (anchorHandle == null || anchorHandle.IsClosed || rootHandle == null || rootHandle.IsClosed) throw new ObjectDisposedException("AmeR2cRDisposableRoot");
        AssertNonReparseDirectory(anchorHandle, "held LocalApplicationData trust anchor");
        if (trustAnchorHandles.Count == 0 || trustAnchorHandles.Count != trustAnchorIdentities.Count) throw new InvalidOperationException("R2c-R held KnownFolder chain is incomplete");
        for (int index = 0; index < trustAnchorHandles.Count; index++)
        {
            SafeFileHandle handle = trustAnchorHandles[index];
            AssertNonReparseDirectory(handle, "held LocalApplicationData physical anchor chain");
            DirectoryIdentity identity = Identity(handle);
            if (!identity.Equals(trustAnchorIdentities[index]) || identity.VolumeSerialNumber != anchorIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R held KnownFolder chain identity changed");
        }
        AssertNonReparseDirectory(rootHandle, "held R2c-R disposable root");
        if (!Identity(anchorHandle).Equals(anchorIdentity) || !Identity(rootHandle).Equals(rootIdentity)) throw new InvalidOperationException("R2c-R held directory file identity changed");
        using (SafeFileHandle currentRoot = OpenRelativeDirectory(anchorHandle, RootName, false, false))
        {
            AssertNonReparseDirectory(currentRoot, "reopened R2c-R root child");
            if (!Identity(currentRoot).Equals(rootIdentity)) throw new InvalidOperationException("R2c-R root is no longer the identity-bound child of its held KnownFolder anchor");
        }
        if (Identity(rootHandle).VolumeSerialNumber != anchorIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R root crossed its trusted volume boundary");
    }

    public void ReleaseRootHandleForCleanup()
    {
        ValidateHeldIdentity();
        rootHandle.Dispose();
        rootHandle = null;
    }

    public void DeleteEmptyRootForCleanup()
    {
        ReleaseRootHandleForCleanup();
        DeleteReleasedRootForCleanup();
    }

    public void DeleteReleasedRootForCleanup()
    {
        AssertHeldAnchorForReleasedCleanup();
        if (rootHandle != null) throw new InvalidOperationException("R2c-R cleanup root blocker was not released");
        using (SafeFileHandle deleteHandle = OpenRelativeDirectory(anchorHandle, RootName, false, true))
        {
            AssertNonReparseDirectory(deleteHandle, "R2c-R cleanup root");
            if (!Identity(deleteHandle).Equals(rootIdentity)) throw new InvalidOperationException("R2c-R cleanup root identity changed after releasing the replacement blocker");
            DeleteEmptyDirectoryHandle(deleteHandle);
        }
    }

    public void DeleteRelocatedExpectedRootForGuardrail(string leafName)
    {
        AssertHeldAnchorForReleasedCleanup();
        if (rootHandle != null) throw new InvalidOperationException("R2c-R guardrail root blocker was not released");
        using (SafeFileHandle deleteHandle = OpenRelativeDirectory(anchorHandle, leafName, false, true))
        {
            AssertNonReparseDirectory(deleteHandle, "relocated R2c-R guardrail root");
            if (!Identity(deleteHandle).Equals(rootIdentity)) throw new InvalidOperationException("R2c-R guardrail relocated root did not retain the fixture identity");
            DeleteEmptyDirectoryHandle(deleteHandle);
        }
    }

    public string CaptureGuardrailReplacementIdentity(string leafName, bool expectReparse)
    {
        AssertHeldAnchorForReleasedCleanup();
        using (SafeFileHandle handle = OpenRelativeDirectory(anchorHandle, leafName, false, false))
        {
            if (expectReparse) AssertReparseDirectory(handle, "R2c-R guardrail replacement");
            else AssertNonReparseDirectory(handle, "R2c-R guardrail replacement");
            DirectoryIdentity identity = Identity(handle);
            if (identity.VolumeSerialNumber != anchorIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail replacement crossed the held fixture volume");
            return identity.ToString();
        }
    }

    public void DeleteGuardrailReplacementForCleanup(string leafName, string expectedIdentityToken, bool expectReparse)
    {
        AssertHeldAnchorForReleasedCleanup();
        if (rootHandle != null) throw new InvalidOperationException("R2c-R guardrail root blocker was not released");
        DirectoryIdentity expectedIdentity = DirectoryIdentity.Parse(expectedIdentityToken);
        using (SafeFileHandle deleteHandle = OpenRelativeDirectory(anchorHandle, leafName, false, true))
        {
            if (expectReparse) AssertReparseDirectory(deleteHandle, "R2c-R guardrail replacement");
            else AssertNonReparseDirectory(deleteHandle, "R2c-R guardrail replacement");
            DirectoryIdentity actualIdentity = Identity(deleteHandle);
            if (!actualIdentity.Equals(expectedIdentity) || actualIdentity.VolumeSerialNumber != anchorIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail replacement identity changed before teardown");
            DeleteEmptyDirectoryHandle(deleteHandle);
        }
    }

    public string CreateGuardrailChildDirectory(string leafName)
    {
        ValidateGuardrailLeafName(leafName);
        ValidateHeldIdentity();
        using (SafeFileHandle handle = OpenRelativeDirectoryComponent(rootHandle, leafName, true, false))
        {
            AssertNonReparseDirectory(handle, "R2c-R guardrail child");
            DirectoryIdentity identity = Identity(handle);
            if (identity.VolumeSerialNumber != rootIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail child crossed its fixture volume");
            return identity.ToString();
        }
    }

    public string CaptureGuardrailChildDirectoryIdentity(string leafName, bool expectReparse)
    {
        ValidateGuardrailLeafName(leafName);
        ValidateHeldIdentity();
        using (SafeFileHandle handle = OpenRelativeDirectoryComponent(rootHandle, leafName, false, false))
        {
            if (expectReparse) AssertReparseDirectory(handle, "R2c-R guardrail child");
            else AssertNonReparseDirectory(handle, "R2c-R guardrail child");
            DirectoryIdentity identity = Identity(handle);
            if (identity.VolumeSerialNumber != rootIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail child crossed its fixture volume");
            return identity.ToString();
        }
    }

    public void DeleteGuardrailChildDirectoryForCleanup(string leafName, string expectedIdentityToken, bool expectReparse)
    {
        ValidateGuardrailLeafName(leafName);
        ValidateHeldIdentity();
        DirectoryIdentity expectedIdentity = DirectoryIdentity.Parse(expectedIdentityToken);
        using (SafeFileHandle handle = OpenRelativeDirectoryComponent(rootHandle, leafName, false, true))
        {
            if (expectReparse) AssertReparseDirectory(handle, "R2c-R guardrail child cleanup");
            else AssertNonReparseDirectory(handle, "R2c-R guardrail child cleanup");
            DirectoryIdentity actualIdentity = Identity(handle);
            if (!actualIdentity.Equals(expectedIdentity) || actualIdentity.VolumeSerialNumber != rootIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail child identity changed before teardown");
            DeleteEmptyDirectoryHandle(handle);
        }
    }

    public void Dispose()
    {
        if (rootHandle != null) { rootHandle.Dispose(); rootHandle = null; }
        DisposeTrustAnchors();
    }

    private SafeFileHandle BindDirectoryChain(string path, string label)
    {
        string fullPath = System.IO.Path.GetFullPath(path).TrimEnd('\\');
        string volumeRoot = System.IO.Path.GetPathRoot(fullPath);
        if (String.IsNullOrEmpty(volumeRoot) || !Regex.IsMatch(volumeRoot, @"\A[A-Za-z]:\\\z", RegexOptions.CultureInvariant)) throw new InvalidOperationException("R2c-R " + label + " must be on a local drive-letter volume");
        SafeFileHandle volumeHandle = OpenDirectoryWithoutDeleteSharing(volumeRoot, "Could not hold the R2c-R " + label + " volume root");
        SafeFileHandle parent;
        DirectoryIdentity volumeIdentity;
        try
        {
            ThrowIfGuardrailFault("bind-volume-post-open", volumeHandle);
            AssertNonReparseDirectory(volumeHandle, "R2c-R " + label + " volume root");
            volumeIdentity = Identity(volumeHandle);
            HoldTrustAnchor(volumeHandle);
            parent = volumeHandle;
            volumeHandle = null;
        }
        finally { if (volumeHandle != null) volumeHandle.Dispose(); }
        string relative = fullPath.Substring(volumeRoot.Length);
        foreach (string component in relative.Split(new char[] { '\\' }, StringSplitOptions.RemoveEmptyEntries))
        {
            SafeFileHandle child = OpenRelativeDirectoryComponent(parent, component, false, false);
            try
            {
                AssertNonReparseDirectory(child, "R2c-R " + label + " component");
                DirectoryIdentity childIdentity = Identity(child);
                if (childIdentity.VolumeSerialNumber != volumeIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R " + label + " crossed its trusted volume boundary");
                HoldTrustAnchor(child);
                parent = child;
                child = null;
            }
            finally { if (child != null) child.Dispose(); }
        }
        return parent;
    }

    private void HoldTrustAnchor(SafeFileHandle handle)
    {
        DirectoryIdentity identity = Identity(handle);
        trustAnchorHandles.Add(handle);
        try { trustAnchorIdentities.Add(identity); }
        catch
        {
            trustAnchorHandles.RemoveAt(trustAnchorHandles.Count - 1);
            throw;
        }
    }

    private void DisposeTrustAnchors()
    {
        for (int index = trustAnchorHandles.Count - 1; index >= 0; index--) trustAnchorHandles[index].Dispose();
        trustAnchorHandles.Clear();
        trustAnchorIdentities.Clear();
        anchorHandle = null;
    }

    private void AssertHeldAnchorForReleasedCleanup()
    {
        if (anchorHandle == null || anchorHandle.IsClosed) throw new ObjectDisposedException("AmeR2cRDisposableRoot");
        if (trustAnchorHandles.Count == 0 || trustAnchorHandles.Count != trustAnchorIdentities.Count) throw new InvalidOperationException("R2c-R cleanup anchor chain is incomplete");
        for (int index = 0; index < trustAnchorHandles.Count; index++)
        {
            SafeFileHandle handle = trustAnchorHandles[index];
            AssertNonReparseDirectory(handle, "held LocalApplicationData physical cleanup anchor");
            if (!Identity(handle).Equals(trustAnchorIdentities[index])) throw new InvalidOperationException("R2c-R cleanup anchor chain identity changed");
        }
        AssertNonReparseDirectory(anchorHandle, "held LocalApplicationData cleanup anchor");
        if (!Identity(anchorHandle).Equals(anchorIdentity)) throw new InvalidOperationException("R2c-R cleanup anchor identity changed");
    }

    private static string ResolveKnownFolderPath(Guid identifier)
    {
        IntPtr rawPath;
        int result = SHGetKnownFolderPath(ref identifier, 0, IntPtr.Zero, out rawPath);
        if (result != 0) Marshal.ThrowExceptionForHR(result);
        try
        {
            string path = Marshal.PtrToStringUni(rawPath);
            if (String.IsNullOrWhiteSpace(path) || !System.IO.Path.IsPathRooted(path)) throw new InvalidOperationException("Could not resolve the LocalApplicationData KnownFolder path");
            return path;
        }
        finally { Marshal.FreeCoTaskMem(rawPath); }
    }

    private static SafeFileHandle OpenDirectoryWithoutDeleteSharing(string path, string message)
    {
        SafeFileHandle handle = CreateFileW(path, FILE_READ_ATTRIBUTES | SYNCHRONIZE, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, IntPtr.Zero);
        if (handle.IsInvalid)
        {
            int error = Marshal.GetLastWin32Error();
            handle.Dispose();
            throw new Win32Exception(error, message);
        }
        return handle;
    }

    private void DeleteFailedEmptyRoot()
    {
        SafeFileHandle failedRootHandle = rootHandle;
        rootHandle = null;
        if (failedRootHandle == null) throw new ObjectDisposedException("AmeR2cRDisposableRoot");
        DirectoryIdentity expectedIdentity;
        try
        {
            ThrowIfGuardrailFault("delete-failed-root-post-open", failedRootHandle);
            expectedIdentity = Identity(failedRootHandle);
        }
        finally { failedRootHandle.Dispose(); }
        using (SafeFileHandle deleteHandle = OpenRelativeDirectory(anchorHandle, RootName, false, true))
        {
            AssertNonReparseDirectory(deleteHandle, "failed R2c-R root cleanup");
            if (!Identity(deleteHandle).Equals(expectedIdentity)) throw new InvalidOperationException("Failed R2c-R root cleanup encountered a replacement identity");
            DeleteEmptyDirectoryHandle(deleteHandle);
        }
    }

    private static void BeginGuardrailFault(string point)
    {
        if (guardrailFaultPoint != null || guardrailFaultHandle != null) throw new InvalidOperationException("R2c-R guardrail fault state was not clean");
        guardrailFaultPoint = point;
    }

    private static void EndGuardrailFault()
    {
        guardrailFaultPoint = null;
    }

    private static void ThrowIfGuardrailFault(string point, SafeFileHandle handle)
    {
        if (!String.Equals(guardrailFaultPoint, point, StringComparison.Ordinal)) return;
        guardrailFaultHandle = handle;
        throw new InvalidOperationException("R2c-R injected post-open handle fault: " + point);
    }

    private static SafeFileHandle OpenRelativeDirectory(SafeFileHandle parent, string name, bool create, bool requestDelete)
    {
        ValidateOwnedLeafName(name);
        return OpenRelativeDirectoryComponentCore(parent, name, create, requestDelete, true);
    }

    private static SafeFileHandle OpenRelativeDirectoryComponent(SafeFileHandle parent, string name, bool create, bool requestDelete)
    {
        return OpenRelativeDirectoryComponentCore(parent, name, create, requestDelete, false);
    }

    private static SafeFileHandle OpenRelativeDirectoryComponentCore(SafeFileHandle parent, string name, bool create, bool requestDelete, bool holdDeleteBlocker)
    {
        ValidateDirectoryComponent(name);
        IntPtr nameBuffer = Marshal.StringToHGlobalUni(name);
        IntPtr nameStructure = IntPtr.Zero;
        IntPtr rawHandle = IntPtr.Zero;
        try
        {
            UNICODE_STRING unicodeName = new UNICODE_STRING();
            unicodeName.Length = checked((ushort)(name.Length * 2));
            unicodeName.MaximumLength = checked((ushort)((name.Length + 1) * 2));
            unicodeName.Buffer = nameBuffer;
            nameStructure = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(UNICODE_STRING)));
            Marshal.StructureToPtr(unicodeName, nameStructure, false);
            OBJECT_ATTRIBUTES attributes = new OBJECT_ATTRIBUTES();
            attributes.Length = (uint)Marshal.SizeOf(typeof(OBJECT_ATTRIBUTES));
            attributes.RootDirectory = parent.DangerousGetHandle();
            attributes.ObjectName = nameStructure;
            attributes.Attributes = 0;
            if (!DirectoryIsCaseSensitive(parent)) attributes.Attributes |= 0x00000040;
            IO_STATUS_BLOCK statusBlock;
            uint desiredAccess = FILE_READ_ATTRIBUTES | SYNCHRONIZE;
            if (holdDeleteBlocker) desiredAccess |= FILE_LIST_DIRECTORY;
            if (requestDelete) desiredAccess |= 0x00010000;
            int status = NtCreateFile(out rawHandle, desiredAccess, ref attributes, out statusBlock, IntPtr.Zero, FILE_ATTRIBUTE_DIRECTORY, FILE_SHARE_READ | FILE_SHARE_WRITE, create ? 2u : 1u, 0x00000001 | 0x00000020 | 0x00004000 | 0x00200000, IntPtr.Zero, 0);
            if (status < 0)
            {
                uint error = RtlNtStatusToDosError(status);
                string operation = create ? "create" : "reopen";
                throw new Win32Exception(unchecked((int)error), "Could not " + operation + " the R2c-R root relative to its held KnownFolder anchor; NTSTATUS=0x" + status.ToString("X8") + " Win32=" + error.ToString());
            }
            SafeFileHandle handle = new SafeFileHandle(rawHandle, true);
            rawHandle = IntPtr.Zero;
            return handle;
        }
        finally
        {
            if (rawHandle != IntPtr.Zero && rawHandle != new IntPtr(-1)) CloseHandle(rawHandle);
            if (nameStructure != IntPtr.Zero) Marshal.FreeHGlobal(nameStructure);
            Marshal.FreeHGlobal(nameBuffer);
        }
    }

    private static bool DirectoryIsCaseSensitive(SafeFileHandle directory)
    {
        FILE_CASE_SENSITIVE_INFO information = new FILE_CASE_SENSITIVE_INFO();
        if (!GetFileInformationByHandleEx(directory, 23, out information, (uint)Marshal.SizeOf(typeof(FILE_CASE_SENSITIVE_INFO)))) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not query R2c-R parent case semantics");
        return (information.Flags & 1) != 0;
    }

    private static void ValidateDirectoryComponent(string name)
    {
        if (String.IsNullOrEmpty(name) || name == "." || name == ".." || name.Length > 255 ||
            name.IndexOfAny(new char[] { '\0', '\\', '/', ':' }) >= 0 ||
            name[name.Length - 1] == '.' || name[name.Length - 1] == ' ') throw new ArgumentException("R2c-R directory component is not one normalized NT name", "name");
    }

    private static void ValidateGuardrailLeafName(string name)
    {
        ValidateDirectoryComponent(name);
        if (!Regex.IsMatch(name, @"\A[a-z][a-z0-9-]{0,63}-[0-9a-f]{32}\z", RegexOptions.CultureInvariant)) throw new ArgumentException("R2c-R guardrail leaf is outside the identity-bound owned-name contract", "name");
    }

    private static void ValidateOwnedLeafName(string name)
    {
        ValidateDirectoryComponent(name);
        if (String.IsNullOrEmpty(name) || name.Length > 160 ||
            !name.StartsWith("Cedarflake-Ame-R2c-R-", StringComparison.Ordinal)) throw new ArgumentException("R2c-R relative directory name is outside the owned-name contract", "name");
        foreach (char value in name)
        {
            if (!((value >= 'A' && value <= 'Z') ||
                  (value >= 'a' && value <= 'z') ||
                  (value >= '0' && value <= '9') || value == '-')) throw new ArgumentException("R2c-R relative directory name contains path, ADS, control, or non-ASCII syntax", "name");
        }
        if (name == "." || name == ".." || name[name.Length - 1] == '.' ||
            name[name.Length - 1] == ' ') throw new ArgumentException("R2c-R relative directory name has a forbidden terminal component", "name");
        if (!Regex.IsMatch(
            name,
            @"\ACedarflake-Ame-R2c-R-[0-9a-f]{32}-[0-9a-f]{32}(?:-moved-[0-9a-f]{32})?\z",
            RegexOptions.CultureInvariant)) throw new ArgumentException("R2c-R relative directory name is not an exact generated owned leaf", "name");
    }

    private static void DeleteEmptyDirectoryHandle(SafeFileHandle handle)
    {
        FILE_DISPOSITION_INFO_EX disposition = new FILE_DISPOSITION_INFO_EX();
        // Retire the proved name when this delete handle closes, even while metadata observers remain.
        disposition.Flags = 0x00000001 | 0x00000002 | 0x00000004;
        if (!SetFileInformationByHandle(handle, 21, ref disposition, (uint)Marshal.SizeOf(typeof(FILE_DISPOSITION_INFO_EX)))) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not delete the exact failed or completed R2c-R root by held handle");
    }

    private static void AssertNonReparseDirectory(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the " + label);
        if ((information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0) throw new InvalidOperationException("The " + label + " is not a directory");
        if ((information.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0) throw new InvalidOperationException("The " + label + " is a reparse point");
    }

    private static void AssertReparseDirectory(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the " + label);
        if ((information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0) throw new InvalidOperationException("The " + label + " is not a directory");
        if ((information.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) == 0) throw new InvalidOperationException("The " + label + " is not a reparse point");
    }

    private static DirectoryIdentity Identity(SafeFileHandle handle)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the R2c-R directory identity");
        return new DirectoryIdentity(information.VolumeSerialNumber, information.FileIndexHigh, information.FileIndexLow);
    }

    private static string FinalPath(SafeFileHandle handle)
    {
        StringBuilder buffer = new StringBuilder(32768);
        uint length = GetFinalPathNameByHandleW(handle, buffer, (uint)buffer.Capacity, 0);
        if (length == 0 || length >= buffer.Capacity) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not resolve the R2c-R directory handle");
        string resolved = buffer.ToString();
        if (resolved.StartsWith("\\\\?\\UNC\\", StringComparison.OrdinalIgnoreCase)) return "\\\\" + resolved.Substring(8);
        return resolved.StartsWith("\\\\?\\", StringComparison.OrdinalIgnoreCase) ? resolved.Substring(4) : resolved;
    }

    private static string AssertFixedNtfs(SafeFileHandle handle, string volumeRoot)
    {
        if (String.IsNullOrEmpty(volumeRoot) || GetDriveTypeW(volumeRoot) != DRIVE_FIXED) throw new InvalidOperationException("R2c-R disposable storage requires a fixed volume");
        StringBuilder volumeName = new StringBuilder(261);
        StringBuilder fileSystemName = new StringBuilder(261);
        uint serial;
        uint maximumComponentLength;
        uint flags;
        if (!GetVolumeInformationByHandleW(handle, volumeName, (uint)volumeName.Capacity, out serial, out maximumComponentLength, out flags, fileSystemName, (uint)fileSystemName.Capacity)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the R2c-R KnownFolder volume");
        string format = fileSystemName.ToString();
        if (!String.Equals(format, "NTFS", StringComparison.Ordinal)) throw new InvalidOperationException("R2c-R disposable storage requires NTFS");
        return format;
    }

    private struct DirectoryIdentity : IEquatable<DirectoryIdentity>
    {
        internal readonly uint VolumeSerialNumber;
        private readonly uint FileIndexHigh;
        private readonly uint FileIndexLow;
        internal DirectoryIdentity(uint volumeSerialNumber, uint fileIndexHigh, uint fileIndexLow) { VolumeSerialNumber = volumeSerialNumber; FileIndexHigh = fileIndexHigh; FileIndexLow = fileIndexLow; }
        public bool Equals(DirectoryIdentity other) { return VolumeSerialNumber == other.VolumeSerialNumber && FileIndexHigh == other.FileIndexHigh && FileIndexLow == other.FileIndexLow; }
        public override bool Equals(object value) { return value is DirectoryIdentity && Equals((DirectoryIdentity)value); }
        public override int GetHashCode() { return VolumeSerialNumber.GetHashCode() ^ FileIndexHigh.GetHashCode() ^ FileIndexLow.GetHashCode(); }
        public override string ToString() { return VolumeSerialNumber.ToString("X8") + ":" + FileIndexHigh.ToString("X8") + ":" + FileIndexLow.ToString("X8"); }
        internal static DirectoryIdentity Parse(string value)
        {
            string[] fields = (value ?? String.Empty).Split(':');
            if (fields.Length != 3) throw new ArgumentException("R2c-R expected identity token is invalid", "value");
            uint volume;
            uint high;
            uint low;
            if (!UInt32.TryParse(fields[0], NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out volume) ||
                !UInt32.TryParse(fields[1], NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out high) ||
                !UInt32.TryParse(fields[2], NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out low)) throw new ArgumentException("R2c-R expected identity token is invalid", "value");
            return new DirectoryIdentity(volume, high, low);
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct FILETIME { public uint LowDateTime; public uint HighDateTime; }
    [StructLayout(LayoutKind.Sequential)]
    private struct BY_HANDLE_FILE_INFORMATION
    {
        public uint FileAttributes;
        public FILETIME CreationTime;
        public FILETIME LastAccessTime;
        public FILETIME LastWriteTime;
        public uint VolumeSerialNumber;
        public uint FileSizeHigh;
        public uint FileSizeLow;
        public uint NumberOfLinks;
        public uint FileIndexHigh;
        public uint FileIndexLow;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct UNICODE_STRING { public ushort Length; public ushort MaximumLength; public IntPtr Buffer; }
    [StructLayout(LayoutKind.Sequential)]
    private struct OBJECT_ATTRIBUTES { public uint Length; public IntPtr RootDirectory; public IntPtr ObjectName; public uint Attributes; public IntPtr SecurityDescriptor; public IntPtr SecurityQualityOfService; }
    [StructLayout(LayoutKind.Sequential)]
    private struct IO_STATUS_BLOCK { public IntPtr Status; public UIntPtr Information; }
    [StructLayout(LayoutKind.Sequential)]
    private struct FILE_DISPOSITION_INFO_EX { public uint Flags; }
    [StructLayout(LayoutKind.Sequential)]
    private struct FILE_CASE_SENSITIVE_INFO { public uint Flags; }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] private static extern int SHGetKnownFolderPath(ref Guid knownFolder, uint flags, IntPtr token, out IntPtr path);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern SafeFileHandle CreateFileW(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, uint creationDisposition, uint flagsAndAttributes, IntPtr templateFile);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetFileInformationByHandle(SafeFileHandle file, out BY_HANDLE_FILE_INFORMATION information);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetFileInformationByHandleEx(SafeFileHandle file, uint informationClass, out FILE_CASE_SENSITIVE_INFO information, uint bufferSize);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool SetFileInformationByHandle(SafeFileHandle file, uint informationClass, ref FILE_DISPOSITION_INFO_EX information, uint bufferSize);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern uint GetFinalPathNameByHandleW(SafeFileHandle file, StringBuilder filePath, uint filePathLength, uint flags);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern uint GetDriveTypeW(string rootPathName);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern bool GetVolumeInformationByHandleW(SafeFileHandle file, StringBuilder volumeNameBuffer, uint volumeNameSize, out uint volumeSerialNumber, out uint maximumComponentLength, out uint fileSystemFlags, StringBuilder fileSystemNameBuffer, uint fileSystemNameSize);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool CloseHandle(IntPtr handle);
    [DllImport("ntdll.dll")] private static extern int NtCreateFile(out IntPtr fileHandle, uint desiredAccess, ref OBJECT_ATTRIBUTES objectAttributes, out IO_STATUS_BLOCK ioStatusBlock, IntPtr allocationSize, uint fileAttributes, uint shareAccess, uint createDisposition, uint createOptions, IntPtr eaBuffer, uint eaLength);
    [DllImport("ntdll.dll")] private static extern uint RtlNtStatusToDosError(int status);
}

public sealed class AmeR2cRAuditedScriptSnapshot : IDisposable
{
    private const uint FILE_READ_DATA = 0x00000001;
    private const uint FILE_TRAVERSE = 0x00000020;
    private const uint FILE_READ_ATTRIBUTES = 0x00000080;
    private const uint SYNCHRONIZE = 0x00100000;
    private const uint FILE_SHARE_READ = 0x00000001;
    private const uint FILE_SHARE_WRITE = 0x00000002;
    private const uint OPEN_EXISTING = 3;
    private const uint FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
    private const uint FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000;
    private const uint FILE_ATTRIBUTE_DIRECTORY = 0x00000010;
    private const uint FILE_ATTRIBUTE_REPARSE_POINT = 0x00000400;
    private const uint FILE_DIRECTORY_FILE = 0x00000001;
    private const uint FILE_SYNCHRONOUS_IO_NONALERT = 0x00000020;
    private const uint FILE_OPEN_FOR_BACKUP_INTENT = 0x00004000;
    private const uint FILE_OPEN_REPARSE_POINT = 0x00200000;

    private readonly List<SafeFileHandle> directoryHandles = new List<SafeFileHandle>();
    private readonly List<ScriptIdentity> directoryIdentities = new List<ScriptIdentity>();
    private SafeFileHandle terminalHandle;
    private FileStream input;
    private ScriptIdentity terminalIdentity;
    private string terminalLeaf;
    private bool disposedForGuardrail;
    private static string guardrailFaultPoint;
    private static SafeFileHandle guardrailFaultHandle;
    private static int guardrailOpenCount;
    private static int guardrailDisposeCount;

    public string Path { get; private set; }
    public long Length { get; private set; }
    public string IdentityToken { get { return terminalIdentity.ToString(); } }

    private AmeR2cRAuditedScriptSnapshot() { }

    public static AmeR2cRAuditedScriptSnapshot Open(string toolRoot, string path)
    {
        string fullToolRoot = System.IO.Path.GetFullPath(toolRoot).TrimEnd('\\');
        string fullPath = System.IO.Path.GetFullPath(path);
        if (!String.Equals(System.IO.Path.GetExtension(fullPath), ".ps1", StringComparison.Ordinal)) throw new InvalidOperationException("R2c-R audited script must have the exact .ps1 extension");
        System.Threading.Interlocked.Increment(ref guardrailOpenCount);

        AmeR2cRAuditedScriptSnapshot snapshot = new AmeR2cRAuditedScriptSnapshot();
        try
        {
            SafeFileHandle toolHandle = snapshot.BindDirectoryChain(fullToolRoot);
            bool caseSensitive = DirectoryIsCaseSensitive(toolHandle);
            StringComparison comparison = caseSensitive ? StringComparison.Ordinal : StringComparison.OrdinalIgnoreCase;
            string toolPrefix = fullToolRoot + "\\";
            if (!fullPath.StartsWith(toolPrefix, comparison)) throw new InvalidOperationException("R2c-R audited script must remain below the held repository tool root");
            string[] components = fullPath.Substring(toolPrefix.Length).Split(new char[] { '\\' }, StringSplitOptions.RemoveEmptyEntries);
            if (components.Length == 0) throw new InvalidOperationException("R2c-R audited script path has no terminal component");
            for (int index = 0; index < components.Length - 1; index++)
            {
                SafeFileHandle child = OpenRelative(toolHandle, components[index], true);
                try
                {
                    snapshot.HoldDirectory(child, "audited-script source parent");
                    ScriptIdentity childIdentity = snapshot.directoryIdentities[snapshot.directoryIdentities.Count - 1];
                    if (childIdentity.VolumeSerialNumber != snapshot.directoryIdentities[0].VolumeSerialNumber) throw new InvalidOperationException("R2c-R audited script source parent crossed its held volume");
                    ThrowIfGuardrailFault("parent-post-open", child);
                    toolHandle = child;
                    child = null;
                }
                finally { if (child != null) child.Dispose(); }
            }
            snapshot.terminalLeaf = components[components.Length - 1];
            ValidateComponent(snapshot.terminalLeaf);
            snapshot.terminalHandle = OpenRelative(toolHandle, snapshot.terminalLeaf, false);
            ThrowIfGuardrailFault("terminal-post-open", snapshot.terminalHandle);
            BY_HANDLE_FILE_INFORMATION information = AssertRegularNonReparseFile(snapshot.terminalHandle, "audited script terminal");
            snapshot.terminalIdentity = Identity(information);
            ScriptIdentity toolIdentity = snapshot.directoryIdentities[snapshot.directoryIdentities.Count - 1];
            if (snapshot.terminalIdentity.VolumeSerialNumber != toolIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R audited script crossed its held tool-root volume");
            snapshot.Length = checked(((long)information.FileSizeHigh << 32) | information.FileSizeLow);
            snapshot.Path = fullPath;
            snapshot.input = new FileStream(snapshot.terminalHandle, FileAccess.Read, 4096, false);
            snapshot.Revalidate();
            return snapshot;
        }
        catch
        {
            snapshot.Dispose();
            throw;
        }
    }

    public string ReadUtf8Text()
    {
        if (input == null || terminalHandle == null || terminalHandle.IsClosed) throw new ObjectDisposedException("AmeR2cRAuditedScriptSnapshot");
        input.Position = 0;
        using (StreamReader reader = new StreamReader(input, new UTF8Encoding(false, true), true, 4096, true))
        {
            return reader.ReadToEnd();
        }
    }

    public void Revalidate()
    {
        if (directoryHandles.Count == 0 || directoryHandles.Count != directoryIdentities.Count) throw new InvalidOperationException("R2c-R audited script parent chain is not held");
        for (int index = 0; index < directoryHandles.Count; index++)
        {
            SafeFileHandle handle = directoryHandles[index];
            if (handle == null || handle.IsClosed) throw new InvalidOperationException("R2c-R audited script parent handle is not held");
            BY_HANDLE_FILE_INFORMATION information = AssertNonReparseDirectory(handle, "audited script parent");
            if (!Identity(information).Equals(directoryIdentities[index])) throw new InvalidOperationException("R2c-R audited script parent identity changed");
        }
        if (terminalHandle == null || terminalHandle.IsClosed || input == null) throw new InvalidOperationException("R2c-R audited script terminal handle is not held");
        BY_HANDLE_FILE_INFORMATION terminalInformation = AssertRegularNonReparseFile(terminalHandle, "held audited script terminal");
        if (!Identity(terminalInformation).Equals(terminalIdentity)) throw new InvalidOperationException("R2c-R audited script terminal identity changed");
        using (SafeFileHandle reopened = OpenRelative(directoryHandles[directoryHandles.Count - 1], terminalLeaf, false))
        {
            BY_HANDLE_FILE_INFORMATION reopenedInformation = AssertRegularNonReparseFile(reopened, "reopened audited script terminal");
            if (!Identity(reopenedInformation).Equals(terminalIdentity)) throw new InvalidOperationException("R2c-R audited script path no longer resolves to the audited object");
        }
    }

    public void ReleaseTerminalForGuardrail()
    {
        if (input != null) { input.Dispose(); input = null; }
        if (terminalHandle != null) { terminalHandle.Dispose(); terminalHandle = null; }
    }

    public void ReleaseAllHandlesForGuardrail()
    {
        ReleaseTerminalForGuardrail();
        DisposeDirectories();
    }

    public bool AllHandlesClosedForGuardrail
    {
        get
        {
            if (terminalHandle != null && !terminalHandle.IsClosed) return false;
            foreach (SafeFileHandle handle in directoryHandles) if (handle != null && !handle.IsClosed) return false;
            return true;
        }
    }

    public static void BeginGuardrailFault(string point)
    {
        if (guardrailFaultPoint != null || guardrailFaultHandle != null) throw new InvalidOperationException("R2c-R audited-script fault state was not clean");
        guardrailFaultPoint = point;
    }

    public static void EndGuardrailFault()
    {
        guardrailFaultPoint = null;
    }

    public static bool GuardrailFaultHandleIsClosed
    {
        get { return guardrailFaultHandle != null && guardrailFaultHandle.IsClosed; }
    }

    public static void ResetGuardrailFaultHandle()
    {
        SafeFileHandle captured = guardrailFaultHandle;
        guardrailFaultHandle = null;
        if (captured != null && !captured.IsClosed) captured.Dispose();
    }

    public static int GuardrailOpenCount { get { return guardrailOpenCount; } }
    public static int GuardrailDisposeCount { get { return guardrailDisposeCount; } }

    public static void ResetGuardrailOwnershipCounts()
    {
        System.Threading.Interlocked.Exchange(ref guardrailOpenCount, 0);
        System.Threading.Interlocked.Exchange(ref guardrailDisposeCount, 0);
    }

    private SafeFileHandle BindDirectoryChain(string path)
    {
        string volumeRoot = System.IO.Path.GetPathRoot(path);
        if (String.IsNullOrEmpty(volumeRoot) || !Regex.IsMatch(volumeRoot, @"\A[A-Za-z]:\\\z", RegexOptions.CultureInvariant)) throw new InvalidOperationException("R2c-R audited script tool root must use one local drive-letter volume");
        SafeFileHandle volume = CreateFileW(volumeRoot, FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, IntPtr.Zero);
        if (volume.IsInvalid)
        {
            int error = Marshal.GetLastWin32Error();
            volume.Dispose();
            throw new Win32Exception(error, "Could not hold the R2c-R audited-script volume root");
        }
        try
        {
            HoldDirectory(volume, "audited-script volume root");
            ThrowIfGuardrailFault("parent-post-open", volume);
            volume = null;
        }
        finally { if (volume != null) volume.Dispose(); }
        SafeFileHandle parent = directoryHandles[directoryHandles.Count - 1];
        string relative = path.Substring(volumeRoot.Length);
        foreach (string component in relative.Split(new char[] { '\\' }, StringSplitOptions.RemoveEmptyEntries))
        {
            SafeFileHandle child = OpenRelative(parent, component, true);
            try
            {
                HoldDirectory(child, "audited-script parent component");
                ScriptIdentity childIdentity = directoryIdentities[directoryIdentities.Count - 1];
                if (childIdentity.VolumeSerialNumber != directoryIdentities[0].VolumeSerialNumber) throw new InvalidOperationException("R2c-R audited script parent chain crossed its held volume");
                ThrowIfGuardrailFault("parent-post-open", child);
                parent = child;
                child = null;
            }
            finally { if (child != null) child.Dispose(); }
        }
        return parent;
    }

    private void HoldDirectory(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information = AssertNonReparseDirectory(handle, label);
        directoryHandles.Add(handle);
        try { directoryIdentities.Add(Identity(information)); }
        catch
        {
            directoryHandles.RemoveAt(directoryHandles.Count - 1);
            throw;
        }
    }

    private static SafeFileHandle OpenRelative(SafeFileHandle parent, string name, bool directory)
    {
        ValidateComponent(name);
        IntPtr nameBuffer = Marshal.StringToHGlobalUni(name);
        IntPtr nameStructure = IntPtr.Zero;
        IntPtr rawHandle = IntPtr.Zero;
        try
        {
            UNICODE_STRING unicodeName = new UNICODE_STRING();
            unicodeName.Length = checked((ushort)(name.Length * 2));
            unicodeName.MaximumLength = checked((ushort)((name.Length + 1) * 2));
            unicodeName.Buffer = nameBuffer;
            nameStructure = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(UNICODE_STRING)));
            Marshal.StructureToPtr(unicodeName, nameStructure, false);
            OBJECT_ATTRIBUTES attributes = new OBJECT_ATTRIBUTES();
            attributes.Length = (uint)Marshal.SizeOf(typeof(OBJECT_ATTRIBUTES));
            attributes.RootDirectory = parent.DangerousGetHandle();
            attributes.ObjectName = nameStructure;
            if (!DirectoryIsCaseSensitive(parent)) attributes.Attributes = 0x00000040;
            IO_STATUS_BLOCK statusBlock;
            uint desiredAccess = (directory ? FILE_TRAVERSE : FILE_READ_DATA) | FILE_READ_ATTRIBUTES | SYNCHRONIZE;
            uint shareAccess = directory ? FILE_SHARE_READ | FILE_SHARE_WRITE : FILE_SHARE_READ;
            uint createOptions = FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT | FILE_OPEN_REPARSE_POINT;
            if (directory) createOptions |= FILE_DIRECTORY_FILE;
            int status = NtCreateFile(out rawHandle, desiredAccess, ref attributes, out statusBlock, IntPtr.Zero, 0, shareAccess, 1, createOptions, IntPtr.Zero, 0);
            if (status < 0)
            {
                uint error = RtlNtStatusToDosError(status);
                throw new Win32Exception(unchecked((int)error), "Could not open the R2c-R audited script relative to its held parent; NTSTATUS=0x" + status.ToString("X8") + " Win32=" + error.ToString());
            }
            SafeFileHandle handle = new SafeFileHandle(rawHandle, true);
            rawHandle = IntPtr.Zero;
            return handle;
        }
        finally
        {
            if (rawHandle != IntPtr.Zero && rawHandle != new IntPtr(-1)) CloseHandle(rawHandle);
            if (nameStructure != IntPtr.Zero) Marshal.FreeHGlobal(nameStructure);
            Marshal.FreeHGlobal(nameBuffer);
        }
    }

    private static void ValidateComponent(string name)
    {
        if (String.IsNullOrEmpty(name) || name == "." || name == ".." || name.Length > 255 || name.IndexOfAny(new char[] { '\0', '\\', '/', ':' }) >= 0 || name[name.Length - 1] == '.' || name[name.Length - 1] == ' ') throw new InvalidOperationException("R2c-R audited script path component is not one normalized NT name");
    }

    private static bool DirectoryIsCaseSensitive(SafeFileHandle handle)
    {
        FILE_CASE_SENSITIVE_INFO information;
        if (!GetFileInformationByHandleEx(handle, 23, out information, (uint)Marshal.SizeOf(typeof(FILE_CASE_SENSITIVE_INFO)))) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not query the R2c-R audited-script parent case semantics");
        return (information.Flags & 1) != 0;
    }

    private static BY_HANDLE_FILE_INFORMATION AssertNonReparseDirectory(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (handle == null || handle.IsClosed || handle.IsInvalid || !GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the R2c-R " + label);
        if ((information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0) throw new InvalidOperationException("R2c-R " + label + " is not a directory");
        if ((information.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0) throw new InvalidOperationException("R2c-R " + label + " is a reparse point");
        return information;
    }

    private static BY_HANDLE_FILE_INFORMATION AssertRegularNonReparseFile(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (handle == null || handle.IsClosed || handle.IsInvalid || !GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the R2c-R " + label);
        if ((information.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0) throw new InvalidOperationException("R2c-R " + label + " is a reparse point");
        if ((information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0) throw new InvalidOperationException("R2c-R " + label + " is not a regular file");
        return information;
    }

    private static ScriptIdentity Identity(BY_HANDLE_FILE_INFORMATION information)
    {
        return new ScriptIdentity(information.VolumeSerialNumber, information.FileIndexHigh, information.FileIndexLow);
    }

    private static void ThrowIfGuardrailFault(string point, SafeFileHandle handle)
    {
        if (!String.Equals(guardrailFaultPoint, point, StringComparison.Ordinal)) return;
        guardrailFaultHandle = handle;
        throw new InvalidOperationException("R2c-R injected audited-script handle fault: " + point);
    }

    private void DisposeDirectories()
    {
        for (int index = directoryHandles.Count - 1; index >= 0; index--) directoryHandles[index].Dispose();
        directoryHandles.Clear();
        directoryIdentities.Clear();
    }

    public void Dispose()
    {
        if (disposedForGuardrail) return;
        disposedForGuardrail = true;
        ReleaseTerminalForGuardrail();
        DisposeDirectories();
        System.Threading.Interlocked.Increment(ref guardrailDisposeCount);
    }

    private struct ScriptIdentity : IEquatable<ScriptIdentity>
    {
        internal readonly uint VolumeSerialNumber;
        private readonly uint FileIndexHigh;
        private readonly uint FileIndexLow;
        internal ScriptIdentity(uint volumeSerialNumber, uint fileIndexHigh, uint fileIndexLow) { VolumeSerialNumber = volumeSerialNumber; FileIndexHigh = fileIndexHigh; FileIndexLow = fileIndexLow; }
        public bool Equals(ScriptIdentity other) { return VolumeSerialNumber == other.VolumeSerialNumber && FileIndexHigh == other.FileIndexHigh && FileIndexLow == other.FileIndexLow; }
        public override bool Equals(object value) { return value is ScriptIdentity && Equals((ScriptIdentity)value); }
        public override int GetHashCode() { return VolumeSerialNumber.GetHashCode() ^ FileIndexHigh.GetHashCode() ^ FileIndexLow.GetHashCode(); }
        public override string ToString() { return VolumeSerialNumber.ToString("X8") + ":" + FileIndexHigh.ToString("X8") + ":" + FileIndexLow.ToString("X8"); }
    }

    [StructLayout(LayoutKind.Sequential)] private struct FILETIME { public uint LowDateTime; public uint HighDateTime; }
    [StructLayout(LayoutKind.Sequential)] private struct BY_HANDLE_FILE_INFORMATION { public uint FileAttributes; public FILETIME CreationTime; public FILETIME LastAccessTime; public FILETIME LastWriteTime; public uint VolumeSerialNumber; public uint FileSizeHigh; public uint FileSizeLow; public uint NumberOfLinks; public uint FileIndexHigh; public uint FileIndexLow; }
    [StructLayout(LayoutKind.Sequential)] private struct UNICODE_STRING { public ushort Length; public ushort MaximumLength; public IntPtr Buffer; }
    [StructLayout(LayoutKind.Sequential)] private struct OBJECT_ATTRIBUTES { public uint Length; public IntPtr RootDirectory; public IntPtr ObjectName; public uint Attributes; public IntPtr SecurityDescriptor; public IntPtr SecurityQualityOfService; }
    [StructLayout(LayoutKind.Sequential)] private struct IO_STATUS_BLOCK { public IntPtr Status; public UIntPtr Information; }
    [StructLayout(LayoutKind.Sequential)] private struct FILE_CASE_SENSITIVE_INFO { public uint Flags; }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern SafeFileHandle CreateFileW(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, uint creationDisposition, uint flagsAndAttributes, IntPtr templateFile);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetFileInformationByHandle(SafeFileHandle file, out BY_HANDLE_FILE_INFORMATION information);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetFileInformationByHandleEx(SafeFileHandle file, uint informationClass, out FILE_CASE_SENSITIVE_INFO information, uint bufferSize);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool CloseHandle(IntPtr handle);
    [DllImport("ntdll.dll")] private static extern int NtCreateFile(out IntPtr fileHandle, uint desiredAccess, ref OBJECT_ATTRIBUTES objectAttributes, out IO_STATUS_BLOCK ioStatusBlock, IntPtr allocationSize, uint fileAttributes, uint shareAccess, uint createDisposition, uint createOptions, IntPtr eaBuffer, uint eaLength);
    [DllImport("ntdll.dll")] private static extern uint RtlNtStatusToDosError(int status);
}

public sealed class AmeR2cRProcessJob : IDisposable
{
    private SafeFileHandle jobHandle;
    private SafeFileHandle processHandle;
    private FileStream output;

    public AmeR2cRProcessJob()
    {
        IntPtr rawHandle = CreateJobObject(IntPtr.Zero, null);
        if (rawHandle == IntPtr.Zero || rawHandle == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not create the R2c-R process job");
        jobHandle = new SafeFileHandle(rawHandle, true);
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
        limits.BasicLimitInformation.LimitFlags = 0x00002000;
        int length = Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
        IntPtr buffer = Marshal.AllocHGlobal(length);
        try
        {
            Marshal.StructureToPtr(limits, buffer, false);
            if (!SetInformationJobObject(rawHandle, 9, buffer, (uint)length)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not configure the R2c-R process job");
        }
        catch { jobHandle.Dispose(); throw; }
        finally { Marshal.FreeHGlobal(buffer); }
    }

    public Process Start(string fileName, string[] arguments, string workingDirectory, string outputPath)
    {
        if (jobHandle == null || jobHandle.IsClosed) throw new ObjectDisposedException("AmeR2cRProcessJob");
        if (processHandle != null) throw new InvalidOperationException("The R2c-R process job already owns a process");
        output = new FileStream(outputPath, FileMode.CreateNew, FileAccess.ReadWrite, FileShare.ReadWrite | FileShare.Delete, 4096, FileOptions.DeleteOnClose);
        IntPtr outputHandle = output.SafeFileHandle.DangerousGetHandle();
        if (!SetHandleInformation(outputHandle, 1, 1)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inherit the R2c-R output handle");
        STARTUPINFO startup = new STARTUPINFO();
        startup.cb = (uint)Marshal.SizeOf(typeof(STARTUPINFO));
        startup.dwFlags = 0x00000100;
        startup.hStdInput = GetStdHandle(-10);
        startup.hStdOutput = outputHandle;
        startup.hStdError = outputHandle;
        PROCESS_INFORMATION process;
        StringBuilder commandLine = new StringBuilder(Quote(fileName));
        foreach (string argument in arguments) commandLine.Append(' ').Append(Quote(argument));
        if (!CreateProcess(fileName, commandLine, IntPtr.Zero, IntPtr.Zero, true, 0x00000004, IntPtr.Zero, workingDirectory, ref startup, out process)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not start the R2c-R owned process");
        try
        {
            if (!AssignProcessToJobObject(jobHandle.DangerousGetHandle(), process.hProcess))
            {
                int error = Marshal.GetLastWin32Error(); TerminateProcess(process.hProcess, 1); throw new Win32Exception(error, "Could not assign the R2c-R process to its job");
            }
            if (ResumeThread(process.hThread) == UInt32.MaxValue)
            {
                int error = Marshal.GetLastWin32Error(); TerminateProcess(process.hProcess, 1); throw new Win32Exception(error, "Could not resume the R2c-R owned process");
            }
            processHandle = new SafeFileHandle(process.hProcess, true);
            process.hProcess = IntPtr.Zero;
            return Process.GetProcessById((int)process.dwProcessId);
        }
        finally
        {
            CloseHandle(process.hThread);
            if (process.hProcess != IntPtr.Zero) CloseHandle(process.hProcess);
        }
    }

    public uint PrimaryExitCode
    {
        get
        {
            uint exitCode;
            if (processHandle == null || processHandle.IsClosed || !GetExitCodeProcess(processHandle.DangerousGetHandle(), out exitCode)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the R2c-R exit code");
            if (exitCode == 259) throw new InvalidOperationException("The R2c-R owned process is still active");
            return exitCode;
        }
    }

    public string[] ReadOutputLines()
    {
        if (output == null) return new string[0];
        output.Flush();
        output.Position = 0;
        List<string> lines = new List<string>();
        using (StreamReader reader = new StreamReader(output, Encoding.UTF8, true, 4096, true))
        {
            string line;
            while ((line = reader.ReadLine()) != null) lines.Add(line);
        }
        return lines.ToArray();
    }

    public void TerminateOwnedTree()
    {
        if (jobHandle != null) { jobHandle.Dispose(); jobHandle = null; }
    }

    private static string Quote(string value)
    {
        if (value.Length > 0 && value.IndexOfAny(new char[] { ' ', '\t', '\n', '\v', '"' }) < 0) return value;
        StringBuilder quoted = new StringBuilder("\"");
        int backslashes = 0;
        foreach (char current in value)
        {
            if (current == '\\') backslashes++;
            else if (current == '"') { quoted.Append('\\', backslashes * 2 + 1).Append(current); backslashes = 0; }
            else { quoted.Append('\\', backslashes).Append(current); backslashes = 0; }
        }
        return quoted.Append('\\', backslashes * 2).Append('"').ToString();
    }

    public void Dispose()
    {
        if (jobHandle != null) { jobHandle.Dispose(); jobHandle = null; }
        if (processHandle != null) { processHandle.Dispose(); processHandle = null; }
        if (output != null) { output.Dispose(); output = null; }
    }

    [StructLayout(LayoutKind.Sequential)] private struct JOBOBJECT_BASIC_LIMIT_INFORMATION { public long PerProcessUserTimeLimit; public long PerJobUserTimeLimit; public uint LimitFlags; public UIntPtr MinimumWorkingSetSize; public UIntPtr MaximumWorkingSetSize; public uint ActiveProcessLimit; public UIntPtr Affinity; public uint PriorityClass; public uint SchedulingClass; }
    [StructLayout(LayoutKind.Sequential)] private struct IO_COUNTERS { public ulong ReadOperationCount; public ulong WriteOperationCount; public ulong OtherOperationCount; public ulong ReadTransferCount; public ulong WriteTransferCount; public ulong OtherTransferCount; }
    [StructLayout(LayoutKind.Sequential)] private struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION { public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation; public IO_COUNTERS IoInfo; public UIntPtr ProcessMemoryLimit; public UIntPtr JobMemoryLimit; public UIntPtr PeakProcessMemoryUsed; public UIntPtr PeakJobMemoryUsed; }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] private struct STARTUPINFO { public uint cb; public string lpReserved; public string lpDesktop; public string lpTitle; public uint dwX; public uint dwY; public uint dwXSize; public uint dwYSize; public uint dwXCountChars; public uint dwYCountChars; public uint dwFillAttribute; public uint dwFlags; public ushort wShowWindow; public ushort cbReserved2; public IntPtr lpReserved2; public IntPtr hStdInput; public IntPtr hStdOutput; public IntPtr hStdError; }
    [StructLayout(LayoutKind.Sequential)] private struct PROCESS_INFORMATION { public IntPtr hProcess; public IntPtr hThread; public uint dwProcessId; public uint dwThreadId; }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern IntPtr CreateJobObject(IntPtr securityAttributes, string name);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool SetInformationJobObject(IntPtr job, uint informationClass, IntPtr information, uint informationLength);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern bool CreateProcess(string applicationName, StringBuilder commandLine, IntPtr processAttributes, IntPtr threadAttributes, bool inheritHandles, uint creationFlags, IntPtr environment, string currentDirectory, ref STARTUPINFO startupInformation, out PROCESS_INFORMATION processInformation);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool TerminateProcess(IntPtr process, uint exitCode);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool SetHandleInformation(IntPtr handle, uint mask, uint flags);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern IntPtr GetStdHandle(int standardHandle);
}
'@
        if ($ForceCompilerFailure) {
            $nativeTypeDefinition += "`npublic class AmeR2cRForcedCompilerFailure {"
        }
        Add-Type -TypeDefinition $nativeTypeDefinition
        Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $bootstrap
        [AmeR2cRNative]::AssertDirectoryAnchor([string]$bootstrap.Path)
    } finally {
        try {
            if ($environmentCaptured -and $environmentMutated) {
                try {
                    if ($null -eq $previousTemp) {
                        [Environment]::SetEnvironmentVariable(
                            "TEMP",
                            [Management.Automation.Language.NullString]::Value,
                            [EnvironmentVariableTarget]::Process
                        )
                    } else {
                        [Environment]::SetEnvironmentVariable(
                            "TEMP",
                            $previousTemp,
                            [EnvironmentVariableTarget]::Process
                        )
                    }
                } finally {
                    if ($null -eq $previousTmp) {
                        [Environment]::SetEnvironmentVariable(
                            "TMP",
                            [Management.Automation.Language.NullString]::Value,
                            [EnvironmentVariableTarget]::Process
                        )
                    } else {
                        [Environment]::SetEnvironmentVariable(
                            "TMP",
                            $previousTmp,
                            [EnvironmentVariableTarget]::Process
                        )
                    }
                }
            }
        } finally {
            if ($null -ne $bootstrap) {
                Remove-AmeR2cRCompilerBootstrap -Bootstrap $bootstrap
            }
        }
    }
}
