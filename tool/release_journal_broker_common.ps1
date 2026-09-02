$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$script:AmeBrokerServiceName = "CedarflakeAmeJournalBroker"
$script:AmeBrokerDisplayName = "Cedarflake Ame Journal Broker"
$script:AmeBrokerBinaryName = "cedarflake_ame_journal_broker.exe"
$script:AmeBrokerRelativeInstallDirectory = "Cedarflake Ame\Journal Broker"
$script:AmeApplicationBinaryName = "cedarflake_ame.exe"
$script:AmeApplicationRelativeInstallDirectory = "Cedarflake Ame\Application"
$script:AmeBrokerRequiredPrivilege = "SeManageVolumePrivilege"
$script:AmeBrokerServiceDacl = "O:BAG:BAD:(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCRPLO;;;IU)"
$script:AmeBrokerProgramFilesX64KnownFolderId = "6D809377-6AF0-444b-8957-A3773F02200E"
$script:AmeBrokerAdministratorsSid = "S-1-5-32-544"
$script:AmeBrokerSystemSid = "S-1-5-18"
$script:AmeBrokerTrustedInstallerSid = "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"
$script:AmeBrokerServiceSid = "S-1-5-80-2098581772-366539847-2170527201-1913264099-3006267874"
$script:AmeBrokerUsersSid = "S-1-5-32-545"
$script:AmeBrokerProtectedDirectorySddl = "O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200A9;;;BU)"
$script:AmeApplicationProtectedDirectorySddl = "O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200A9;;;BU)"
$script:AmeBrokerProtocolVersion = 5
$script:AmeBrokerMaximumFrameBytes = 1048576
$script:AmeBrokerTransactionSchema = 2
$script:AmeBrokerTransactionMarkerName = ".cedarflake-ame-journal-broker-transaction-v2.json"
$script:AmeBrokerTransactionMaximumBytes = 65536

if (-not ("Ame.JournalBrokerInstaller.NativeMethods" -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace Ame.JournalBrokerInstaller
{
    public static class NativeMethods
    {
        private const uint FileShareRead = 0x00000001;
        private const uint FileShareWrite = 0x00000002;
        private const uint FileShareDelete = 0x00000004;
        private const uint OpenExisting = 3;
        private const uint FileFlagBackupSemantics = 0x02000000;
        private const uint SecurityDescriptorRevision = 1;
        private const uint ScManagerConnect = 0x00000001;
        private const uint ReadControl = 0x00020000;
        private const uint OwnerSecurityInformation = 0x00000001;
        private const uint GroupSecurityInformation = 0x00000002;
        private const uint DaclSecurityInformation = 0x00000004;
        private const int ErrorInsufficientBuffer = 122;
        private const uint MoveFileReplaceExisting = 0x00000001;
        private const uint MoveFileWriteThrough = 0x00000008;

        [StructLayout(LayoutKind.Sequential)]
        private struct ByHandleFileInformation
        {
            public uint FileAttributes;
            public System.Runtime.InteropServices.ComTypes.FILETIME CreationTime;
            public System.Runtime.InteropServices.ComTypes.FILETIME LastAccessTime;
            public System.Runtime.InteropServices.ComTypes.FILETIME LastWriteTime;
            public uint VolumeSerialNumber;
            public uint FileSizeHigh;
            public uint FileSizeLow;
            public uint NumberOfLinks;
            public uint FileIndexHigh;
            public uint FileIndexLow;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct SecurityAttributes
        {
            public int Length;
            public IntPtr SecurityDescriptor;
            [MarshalAs(UnmanagedType.Bool)]
            public bool InheritHandle;
        }

        [DllImport("shell32.dll")]
        private static extern int SHGetKnownFolderPath(
            ref Guid folderId,
            uint flags,
            IntPtr token,
            out IntPtr path
        );

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern SafeFileHandle CreateFileW(
            string fileName,
            uint desiredAccess,
            uint shareMode,
            IntPtr securityAttributes,
            uint creationDisposition,
            uint flagsAndAttributes,
            IntPtr templateFile
        );

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern uint GetFinalPathNameByHandleW(
            SafeFileHandle file,
            StringBuilder path,
            uint pathLength,
            uint flags
        );

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetFileInformationByHandle(
            SafeFileHandle file,
            out ByHandleFileInformation information
        );

        [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool ConvertStringSecurityDescriptorToSecurityDescriptorW(
            string stringSecurityDescriptor,
            uint stringSDRevision,
            out IntPtr securityDescriptor,
            out uint securityDescriptorSize
        );

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CreateDirectoryW(
            string path,
            ref SecurityAttributes securityAttributes
        );

        [DllImport("kernel32.dll")]
        private static extern IntPtr LocalFree(IntPtr memory);

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool MoveFileExW(
            string existingFileName,
            string newFileName,
            uint flags
        );

        [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr OpenSCManagerW(
            string machineName,
            string databaseName,
            uint desiredAccess
        );

        [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr OpenServiceW(
            IntPtr serviceManager,
            string serviceName,
            uint desiredAccess
        );

        [DllImport("advapi32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CloseServiceHandle(IntPtr serviceHandle);

        [DllImport("advapi32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool QueryServiceObjectSecurity(
            IntPtr service,
            uint securityInformation,
            byte[] securityDescriptor,
            uint bufferSize,
            out uint bytesNeeded
        );

        [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool ConvertSecurityDescriptorToStringSecurityDescriptorW(
            byte[] securityDescriptor,
            uint requestedStringSDRevision,
            uint securityInformation,
            out IntPtr stringSecurityDescriptor,
            out uint stringSecurityDescriptorLength
        );

        public static string GetProgramFilesX64(string folderIdText)
        {
            Guid folderId = new Guid(folderIdText);
            IntPtr pathPointer = IntPtr.Zero;
            int result = SHGetKnownFolderPath(ref folderId, 0, IntPtr.Zero, out pathPointer);
            if (result < 0)
            {
                Marshal.ThrowExceptionForHR(result);
            }
            try
            {
                string path = Marshal.PtrToStringUni(pathPointer);
                if (String.IsNullOrWhiteSpace(path))
                {
                    throw new InvalidOperationException("Program Files x64 known folder is empty");
                }
                return path;
            }
            finally
            {
                if (pathPointer != IntPtr.Zero)
                {
                    Marshal.FreeCoTaskMem(pathPointer);
                }
            }
        }

        public static string GetFinalPath(string path)
        {
            using (SafeFileHandle handle = CreateFileW(
                path,
                0,
                FileShareRead | FileShareWrite | FileShareDelete,
                IntPtr.Zero,
                OpenExisting,
                FileFlagBackupSemantics,
                IntPtr.Zero
            ))
            {
                if (handle.IsInvalid)
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                uint capacity = 512;
                while (true)
                {
                    StringBuilder buffer = new StringBuilder((int)capacity);
                    uint length = GetFinalPathNameByHandleW(handle, buffer, capacity, 0);
                    if (length == 0)
                    {
                        throw new Win32Exception(Marshal.GetLastWin32Error());
                    }
                    if (length < capacity)
                    {
                        return buffer.ToString();
                    }
                    capacity = checked(length + 1);
                }
            }
        }

        public static void CreateDirectoryWithSddl(string path, string sddl)
        {
            IntPtr securityDescriptor = IntPtr.Zero;
            uint securityDescriptorSize;
            if (!ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl,
                SecurityDescriptorRevision,
                out securityDescriptor,
                out securityDescriptorSize
            ))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            try
            {
                SecurityAttributes attributes = new SecurityAttributes();
                attributes.Length = Marshal.SizeOf(typeof(SecurityAttributes));
                attributes.SecurityDescriptor = securityDescriptor;
                attributes.InheritHandle = false;
                if (!CreateDirectoryW(path, ref attributes))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
            }
            finally
            {
                if (securityDescriptor != IntPtr.Zero)
                {
                    LocalFree(securityDescriptor);
                }
            }
        }

        public static void ReplaceFileWriteThrough(string sourcePath, string destinationPath)
        {
            if (!MoveFileExW(
                sourcePath,
                destinationPath,
                MoveFileReplaceExisting | MoveFileWriteThrough
            ))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
        }

        public static void MoveFileNoReplaceWriteThrough(
            string sourcePath,
            string destinationPath
        )
        {
            if (!MoveFileExW(sourcePath, destinationPath, MoveFileWriteThrough))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
        }

        public static string GetFileIdentity(string path)
        {
            using (SafeFileHandle handle = CreateFileW(
                path,
                0,
                FileShareRead | FileShareWrite | FileShareDelete,
                IntPtr.Zero,
                OpenExisting,
                0,
                IntPtr.Zero
            ))
            {
                if (handle.IsInvalid)
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                ByHandleFileInformation information;
                if (!GetFileInformationByHandle(handle, out information))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                ulong fileIndex = ((ulong)information.FileIndexHigh << 32) |
                    information.FileIndexLow;
                return information.VolumeSerialNumber.ToString("X8") + ":" +
                    fileIndex.ToString("X16");
            }
        }

        public static string GetServiceSecuritySddl(string serviceName)
        {
            uint securityInformation = OwnerSecurityInformation |
                GroupSecurityInformation |
                DaclSecurityInformation;
            IntPtr manager = OpenSCManagerW(null, null, ScManagerConnect);
            if (manager == IntPtr.Zero)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            IntPtr service = IntPtr.Zero;
            try
            {
                service = OpenServiceW(manager, serviceName, ReadControl);
                if (service == IntPtr.Zero)
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                uint required;
                QueryServiceObjectSecurity(service, securityInformation, null, 0, out required);
                if (required == 0 || Marshal.GetLastWin32Error() != ErrorInsufficientBuffer)
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                byte[] descriptor = new byte[required];
                if (!QueryServiceObjectSecurity(
                    service,
                    securityInformation,
                    descriptor,
                    required,
                    out required
                ))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                IntPtr sddl = IntPtr.Zero;
                uint sddlLength;
                try
                {
                    if (!ConvertSecurityDescriptorToStringSecurityDescriptorW(
                        descriptor,
                        SecurityDescriptorRevision,
                        securityInformation,
                        out sddl,
                        out sddlLength
                    ))
                    {
                        throw new Win32Exception(Marshal.GetLastWin32Error());
                    }
                    string value = Marshal.PtrToStringUni(sddl);
                    if (String.IsNullOrWhiteSpace(value))
                    {
                        throw new InvalidOperationException("Service security descriptor is empty");
                    }
                    return value;
                }
                finally
                {
                    if (sddl != IntPtr.Zero)
                    {
                        LocalFree(sddl);
                    }
                }
            }
            finally
            {
                if (service != IntPtr.Zero)
                {
                    CloseServiceHandle(service);
                }
                CloseServiceHandle(manager);
            }
        }
    }
}
"@
}

function Get-AmeBrokerServicePlan {
    return [pscustomobject]@{
        ServiceName = $script:AmeBrokerServiceName
        DisplayName = $script:AmeBrokerDisplayName
        BinaryName = $script:AmeBrokerBinaryName
        ServiceType = "own"
        StartMode = "demand"
        ServiceAccount = "LocalSystem"
        ServiceSidType = "restricted"
        RequiredPrivileges = @($script:AmeBrokerRequiredPrivilege)
        ServiceDacl = $script:AmeBrokerServiceDacl
    }
}

function Get-AmeBrokerProgramFilesX64 {
    if (-not [Environment]::Is64BitProcess) {
        throw "The journal broker installer must run in a native Windows x64 process"
    }
    $path = [Ame.JournalBrokerInstaller.NativeMethods]::GetProgramFilesX64(
        $script:AmeBrokerProgramFilesX64KnownFolderId
    )
    if (-not [System.IO.Path]::IsPathRooted($path)) {
        throw "Program Files x64 known folder is not an absolute path"
    }
    return [System.IO.Path]::GetFullPath($path)
}

function Get-AmeBrokerInstallDirectory {
    $programFilesPath = Get-AmeBrokerProgramFilesX64
    return [System.IO.Path]::GetFullPath(
        (Join-Path $programFilesPath $script:AmeBrokerRelativeInstallDirectory)
    )
}

function Get-AmeApplicationInstallDirectory {
    $programFilesPath = Get-AmeBrokerProgramFilesX64
    return [System.IO.Path]::GetFullPath(
        (Join-Path $programFilesPath $script:AmeApplicationRelativeInstallDirectory)
    )
}

function Get-AmeApplicationInstalledBinaryPath {
    return Join-Path (Get-AmeApplicationInstallDirectory) $script:AmeApplicationBinaryName
}

function Get-AmeProductParentDirectory {
    return [IO.Path]::GetDirectoryName((Get-AmeBrokerInstallDirectory))
}

function Get-AmeBrokerTransactionMarkerPath {
    return Join-Path (Get-AmeBrokerProgramFilesX64) $script:AmeBrokerTransactionMarkerName
}

function Get-AmeBrokerParentPrestate {
    $parent = Get-AmeProductParentDirectory
    if (-not (Test-Path -LiteralPath $parent)) {
        return [pscustomobject]@{
            Path = $parent
            Existed = $false
            Sddl = $null
        }
    }
    $item = Get-Item -LiteralPath $parent -Force
    if (-not $item.PSIsContainer -or
        ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "The fixed Ame parent must be a physical directory"
    }
    return [pscustomobject]@{
        Path = $parent
        Existed = $true
        Sddl = [string](Get-Acl -LiteralPath $parent).Sddl
    }
}

function Assert-AmeBrokerParentPrestateRestoredFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate,
        [Parameter(Mandatory = $true)]
        [bool]$CurrentExists,
        [bool]$CurrentIsReparsePoint = $false,
        [AllowNull()]
        [string]$CurrentSddl
    )

    if ([bool]$Prestate.Existed) {
        if (-not $CurrentExists -or $CurrentIsReparsePoint -or
            -not ([string]$CurrentSddl).Equals(
                [string]$Prestate.Sddl,
                [StringComparison]::Ordinal
            )) {
            throw "The pre-existing Ame parent was not restored exactly"
        }
    } elseif ($CurrentExists) {
        throw "The transaction-created Ame parent was not removed"
    }
}

function Restore-AmeBrokerParentPrestate {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate
    )

    $expectedParent = Get-AmeProductParentDirectory
    if (-not ([string]$Prestate.Path).Equals(
        $expectedParent,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The transaction parent prestate is outside the fixed Ame tree"
    }
    if ([bool]$Prestate.Existed) {
        if (-not (Test-Path -LiteralPath $expectedParent -PathType Container)) {
            throw "The pre-existing Ame parent disappeared during rollback"
        }
        $item = Get-Item -LiteralPath $expectedParent -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "The pre-existing Ame parent became a reparse point"
        }
        $security = [Security.AccessControl.DirectorySecurity]::new()
        $security.SetSecurityDescriptorSddlForm([string]$Prestate.Sddl)
        Set-Acl -LiteralPath $expectedParent -AclObject $security
        Assert-AmeBrokerParentPrestateRestoredFacts `
            -Prestate $Prestate `
            -CurrentExists $true `
            -CurrentIsReparsePoint $false `
            -CurrentSddl ([string](Get-Acl -LiteralPath $expectedParent).Sddl)
        return
    }
    if (Test-Path -LiteralPath $expectedParent -PathType Container) {
        $item = Get-Item -LiteralPath $expectedParent -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "The transaction-owned Ame parent became a reparse point"
        }
        if (@(Get-ChildItem -LiteralPath $expectedParent -Force).Count -eq 0) {
            Remove-Item -LiteralPath $expectedParent
        }
    }
    Assert-AmeBrokerParentPrestateRestoredFacts `
        -Prestate $Prestate `
        -CurrentExists (Test-Path -LiteralPath $expectedParent)
}

function Assert-AmeBrokerNoFollowEntryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Entries,
        [Parameter(Mandatory = $true)]
        [string]$RootPath
    )

    $root = ConvertTo-AmeBrokerComparablePath -Path $RootPath
    $prefix = "$($root.TrimEnd([IO.Path]::DirectorySeparatorChar))$([IO.Path]::DirectorySeparatorChar)"
    foreach ($entry in $Entries) {
        $path = ConvertTo-AmeBrokerComparablePath -Path ([string]$entry.FullName)
        if (-not $path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "A no-follow tree entry escaped its admitted root"
        }
        if ([bool]$entry.IsReparsePoint) {
            throw "A no-follow tree operation refuses descendant reparse points"
        }
    }
}

function Get-AmeBrokerNoFollowTreeEntries {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootPath,
        [int]$MaximumEntries = 4096
    )

    $root = [IO.Path]::GetFullPath($RootPath)
    $rootItem = Get-Item -LiteralPath $root -Force
    if (-not $rootItem.PSIsContainer -or
        ($rootItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "A no-follow tree root must be a physical directory"
    }
    $pending = [Collections.Generic.Stack[string]]::new()
    $pending.Push($root)
    $entries = [Collections.Generic.List[object]]::new()
    while ($pending.Count -gt 0) {
        $directory = $pending.Pop()
        foreach ($entry in @(Get-ChildItem -LiteralPath $directory -Force)) {
            if ($entries.Count -ge $MaximumEntries) {
                throw "A no-follow tree operation exceeded its bounded entry count"
            }
            $fact = [pscustomobject]@{
                FullName = $entry.FullName
                IsDirectory = [bool]$entry.PSIsContainer
                IsReparsePoint = [bool](
                    $entry.Attributes -band [IO.FileAttributes]::ReparsePoint
                )
                Item = $entry
            }
            Assert-AmeBrokerNoFollowEntryFacts -Entries @($fact) -RootPath $root
            $entries.Add($fact)
            if ($entry.PSIsContainer) {
                $pending.Push($entry.FullName)
            }
        }
    }
    return @($entries)
}

function Remove-AmeBrokerOwnedTreeNoFollow {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedParent
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return
    }
    $root = [IO.Path]::GetFullPath($Path)
    $parent = [IO.Path]::GetDirectoryName($root)
    if (-not $parent.Equals(
        [IO.Path]::GetFullPath($ExpectedParent),
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "A transaction-owned tree is outside its fixed parent"
    }
    $entries = @(Get-AmeBrokerNoFollowTreeEntries -RootPath $root)
    foreach ($entry in @($entries | Sort-Object { ([string]$_.FullName).Length } -Descending)) {
        if ([bool]$entry.IsDirectory) {
            Remove-Item -LiteralPath $entry.FullName
        } else {
            Remove-Item -LiteralPath $entry.FullName -Force
        }
    }
    Remove-Item -LiteralPath $root
}

function Set-AmeBrokerTransactionMarkerAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $security = [Security.AccessControl.FileSecurity]::new()
    $administrators = [Security.Principal.SecurityIdentifier]::new(
        $script:AmeBrokerAdministratorsSid
    )
    $system = [Security.Principal.SecurityIdentifier]::new($script:AmeBrokerSystemSid)
    $security.SetOwner($administrators)
    $security.SetGroup($administrators)
    $security.SetAccessRuleProtection($true, $false)
    foreach ($identity in @($system, $administrators)) {
        $security.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
            $identity,
            [Security.AccessControl.FileSystemRights]::FullControl,
            [Security.AccessControl.AccessControlType]::Allow
        )) | Out-Null
    }
    Set-Acl -LiteralPath $Path -AclObject $security
    Assert-AmeBrokerTransactionMarkerAclFacts -Facts (Get-AmeBrokerAclFacts -Path $Path)
}

function Assert-AmeBrokerTransactionMarkerAclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts
    )

    if ([string]$Facts.OwnerSid -cne $script:AmeBrokerAdministratorsSid -or
        [string]$Facts.GroupSid -cne $script:AmeBrokerAdministratorsSid -or
        -not [bool]$Facts.AreAccessRulesProtected -or
        @($Facts.Rules).Count -ne 2) {
        throw "The broker transaction marker ACL is not protected exactly"
    }
    $expected = @($script:AmeBrokerSystemSid, $script:AmeBrokerAdministratorsSid)
    foreach ($rule in @($Facts.Rules)) {
        if ($expected -notcontains [string]$rule.Sid -or
            [uint64]$rule.Rights -ne [uint64]2032127 -or
            [string]$rule.AccessControlType -cne "Allow" -or
            [int]$rule.InheritanceFlags -ne 0 -or
            [int]$rule.PropagationFlags -ne 0 -or
            [bool]$rule.IsInherited) {
            throw "The broker transaction marker ACL grants unexpected authority"
        }
    }
}

function Assert-AmeBrokerAdoptableOrphanFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts
    )

    if (-not [bool]$Facts.DirectoryExists -or
        [bool]$Facts.ServiceExists -or
        [bool]$Facts.MarkerExists -or
        [bool]$Facts.BinaryExists -or
        [int]$Facts.EntryCount -ne 0) {
        throw "The pre-marker broker tree is not an empty unowned orphan"
    }
    Assert-AmeBrokerExactAclFacts `
        -Facts $Facts.Acl `
        -Kind "Directory" `
        -IncludeServiceSid $false
}

function Assert-AmeBrokerAdoptableOrphan {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $binary = Join-Path $InstallDirectory $script:AmeBrokerBinaryName
    $marker = Join-Path $InstallDirectory $script:AmeBrokerTransactionMarkerName
    $facts = [pscustomobject]@{
        DirectoryExists = Test-Path -LiteralPath $InstallDirectory -PathType Container
        ServiceExists = Test-AmeBrokerServiceExists
        MarkerExists = Test-Path -LiteralPath $marker
        BinaryExists = Test-Path -LiteralPath $binary
        EntryCount = if (Test-Path -LiteralPath $InstallDirectory -PathType Container) {
            @(Get-ChildItem -LiteralPath $InstallDirectory -Force).Count
        } else {
            -1
        }
        Acl = if (Test-Path -LiteralPath $InstallDirectory -PathType Container) {
            Get-AmeBrokerAclFacts -Path $InstallDirectory
        } else {
            $null
        }
    }
    Assert-AmeBrokerAdoptableOrphanFacts -Facts $facts
}

function New-AmeBrokerAbsentBinaryFacts {
    return [pscustomobject]@{
        Exists = $false
        Length = [int64]0
        Sha256 = $null
        FileIdentity = $null
        FinalPath = $null
    }
}

function Assert-AmeBrokerExpectedBinaryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts
    )

    $required = @("Exists", "Length", "Sha256", "FileIdentity", "FinalPath")
    foreach ($name in $required) {
        if ($null -eq $Facts.PSObject.Properties[$name]) {
            throw "A transaction binary fact is missing $name"
        }
    }
    if ([bool]$Facts.Exists) {
        if ([int64]$Facts.Length -lt 0 -or
            [string]$Facts.Sha256 -notmatch '^[0-9A-F]{64}$' -or
            [string]$Facts.FileIdentity -notmatch '^[0-9A-F]{8}:[0-9A-F]{16}$' -or
            [string]::IsNullOrWhiteSpace([string]$Facts.FinalPath) -or
            -not [IO.Path]::IsPathRooted([string]$Facts.FinalPath)) {
            throw "A present transaction binary fact is invalid"
        }
    } elseif ([int64]$Facts.Length -ne 0 -or
        -not [string]::IsNullOrEmpty([string]$Facts.Sha256) -or
        -not [string]::IsNullOrEmpty([string]$Facts.FileIdentity) -or
        -not [string]::IsNullOrEmpty([string]$Facts.FinalPath)) {
        throw "An absent transaction binary fact carries file identity"
    }
}

function Get-AmeBrokerTransactionBinaryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $full = [IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
        return New-AmeBrokerAbsentBinaryFacts
    }
    $item = Get-Item -LiteralPath $full -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "A transaction binary became a reparse point"
    }
    Assert-AmeBrokerPhysicalFile -ExpectedPath $full
    Assert-AmeBrokerBinary -Path $full -ExpectedPublisher $ExpectedPublisher | Out-Null
    $facts = [pscustomobject]@{
        Exists = $true
        Length = [int64]$item.Length
        Sha256 = (Get-FileHash -LiteralPath $full -Algorithm SHA256).Hash.ToUpperInvariant()
        FileIdentity = [Ame.JournalBrokerInstaller.NativeMethods]::GetFileIdentity($full)
        FinalPath = ConvertTo-AmeBrokerComparablePath -Path (
            [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($full)
        )
    }
    Assert-AmeBrokerExpectedBinaryFacts -Facts $facts
    return $facts
}

function Test-AmeBrokerBinaryFactsMatch {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Expected,
        [Parameter(Mandatory = $true)]
        [psobject]$Actual
    )

    Assert-AmeBrokerExpectedBinaryFacts -Facts $Expected
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Actual
    if ([bool]$Expected.Exists -ne [bool]$Actual.Exists) {
        return $false
    }
    if (-not [bool]$Expected.Exists) {
        return $true
    }
    return [int64]$Expected.Length -eq [int64]$Actual.Length -and
        [string]$Expected.Sha256 -ceq [string]$Actual.Sha256 -and
        [string]$Expected.FileIdentity -ceq [string]$Actual.FileIdentity
}

function Assert-AmeBrokerMoveIntentFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Intent,
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    $required = @(
        "OperationId", "SourcePath", "DestinationPath", "ExpectedSource",
        "ExpectedDestinationBefore", "NextPhase"
    )
    foreach ($name in $required) {
        if ($null -eq $Intent.PSObject.Properties[$name]) {
            throw "A transaction move intent is missing $name"
        }
    }
    if ([string]$Intent.OperationId -notmatch (
        '^' + [regex]::Escape([string]$State.TransactionId) + ':[1-9][0-9]{0,2}$'
    ) -or [string]$Intent.NextPhase -notmatch '^[a-z][a-z0-9_]{0,63}$') {
        throw "A transaction move intent identity is invalid"
    }
    $allowed = @(
        [string]$State.InstalledBinary,
        [string]$State.StagedBinary,
        [string]$State.BackupBinary
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    foreach ($path in @([string]$Intent.SourcePath, [string]$Intent.DestinationPath)) {
        if (-not @($allowed | Where-Object {
            ([IO.Path]::GetFullPath($path)).Equals(
                [IO.Path]::GetFullPath($_),
                [StringComparison]::OrdinalIgnoreCase
            )
        }).Count) {
            throw "A transaction move intent escaped its fixed leaves"
        }
    }
    if (([IO.Path]::GetFullPath([string]$Intent.SourcePath)).Equals(
        [IO.Path]::GetFullPath([string]$Intent.DestinationPath),
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "A transaction move intent source and destination are identical"
    }
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Intent.ExpectedSource
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Intent.ExpectedDestinationBefore
    if (-not [bool]$Intent.ExpectedSource.Exists) {
        throw "A transaction move intent lacks an existing expected source"
    }
}

function Assert-AmeBrokerMoveJournalFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if (@($State.AppliedOperations).Count -gt 16) {
        throw "The transaction move journal exceeded its bounded operation count"
    }
    $operationIds = [Collections.Generic.HashSet[string]]::new(
        [StringComparer]::Ordinal
    )
    foreach ($intent in @($State.AppliedOperations)) {
        Assert-AmeBrokerMoveIntentFacts -Intent $intent -State $State
        if (-not $operationIds.Add([string]$intent.OperationId)) {
            throw "The transaction move journal contains a duplicate operation"
        }
    }
    if ($null -ne $State.PendingIntent) {
        Assert-AmeBrokerMoveIntentFacts -Intent $State.PendingIntent -State $State
        if (-not $operationIds.Add([string]$State.PendingIntent.OperationId)) {
            throw "The pending move was already recorded as applied"
        }
    }
    foreach ($operationId in $operationIds) {
        $sequence = [int]$operationId.Substring($operationId.LastIndexOf(':') + 1)
        if ($sequence -ge [int]$State.NextOperationId) {
            throw "The transaction move journal sequence did not advance"
        }
    }
}

function Resolve-AmeBrokerMoveIntentFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Intent,
        [Parameter(Mandatory = $true)]
        [psobject]$SourceFacts,
        [Parameter(Mandatory = $true)]
        [psobject]$DestinationFacts
    )

    if ((Test-AmeBrokerBinaryFactsMatch `
        -Expected $Intent.ExpectedSource `
        -Actual $SourceFacts) -and
        (Test-AmeBrokerBinaryFactsMatch `
            -Expected $Intent.ExpectedDestinationBefore `
            -Actual $DestinationFacts)) {
        return "NotApplied"
    }
    if ((Test-AmeBrokerBinaryFactsMatch `
        -Expected (New-AmeBrokerAbsentBinaryFacts) `
        -Actual $SourceFacts) -and
        (Test-AmeBrokerBinaryFactsMatch `
            -Expected $Intent.ExpectedSource `
            -Actual $DestinationFacts)) {
        return "Applied"
    }
    throw "The pending broker move cannot be reconciled with physical file facts"
}

function Invoke-AmeBrokerWriteAheadMove {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [string]$SourcePath,
        [Parameter(Mandatory = $true)]
        [string]$DestinationPath,
        [Parameter(Mandatory = $true)]
        [psobject]$ExpectedSource,
        [Parameter(Mandatory = $true)]
        [psobject]$ExpectedDestinationBefore,
        [Parameter(Mandatory = $true)]
        [string]$NextPhase,
        [scriptblock]$StateWriter = { param([psobject]$Value) Write-AmeBrokerTransactionState -State $Value },
        [scriptblock]$FactProbe = {
            param([string]$Path, [string]$Publisher)
            Get-AmeBrokerTransactionBinaryFacts -Path $Path -ExpectedPublisher $Publisher
        },
        [scriptblock]$MoveAction = {
            param([string]$Source, [string]$Destination)
            [Ame.JournalBrokerInstaller.NativeMethods]::MoveFileNoReplaceWriteThrough(
                $Source,
                $Destination
            )
        },
        [scriptblock]$BeforePendingAction = {},
        [scriptblock]$AfterPendingAction = {},
        [scriptblock]$AfterMoveAction = {},
        [scriptblock]$AfterAppliedAction = {}
    )

    if ($null -ne $State.PendingIntent) {
        throw "A transaction cannot begin a second move while one is pending"
    }
    $intent = [pscustomobject]@{
        OperationId = "$($State.TransactionId):$($State.NextOperationId)"
        SourcePath = [IO.Path]::GetFullPath($SourcePath)
        DestinationPath = [IO.Path]::GetFullPath($DestinationPath)
        ExpectedSource = $ExpectedSource
        ExpectedDestinationBefore = $ExpectedDestinationBefore
        NextPhase = $NextPhase
    }
    Assert-AmeBrokerMoveIntentFacts -Intent $intent -State $State
    & $BeforePendingAction
    $State.PendingIntent = $intent
    $State.NextOperationId = [int]$State.NextOperationId + 1
    & $StateWriter $State
    & $AfterPendingAction
    $sourceBefore = & $FactProbe $intent.SourcePath ([string]$State.ExpectedPublisher)
    $destinationBefore = & $FactProbe `
        $intent.DestinationPath `
        ([string]$State.ExpectedPublisher)
    if ((Resolve-AmeBrokerMoveIntentFacts `
        -Intent $intent `
        -SourceFacts $sourceBefore `
        -DestinationFacts $destinationBefore) -cne "NotApplied") {
        throw "The broker move changed before its write-ahead action"
    }
    & $MoveAction $intent.SourcePath $intent.DestinationPath
    & $AfterMoveAction
    $sourceAfter = & $FactProbe $intent.SourcePath ([string]$State.ExpectedPublisher)
    $destinationAfter = & $FactProbe `
        $intent.DestinationPath `
        ([string]$State.ExpectedPublisher)
    if ((Resolve-AmeBrokerMoveIntentFacts `
        -Intent $intent `
        -SourceFacts $sourceAfter `
        -DestinationFacts $destinationAfter) -cne "Applied") {
        throw "The broker move did not reach its expected physical state"
    }
    $State.AppliedOperations = @($State.AppliedOperations) + @($intent)
    $State.PendingIntent = $null
    $State.Phase = $NextPhase
    & $StateWriter $State
    & $AfterAppliedAction
}

function Resolve-AmeBrokerPendingMove {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [scriptblock]$StateWriter = { param([psobject]$Value) Write-AmeBrokerTransactionState -State $Value },
        [scriptblock]$FactProbe = {
            param([string]$Path, [string]$Publisher)
            Get-AmeBrokerTransactionBinaryFacts -Path $Path -ExpectedPublisher $Publisher
        },
        [scriptblock]$MoveAction = {
            param([string]$Source, [string]$Destination)
            [Ame.JournalBrokerInstaller.NativeMethods]::MoveFileNoReplaceWriteThrough(
                $Source,
                $Destination
            )
        }
    )

    if ($null -eq $State.PendingIntent) {
        return
    }
    $intent = $State.PendingIntent
    $source = & $FactProbe $intent.SourcePath ([string]$State.ExpectedPublisher)
    $destination = & $FactProbe `
        $intent.DestinationPath `
        ([string]$State.ExpectedPublisher)
    $resolution = Resolve-AmeBrokerMoveIntentFacts `
        -Intent $intent `
        -SourceFacts $source `
        -DestinationFacts $destination
    if ($resolution -ceq "NotApplied") {
        & $MoveAction $intent.SourcePath $intent.DestinationPath
        $source = & $FactProbe $intent.SourcePath ([string]$State.ExpectedPublisher)
        $destination = & $FactProbe `
            $intent.DestinationPath `
            ([string]$State.ExpectedPublisher)
        $resolution = Resolve-AmeBrokerMoveIntentFacts `
            -Intent $intent `
            -SourceFacts $source `
            -DestinationFacts $destination
    }
    if ($resolution -cne "Applied") {
        throw "The pending broker move did not resolve to the applied state"
    }
    $State.AppliedOperations = @($State.AppliedOperations) + @($intent)
    $State.PendingIntent = $null
    $State.Phase = [string]$intent.NextPhase
    & $StateWriter $State
}

function New-AmeBrokerTransactionState {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("install", "repair", "upgrade", "uninstall")]
        [string]$Operation,
        [Parameter(Mandatory = $true)]
        [psobject]$ParentPrestate,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher,
        [string[]]$OwnedDirectories = @()
    )

    $process = [Diagnostics.Process]::GetCurrentProcess()
    $installedBinary = Join-Path `
        (Get-AmeBrokerInstallDirectory) `
        $script:AmeBrokerBinaryName
    $stagedBinary = switch ($Operation) {
        "install" { "$installedBinary.installing-$PID" }
        "repair" { "$installedBinary.upgrading-$PID" }
        "upgrade" { "$installedBinary.upgrading-$PID" }
        default { $null }
    }
    $backupBinary = if ($Operation -in @("repair", "upgrade")) {
        "$installedBinary.previous-$PID"
    } else {
        $null
    }
    return [pscustomobject]@{
        Schema = $script:AmeBrokerTransactionSchema
        TransactionId = [Guid]::NewGuid().ToString("N").ToUpperInvariant()
        Operation = $Operation
        OwnerProcessId = [int]$PID
        OwnerStartUtcTicks = [int64]$process.StartTime.ToUniversalTime().Ticks
        Phase = "prepared"
        NextOperationId = 1
        PendingIntent = $null
        AppliedOperations = @()
        InstalledBinary = $installedBinary
        StagedBinary = $stagedBinary
        BackupBinary = $backupBinary
        ApplicationDirectory = Get-AmeApplicationInstallDirectory
        OwnedDirectories = @($OwnedDirectories)
        ApplicationOwned = $false
        InstalledOwned = $false
        BackupCreated = $false
        ServiceCreated = $false
        ConfigurationChanged = $false
        PreviousIdentityManifest = $null
        NewIdentityManifest = $null
        ExpectedPreviousBinary = $null
        ExpectedNewBinary = $null
        ExpectedPublisher = $ExpectedPublisher
        WasRunning = $false
        ParentPath = [string]$ParentPrestate.Path
        ParentExisted = [bool]$ParentPrestate.Existed
        ParentSddl = $ParentPrestate.Sddl
        UninstallPendingOperation = $null
        UninstallProtectedTreeReady = $false
        UninstallServiceRemoved = $false
        UninstallBinaryRemoved = $false
        UninstallApplicationRemoved = $false
        UninstallInstallTreeRemoved = $false
        UninstallOwnedDirectoriesRemoved = $false
        UninstallParentRestored = $false
    }
}

function Invoke-AmeBrokerInitialTreeCreation {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [scriptblock]$CreateTreeAction,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        }
    )

    if ([string]$State.Operation -cne "install" -or
        [string]$State.Phase -cne "prepared") {
        throw "Only a prepared install may create the initial protected tree"
    }
    $State.Phase = "tree_creation_pending"
    & $StateWriter $State
    & $CreateTreeAction
    $State.Phase = "protected_tree_created"
    & $StateWriter $State
}

function Assert-AmeBrokerTransactionStateFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    $required = @(
        "Schema", "TransactionId", "Operation", "OwnerProcessId", "OwnerStartUtcTicks", "Phase",
        "NextOperationId", "PendingIntent", "AppliedOperations",
        "InstalledBinary", "ApplicationDirectory", "OwnedDirectories", "ApplicationOwned",
        "InstalledOwned", "BackupCreated", "ServiceCreated", "ConfigurationChanged",
        "PreviousIdentityManifest", "NewIdentityManifest", "ExpectedPreviousBinary",
        "ExpectedNewBinary", "ExpectedPublisher", "WasRunning", "ParentPath", "ParentExisted",
        "UninstallPendingOperation",
        "UninstallProtectedTreeReady", "UninstallServiceRemoved", "UninstallBinaryRemoved",
        "UninstallApplicationRemoved", "UninstallInstallTreeRemoved",
        "UninstallOwnedDirectoriesRemoved", "UninstallParentRestored"
    )
    foreach ($name in $required) {
        if ($null -eq $State.PSObject.Properties[$name]) {
            throw "The broker transaction marker is missing $name"
        }
    }
    if ([int]$State.Schema -ne $script:AmeBrokerTransactionSchema -or
        @("install", "repair", "upgrade", "uninstall") -notcontains [string]$State.Operation -or
        [string]$State.TransactionId -notmatch '^[0-9A-F]{32}$' -or
        [int]$State.OwnerProcessId -le 0 -or
        [int64]$State.OwnerStartUtcTicks -le 0 -or
        [int]$State.NextOperationId -le 0 -or
        [string]::IsNullOrWhiteSpace([string]$State.ExpectedPublisher) -or
        ([string]$State.ExpectedPublisher).Length -gt 512) {
        throw "The broker transaction marker identity is invalid"
    }
    $installDirectory = Get-AmeBrokerInstallDirectory
    $expectedInstalled = Join-Path $installDirectory $script:AmeBrokerBinaryName
    foreach ($fixedPathProperty in @("InstalledBinary", "ApplicationDirectory", "ParentPath")) {
        if ([string]::IsNullOrWhiteSpace([string]$State.$fixedPathProperty)) {
            throw "The broker transaction marker lacks fixed path $fixedPathProperty"
        }
    }
    try {
        $stateInstalledBinary = [IO.Path]::GetFullPath([string]$State.InstalledBinary)
        $stateApplicationDirectory = [IO.Path]::GetFullPath([string]$State.ApplicationDirectory)
        $stateParentPath = [IO.Path]::GetFullPath([string]$State.ParentPath)
    } catch [ArgumentException] {
        throw "The broker transaction marker contains an invalid fixed path"
    }
    if (-not $stateInstalledBinary.Equals(
        $expectedInstalled,
        [StringComparison]::OrdinalIgnoreCase
    ) -or -not $stateApplicationDirectory.Equals(
        (Get-AmeApplicationInstallDirectory),
        [StringComparison]::OrdinalIgnoreCase
    ) -or -not $stateParentPath.Equals(
        (Get-AmeProductParentDirectory),
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The broker transaction marker escaped the fixed install tree"
    }
    $pidText = [string][int]$State.OwnerProcessId
    $expectedStaged = switch ([string]$State.Operation) {
        "install" { "$expectedInstalled.installing-$pidText" }
        "repair" { "$expectedInstalled.upgrading-$pidText" }
        "upgrade" { "$expectedInstalled.upgrading-$pidText" }
        default { "" }
    }
    $expectedBackup = if ([string]$State.Operation -in @("repair", "upgrade")) {
        "$expectedInstalled.previous-$pidText"
    } else {
        ""
    }
    if ([string]$State.StagedBinary -cne $expectedStaged -or
        [string]$State.BackupBinary -cne $expectedBackup) {
        throw "The broker transaction marker contains an unowned staging path"
    }
    if ($null -ne $State.ExpectedPreviousBinary) {
        Assert-AmeBrokerExpectedBinaryFacts -Facts $State.ExpectedPreviousBinary
    }
    if ($null -ne $State.ExpectedNewBinary) {
        Assert-AmeBrokerExpectedBinaryFacts -Facts $State.ExpectedNewBinary
        if (-not [bool]$State.ExpectedNewBinary.Exists) {
            throw "The transaction marker expected new broker identity is absent"
        }
    }
    if (($null -ne $State.PendingIntent -or @($State.AppliedOperations).Count -gt 0) -and
        $null -eq $State.ExpectedNewBinary) {
        throw "A transaction with file moves lacks the expected new broker identity"
    }
    Assert-AmeBrokerMoveJournalFacts -State $State
    $allowedOwnedDirectories = @(
        Get-AmeProductParentDirectory
        Get-AmeBrokerInstallDirectory
        Get-AmeApplicationInstallDirectory
    )
    foreach ($directory in @($State.OwnedDirectories)) {
        if ([string]::IsNullOrWhiteSpace([string]$directory)) {
            throw "The broker transaction marker claims an empty owned directory"
        }
        try {
            $full = [IO.Path]::GetFullPath([string]$directory)
        } catch [ArgumentException] {
            throw "The broker transaction marker claims an invalid owned directory"
        }
        if (-not @($allowedOwnedDirectories | Where-Object {
            $full.Equals($_, [StringComparison]::OrdinalIgnoreCase)
        }).Count) {
            throw "The broker transaction marker claims an unowned directory"
        }
    }
    if ([bool]$State.ParentExisted) {
        if ([string]::IsNullOrWhiteSpace([string]$State.ParentSddl) -or
            ([string]$State.ParentSddl).Length -gt 8192) {
            throw "The broker transaction marker lacks the pre-existing parent ACL"
        }
        $probe = [Security.AccessControl.DirectorySecurity]::new()
        $probe.SetSecurityDescriptorSddlForm([string]$State.ParentSddl)
    } elseif (-not [string]::IsNullOrEmpty([string]$State.ParentSddl)) {
        throw "A transaction-created parent must not claim a prior ACL"
    }
}

function Write-AmeBrokerTransactionState {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    Assert-AmeBrokerTransactionStateFacts -State $State
    $marker = Get-AmeBrokerTransactionMarkerPath
    $directory = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($marker))
    $trustedProgramFiles = Get-AmeBrokerProgramFilesX64
    if (-not $directory.Equals(
        $trustedProgramFiles,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The broker transaction marker escaped Program Files x64"
    }
    Assert-AmeBrokerFixedInstallTree `
        -InstallDirectory (Get-AmeBrokerInstallDirectory) | Out-Null
    $temporary = "$marker.writing-$PID"
    if (Test-Path -LiteralPath $temporary) {
        throw "The broker transaction marker staging path already exists"
    }
    $json = $State | ConvertTo-Json -Depth 6 -Compress
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes($json)
    if ($bytes.Length -gt $script:AmeBrokerTransactionMaximumBytes) {
        throw "The broker transaction marker exceeds its bounded size"
    }
    $stream = [IO.FileStream]::new(
        $temporary,
        [IO.FileMode]::CreateNew,
        [IO.FileAccess]::Write,
        [IO.FileShare]::None,
        4096,
        [IO.FileOptions]::WriteThrough
    )
    try {
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush($true)
    } finally {
        $stream.Dispose()
    }
    try {
        Set-AmeBrokerTransactionMarkerAcl -Path $temporary
        [Ame.JournalBrokerInstaller.NativeMethods]::ReplaceFileWriteThrough($temporary, $marker)
        Assert-AmeBrokerTransactionMarkerAclFacts -Facts (Get-AmeBrokerAclFacts -Path $marker)
    } finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Read-AmeBrokerTransactionState {
    $marker = Get-AmeBrokerTransactionMarkerPath
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) {
        return $null
    }
    $item = Get-Item -LiteralPath $marker -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint -or
        [int64]$item.Length -gt $script:AmeBrokerTransactionMaximumBytes) {
        throw "The broker transaction marker is redirected or unbounded"
    }
    Assert-AmeBrokerTransactionMarkerAclFacts -Facts (Get-AmeBrokerAclFacts -Path $marker)
    $state = Get-Content -LiteralPath $marker -Raw -Encoding UTF8 | ConvertFrom-Json
    Assert-AmeBrokerTransactionStateFacts -State $state
    return $state
}

function Test-AmeBrokerTransactionOwnerActive {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [scriptblock]$ProcessProbe = {
            param([int]$ProcessId)
            try {
                $process = [Diagnostics.Process]::GetProcessById($ProcessId)
                try {
                    return [pscustomobject]@{
                        Exists = $true
                        StartUtcTicks = [int64]$process.StartTime.ToUniversalTime().Ticks
                    }
                } finally {
                    $process.Dispose()
                }
            } catch [ArgumentException] {
                return [pscustomobject]@{ Exists = $false; StartUtcTicks = [int64]0 }
            }
        }
    )

    $fact = & $ProcessProbe ([int]$State.OwnerProcessId)
    return [bool]$fact.Exists -and
        [int64]$fact.StartUtcTicks -eq [int64]$State.OwnerStartUtcTicks
}

function Remove-AmeBrokerTransactionMarker {
    $marker = Get-AmeBrokerTransactionMarkerPath
    if (Test-Path -LiteralPath $marker -PathType Leaf) {
        $item = Get-Item -LiteralPath $marker -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "The broker transaction marker became a reparse point"
        }
        Assert-AmeBrokerTransactionMarkerAclFacts -Facts (Get-AmeBrokerAclFacts -Path $marker)
        Remove-Item -LiteralPath $marker -Force
    }
}

function Get-AmeBrokerUninstallStepDefinitions {
    return @(
        [pscustomobject]@{
            Property = "UninstallProtectedTreeReady"
            Pending = "uninstall_tree_creation_pending"
            Completed = "uninstall_tree_ready"
        },
        [pscustomobject]@{
            Property = "UninstallServiceRemoved"
            Pending = "uninstall_service_removal_pending"
            Completed = "uninstall_service_removed"
        },
        [pscustomobject]@{
            Property = "UninstallBinaryRemoved"
            Pending = "uninstall_binary_removal_pending"
            Completed = "uninstall_binary_removed"
        },
        [pscustomobject]@{
            Property = "UninstallApplicationRemoved"
            Pending = "uninstall_application_removal_pending"
            Completed = "uninstall_application_removed"
        },
        [pscustomobject]@{
            Property = "UninstallInstallTreeRemoved"
            Pending = "uninstall_install_tree_removal_pending"
            Completed = "uninstall_install_tree_removed"
        },
        [pscustomobject]@{
            Property = "UninstallOwnedDirectoriesRemoved"
            Pending = "uninstall_owned_directories_removal_pending"
            Completed = "uninstall_owned_directories_removed"
        },
        [pscustomobject]@{
            Property = "UninstallParentRestored"
            Pending = "uninstall_parent_restore_pending"
            Completed = "uninstall_parent_restored"
        }
    )
}

function Assert-AmeBrokerUninstallProgressFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if ([string]$State.Operation -cne "uninstall") {
        throw "Only an uninstall transaction has uninstall progress"
    }
    $steps = @(Get-AmeBrokerUninstallStepDefinitions)
    $phase = [string]$State.Phase
    $pendingOperation = [string]$State.UninstallPendingOperation
    $expectedCompletedCount = $null
    $expectedPendingOperation = ""
    if ($phase -ceq "prepared") {
        $expectedCompletedCount = 0
    } elseif ($phase -ceq "committed") {
        $expectedCompletedCount = 4
    } elseif ($phase -ceq "final_cleanup_verified") {
        $expectedCompletedCount = $steps.Count
    } else {
        for ($index = 0; $index -lt $steps.Count; $index += 1) {
            if ($phase -ceq [string]$steps[$index].Pending) {
                $expectedCompletedCount = $index
                $expectedPendingOperation = [string]$steps[$index].Property
                break
            }
            if ($phase -ceq [string]$steps[$index].Completed) {
                $expectedCompletedCount = $index + 1
                break
            }
        }
    }
    if ($null -eq $expectedCompletedCount -or
        $pendingOperation -cne $expectedPendingOperation) {
        throw "The uninstall marker phase and pending operation are inconsistent"
    }
    for ($index = 0; $index -lt $steps.Count; $index += 1) {
        $expected = $index -lt $expectedCompletedCount
        $propertyName = [string]$steps[$index].Property
        $actual = [bool]$State.$propertyName
        if ($actual -ne $expected) {
            throw "The uninstall marker contains non-prefix completion state"
        }
    }
}

function Test-AmeBrokerUninstallPostconditionSuperseded {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [string]$CompletionProperty
    )

    if ($CompletionProperty -cne "UninstallProtectedTreeReady") {
        return $false
    }
    if ([bool]$State.UninstallInstallTreeRemoved) {
        return $true
    }
    return [string]$State.UninstallPendingOperation -in @(
        "UninstallInstallTreeRemoved",
        "UninstallOwnedDirectoriesRemoved",
        "UninstallParentRestored"
    )
}

function Resolve-AmeBrokerUninstallActionPair {
    param(
        [hashtable]$Overrides,
        [Parameter(Mandatory = $true)]
        [string]$CompletionProperty,
        [Parameter(Mandatory = $true)]
        [scriptblock]$DefaultMutationAction,
        [Parameter(Mandatory = $true)]
        [scriptblock]$DefaultVerifyAction
    )

    if ($null -eq $Overrides -or -not $Overrides.ContainsKey($CompletionProperty)) {
        return [pscustomobject]@{
            MutationAction = $DefaultMutationAction
            VerifyAction = $DefaultVerifyAction
        }
    }
    $override = $Overrides[$CompletionProperty]
    if ($null -eq $override -or
        $override.MutationAction -isnot [scriptblock] -or
        $override.VerifyAction -isnot [scriptblock]) {
        throw "The uninstall action override for $CompletionProperty is invalid"
    }
    return $override
}

function Get-AmeBrokerUninstallFaultAction {
    param(
        [string]$FaultOperation,
        [string]$FaultBoundary,
        [Parameter(Mandatory = $true)]
        [string]$CurrentOperation,
        [Parameter(Mandatory = $true)]
        [string]$CurrentBoundary,
        [scriptblock]$FaultAction = {}
    )

    if ($FaultOperation -ceq $CurrentOperation -and
        $FaultBoundary -ceq $CurrentBoundary) {
        return $FaultAction
    }
    return {}
}

function Invoke-AmeBrokerUninstallWriteAheadStep {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [ValidateSet(
            "UninstallProtectedTreeReady",
            "UninstallServiceRemoved",
            "UninstallBinaryRemoved",
            "UninstallApplicationRemoved",
            "UninstallInstallTreeRemoved",
            "UninstallOwnedDirectoriesRemoved",
            "UninstallParentRestored"
        )]
        [string]$CompletionProperty,
        [Parameter(Mandatory = $true)]
        [string]$PendingPhase,
        [Parameter(Mandatory = $true)]
        [string]$CompletedPhase,
        [Parameter(Mandatory = $true)]
        [scriptblock]$MutationAction,
        [Parameter(Mandatory = $true)]
        [scriptblock]$VerifyAction,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$BeforePendingAction = {},
        [scriptblock]$AfterPendingAction = {},
        [scriptblock]$AfterMutationAction = {},
        [scriptblock]$AfterCompletedAction = {},
        [scriptblock]$PostconditionSupersededAction = { $false }
    )

    if ([string]$State.Operation -cne "uninstall") {
        throw "Only an uninstall transaction may execute uninstall steps"
    }
    if ([bool]$State.$CompletionProperty) {
        if ([bool](& $PostconditionSupersededAction)) {
            return
        }
        & $VerifyAction
        return
    }
    & $BeforePendingAction
    $State.Phase = $PendingPhase
    $State.UninstallPendingOperation = $CompletionProperty
    & $StateWriter $State
    & $AfterPendingAction
    & $MutationAction
    & $AfterMutationAction
    & $VerifyAction
    $State.$CompletionProperty = $true
    $State.UninstallPendingOperation = $null
    $State.Phase = $CompletedPhase
    & $StateWriter $State
    & $AfterCompletedAction
}

function Assert-AmeBrokerParentPrestateRestored {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate
    )

    $parent = Get-AmeProductParentDirectory
    $exists = Test-Path -LiteralPath $parent -PathType Container
    $isReparsePoint = $false
    $sddl = $null
    if ($exists) {
        $item = Get-Item -LiteralPath $parent -Force
        $isReparsePoint = [bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
        if (-not $isReparsePoint) {
            $sddl = [string](Get-Acl -LiteralPath $parent).Sddl
        }
    }
    Assert-AmeBrokerParentPrestateRestoredFacts `
        -Prestate $Prestate `
        -CurrentExists $exists `
        -CurrentIsReparsePoint $isReparsePoint `
        -CurrentSddl $sddl
}

function Set-AmeBrokerUninstallCommitted {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$AfterCommittedAction = {}
    )

    Assert-AmeBrokerUninstallProgressFacts -State $State
    if ([string]$State.Operation -cne "uninstall" -or
        -not [bool]$State.UninstallProtectedTreeReady -or
        -not [bool]$State.UninstallServiceRemoved -or
        -not [bool]$State.UninstallBinaryRemoved -or
        -not [bool]$State.UninstallApplicationRemoved) {
        throw "Uninstall cannot commit before every core removal is verified"
    }
    if ([bool]$State.UninstallInstallTreeRemoved -or
        [bool]$State.UninstallOwnedDirectoriesRemoved -or
        [bool]$State.UninstallParentRestored -or
        [string]$State.UninstallPendingOperation -in @(
            "UninstallInstallTreeRemoved",
            "UninstallOwnedDirectoriesRemoved",
            "UninstallParentRestored"
        )) {
        return
    }
    $State.UninstallPendingOperation = $null
    $State.Phase = "committed"
    & $StateWriter $State
    & $AfterCommittedAction
}

function Remove-AmeBrokerCompletedUninstallMarker {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [scriptblock]$FinalVerifyAction,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$BeforeMarkerRemovalAction = {},
        [scriptblock]$MarkerRemovalAction = { Remove-AmeBrokerTransactionMarker },
        [scriptblock]$AfterMarkerRemovalAction = {}
    )

    Assert-AmeBrokerUninstallProgressFacts -State $State
    if ([string]$State.Operation -cne "uninstall" -or
        -not [bool]$State.UninstallProtectedTreeReady -or
        -not [bool]$State.UninstallServiceRemoved -or
        -not [bool]$State.UninstallBinaryRemoved -or
        -not [bool]$State.UninstallApplicationRemoved -or
        -not [bool]$State.UninstallInstallTreeRemoved -or
        -not [bool]$State.UninstallOwnedDirectoriesRemoved -or
        -not [bool]$State.UninstallParentRestored) {
        throw "Uninstall cannot remove its marker before every final action is verified"
    }
    & $FinalVerifyAction
    $State.UninstallPendingOperation = $null
    $State.Phase = "final_cleanup_verified"
    & $StateWriter $State
    & $BeforeMarkerRemovalAction
    & $MarkerRemovalAction
    & $AfterMarkerRemovalAction
}

function Complete-AmeBrokerUninstallTransaction {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$AfterCommittedAction = {},
        [scriptblock]$BeforeMarkerRemovalAction = {},
        [scriptblock]$MarkerRemovalAction = { Remove-AmeBrokerTransactionMarker },
        [scriptblock]$AfterMarkerRemovalAction = {},
        [hashtable]$ActionOverrides,
        [string]$FaultOperation = "",
        [string]$FaultBoundary = "",
        [scriptblock]$FaultAction = {}
    )

    if ([string]$State.Operation -cne "uninstall") {
        throw "Only an uninstall transaction may use uninstall completion"
    }
    Assert-AmeBrokerUninstallProgressFacts -State $State
    $knownOperations = @(Get-AmeBrokerUninstallStepDefinitions | ForEach-Object {
        [string]$_.Property
    })
    if (([string]::IsNullOrEmpty($FaultOperation)) -ne
        ([string]::IsNullOrEmpty($FaultBoundary)) -or
        (-not [string]::IsNullOrEmpty($FaultOperation) -and
            ($FaultOperation -notin $knownOperations -or
                $FaultBoundary -notin @("BeforeMutation", "AfterMutation", "AfterCompletion")))) {
        throw "The uninstall transaction fault boundary is invalid"
    }
    $installDirectory = Get-AmeBrokerInstallDirectory
    $applicationDirectory = [string]$State.ApplicationDirectory
    $installedBinary = [string]$State.InstalledBinary
    $parentPrestate = [pscustomobject]@{
        Path = [string]$State.ParentPath
        Existed = [bool]$State.ParentExisted
        Sddl = $State.ParentSddl
    }
    $treeMutation = {
        if (-not (Test-Path -LiteralPath $installDirectory -PathType Container)) {
            $created = New-AmeBrokerProtectedInstallTree -InstallDirectory $installDirectory
            $unexpected = @($created.CreatedDirectories | Where-Object {
                $createdPath = [IO.Path]::GetFullPath([string]$_)
                -not @($State.OwnedDirectories | Where-Object {
                    $createdPath.Equals(
                        [IO.Path]::GetFullPath([string]$_),
                        [StringComparison]::OrdinalIgnoreCase
                    )
                }).Count
            })
            if ($unexpected.Count -gt 0) {
                throw "Uninstall created a directory that was absent from its write-ahead ownership"
            }
        }
    }
    $treeVerify = {
        Assert-AmeBrokerFixedInstallTree `
            -InstallDirectory $installDirectory `
            -RequireInstallDirectory | Out-Null
    }
    $treeActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallProtectedTreeReady `
        -DefaultMutationAction $treeMutation `
        -DefaultVerifyAction $treeVerify
    $treeSuperseded = {
        Test-AmeBrokerUninstallPostconditionSuperseded `
            -State $State `
            -CompletionProperty UninstallProtectedTreeReady
    }
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallProtectedTreeReady `
        -PendingPhase uninstall_tree_creation_pending `
        -CompletedPhase uninstall_tree_ready `
        -MutationAction $treeActions.MutationAction `
        -VerifyAction $treeActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallProtectedTreeReady -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallProtectedTreeReady -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallProtectedTreeReady -CurrentBoundary AfterCompletion -FaultAction $FaultAction) `
        -PostconditionSupersededAction $treeSuperseded

    $serviceMutation = {
        if (Test-AmeBrokerServiceExists) {
            Remove-AmeBrokerServiceBounded
        }
    }
    $serviceVerify = {
        if (Test-AmeBrokerServiceExists) {
            throw "The broker service remains after uninstall removal"
        }
    }
    $serviceActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallServiceRemoved `
        -DefaultMutationAction $serviceMutation `
        -DefaultVerifyAction $serviceVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallServiceRemoved `
        -PendingPhase uninstall_service_removal_pending `
        -CompletedPhase uninstall_service_removed `
        -MutationAction $serviceActions.MutationAction `
        -VerifyAction $serviceActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallServiceRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallServiceRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallServiceRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $binaryMutation = {
        Remove-AmeBrokerOwnedLeafNoFollow `
            -Path $installedBinary `
            -AllowedPaths @($installedBinary)
    }
    $binaryVerify = {
        if (Test-Path -LiteralPath $installedBinary) {
            throw "The broker binary remains after uninstall removal"
        }
    }
    $binaryActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallBinaryRemoved `
        -DefaultMutationAction $binaryMutation `
        -DefaultVerifyAction $binaryVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallBinaryRemoved `
        -PendingPhase uninstall_binary_removal_pending `
        -CompletedPhase uninstall_binary_removed `
        -MutationAction $binaryActions.MutationAction `
        -VerifyAction $binaryActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallBinaryRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallBinaryRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallBinaryRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $applicationMutation = {
        Remove-AmeBrokerOwnedTreeNoFollow `
            -Path $applicationDirectory `
            -ExpectedParent (Get-AmeProductParentDirectory)
    }
    $applicationVerify = {
        if (Test-Path -LiteralPath $applicationDirectory) {
            throw "The installed Application bundle remains after uninstall removal"
        }
    }
    $applicationActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallApplicationRemoved `
        -DefaultMutationAction $applicationMutation `
        -DefaultVerifyAction $applicationVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallApplicationRemoved `
        -PendingPhase uninstall_application_removal_pending `
        -CompletedPhase uninstall_application_removed `
        -MutationAction $applicationActions.MutationAction `
        -VerifyAction $applicationActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallApplicationRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallApplicationRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallApplicationRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    Set-AmeBrokerUninstallCommitted `
        -State $State `
        -StateWriter $StateWriter `
        -AfterCommittedAction $AfterCommittedAction

    $installTreeMutation = {
        if (Test-Path -LiteralPath $installDirectory -PathType Container) {
            Assert-AmeBrokerFixedInstallTree `
                -InstallDirectory $installDirectory `
                -RequireInstallDirectory | Out-Null
            if (@(Get-ChildItem -LiteralPath $installDirectory -Force).Count -ne 0) {
                throw "The broker install directory contains an unowned uninstall entry"
            }
            Remove-Item -LiteralPath $installDirectory
        }
    }
    $installTreeVerify = {
        if (Test-Path -LiteralPath $installDirectory) {
            throw "The broker install directory remains after uninstall cleanup"
        }
    }
    $installTreeActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallInstallTreeRemoved `
        -DefaultMutationAction $installTreeMutation `
        -DefaultVerifyAction $installTreeVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallInstallTreeRemoved `
        -PendingPhase uninstall_install_tree_removal_pending `
        -CompletedPhase uninstall_install_tree_removed `
        -MutationAction $installTreeActions.MutationAction `
        -VerifyAction $installTreeActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallInstallTreeRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallInstallTreeRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallInstallTreeRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $ownedDirectoriesMutation = {
        $failures = @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories @($State.OwnedDirectories | Sort-Object Length -Descending))
        if ($failures.Count -gt 0) {
            throw "Uninstall owned-directory cleanup failed: " + ($failures -join "; ")
        }
    }
    $ownedDirectoriesVerify = {
        foreach ($directory in @($State.OwnedDirectories)) {
            if (Test-Path -LiteralPath ([string]$directory)) {
                throw "A transaction-created uninstall ancestor remains"
            }
        }
    }
    $ownedDirectoriesActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallOwnedDirectoriesRemoved `
        -DefaultMutationAction $ownedDirectoriesMutation `
        -DefaultVerifyAction $ownedDirectoriesVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallOwnedDirectoriesRemoved `
        -PendingPhase uninstall_owned_directories_removal_pending `
        -CompletedPhase uninstall_owned_directories_removed `
        -MutationAction $ownedDirectoriesActions.MutationAction `
        -VerifyAction $ownedDirectoriesActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallOwnedDirectoriesRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallOwnedDirectoriesRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallOwnedDirectoriesRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $parentMutation = {
        Restore-AmeBrokerParentPrestate -Prestate $parentPrestate
    }
    $parentVerify = {
        Assert-AmeBrokerParentPrestateRestored -Prestate $parentPrestate
    }
    $parentActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallParentRestored `
        -DefaultMutationAction $parentMutation `
        -DefaultVerifyAction $parentVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallParentRestored `
        -PendingPhase uninstall_parent_restore_pending `
        -CompletedPhase uninstall_parent_restored `
        -MutationAction $parentActions.MutationAction `
        -VerifyAction $parentActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallParentRestored -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallParentRestored -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallParentRestored -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $finalVerify = {
        & $serviceActions.VerifyAction
        & $binaryActions.VerifyAction
        & $applicationActions.VerifyAction
        & $installTreeActions.VerifyAction
        & $ownedDirectoriesActions.VerifyAction
        & $parentActions.VerifyAction
    }
    Remove-AmeBrokerCompletedUninstallMarker `
        -State $State `
        -FinalVerifyAction $finalVerify `
        -StateWriter $StateWriter `
        -BeforeMarkerRemovalAction $BeforeMarkerRemovalAction `
        -MarkerRemovalAction $MarkerRemovalAction `
        -AfterMarkerRemovalAction $AfterMarkerRemovalAction
}

function Assert-AmeBrokerPlatformFacts {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$IsWindowsPlatform,
        [Parameter(Mandatory = $true)]
        [bool]$Is64BitOperatingSystem,
        [Parameter(Mandatory = $true)]
        [bool]$Is64BitProcess,
        [Parameter(Mandatory = $true)]
        [int]$BuildNumber,
        [Parameter(Mandatory = $true)]
        [int]$ProductType
    )

    if (-not $IsWindowsPlatform -or -not $Is64BitOperatingSystem -or -not $Is64BitProcess) {
        throw "The journal broker supports only Windows 11 x64"
    }
    if ($BuildNumber -lt 22000 -or $ProductType -ne 1) {
        throw "The journal broker supports only Windows 11 client x64"
    }
}

function Assert-AmeBrokerPlatform {
    $isWindowsPlatform = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )
    $operatingSystem = Get-CimInstance -ClassName Win32_OperatingSystem
    Assert-AmeBrokerPlatformFacts `
        -IsWindowsPlatform $isWindowsPlatform `
        -Is64BitOperatingSystem ([Environment]::Is64BitOperatingSystem) `
        -Is64BitProcess ([Environment]::Is64BitProcess) `
        -BuildNumber ([int]$operatingSystem.BuildNumber) `
        -ProductType ([int]$operatingSystem.ProductType)
}

function Assert-AmeBrokerAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "Journal broker lifecycle operations require an elevated administrator token"
    }
}

function Assert-AmeBrokerInstallDirectoryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CandidatePath,
        [Parameter(Mandatory = $true)]
        [string]$TrustedProgramFilesPath
    )

    $expected = [System.IO.Path]::GetFullPath(
        (Join-Path $TrustedProgramFilesPath $script:AmeBrokerRelativeInstallDirectory)
    )
    $candidate = [System.IO.Path]::GetFullPath($CandidatePath)
    if (-not $candidate.Equals($expected, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "The broker install directory must be the fixed Ame-owned Program Files location"
    }
}

function Assert-AmeApplicationInstallDirectoryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CandidatePath,
        [Parameter(Mandatory = $true)]
        [string]$TrustedProgramFilesPath
    )

    $expected = [System.IO.Path]::GetFullPath(
        (Join-Path $TrustedProgramFilesPath $script:AmeApplicationRelativeInstallDirectory)
    )
    $candidate = [System.IO.Path]::GetFullPath($CandidatePath)
    if (-not $candidate.Equals($expected, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "The Ame application directory must be the fixed Program Files location"
    }
}

function ConvertTo-AmeBrokerComparablePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $normalized = $Path
    if ($normalized.StartsWith("\\?\UNC\", [System.StringComparison]::OrdinalIgnoreCase)) {
        $normalized = "\\" + $normalized.Substring(8)
    } elseif ($normalized.StartsWith("\\?\", [System.StringComparison]::OrdinalIgnoreCase)) {
        $normalized = $normalized.Substring(4)
    }
    $fullPath = [System.IO.Path]::GetFullPath($normalized)
    $root = [System.IO.Path]::GetPathRoot($fullPath)
    if ($fullPath.Length -gt $root.Length) {
        return $fullPath.TrimEnd([System.IO.Path]::DirectorySeparatorChar)
    }
    return $root
}

function ConvertTo-AmeBrokerAccessMask {
    param(
        [Parameter(Mandatory = $true)]
        [Security.AccessControl.FileSystemRights]$Rights
    )

    $bytes = [BitConverter]::GetBytes([int32]$Rights)
    return [uint64]([BitConverter]::ToUInt32($bytes, 0))
}

function Get-AmeBrokerAclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $acl = Get-Acl -LiteralPath $Path
    try {
        $ownerSid = ([Security.Principal.NTAccount]::new(
            [string]$acl.Owner
        )).Translate([Security.Principal.SecurityIdentifier]).Value
    } catch {
        if ([string]$acl.Owner -match '^S-\d-') {
            $ownerSid = [string]$acl.Owner
        } else {
            throw "The owner of $Path cannot be resolved to a SID"
        }
    }
    try {
        $groupSid = ([Security.Principal.NTAccount]::new(
            [string]$acl.Group
        )).Translate([Security.Principal.SecurityIdentifier]).Value
    } catch {
        if ([string]$acl.Group -match '^S-\d-') {
            $groupSid = [string]$acl.Group
        } else {
            throw "The group of $Path cannot be resolved to a SID"
        }
    }
    $rules = @(
        foreach ($rule in $acl.Access) {
            $identityText = [string]$rule.IdentityReference
            $sid = $null
            try {
                $sid = $rule.IdentityReference.Translate(
                    [Security.Principal.SecurityIdentifier]
                ).Value
            } catch {
                $sid = $null
            }
            [pscustomobject]@{
                Sid = $sid
                Identity = $identityText
                Rights = ConvertTo-AmeBrokerAccessMask -Rights $rule.FileSystemRights
                AccessControlType = [string]$rule.AccessControlType
                InheritanceFlags = [int]$rule.InheritanceFlags
                PropagationFlags = [int]$rule.PropagationFlags
                IsInherited = [bool]$rule.IsInherited
            }
        }
    )
    return [pscustomobject]@{
        OwnerSid = $ownerSid
        GroupSid = $groupSid
        AreAccessRulesProtected = [bool]$acl.AreAccessRulesProtected
        Rules = $rules
    }
}

function Get-AmeBrokerDirectoryIdentityFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPath,
        [Parameter(Mandatory = $true)]
        [bool]$IsVolumeRoot
    )

    $item = Get-Item -LiteralPath $ExpectedPath -Force
    $finalPath = [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath(
        $item.FullName
    )
    return [pscustomobject]@{
        ExpectedPath = ConvertTo-AmeBrokerComparablePath -Path $item.FullName
        FinalPath = ConvertTo-AmeBrokerComparablePath -Path $finalPath
        IsDirectory = [bool]$item.PSIsContainer
        IsReparsePoint = [bool](
            $item.Attributes -band [System.IO.FileAttributes]::ReparsePoint
        )
        IsVolumeRoot = $IsVolumeRoot
        Acl = Get-AmeBrokerAclFacts -Path $item.FullName
    }
}

function Get-AmeBrokerInstallTreeFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $trustedProgramFilesPath = Get-AmeBrokerProgramFilesX64
    Assert-AmeBrokerInstallDirectoryFacts `
        -CandidatePath $InstallDirectory `
        -TrustedProgramFilesPath $trustedProgramFilesPath
    $candidate = ConvertTo-AmeBrokerComparablePath -Path $InstallDirectory
    $root = [System.IO.Path]::GetPathRoot($candidate)
    if ([string]::IsNullOrWhiteSpace($root) -or $root.StartsWith("\\")) {
        throw "The broker install directory must be on a local Windows volume"
    }
    $expectedPaths = @($root)
    $currentPath = $root
    $relativePath = $candidate.Substring($root.Length)
    foreach ($component in $relativePath.Split(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.StringSplitOptions]::RemoveEmptyEntries
    )) {
        $currentPath = Join-Path $currentPath $component
        $expectedPaths += $currentPath
    }

    $entries = @()
    $firstMissingIndex = $expectedPaths.Count
    for ($index = 0; $index -lt $expectedPaths.Count; $index += 1) {
        try {
            $entry = Get-AmeBrokerDirectoryIdentityFacts `
                -ExpectedPath $expectedPaths[$index] `
                -IsVolumeRoot ($index -eq 0)
        } catch [System.Management.Automation.ItemNotFoundException] {
            $firstMissingIndex = $index
            break
        }
        $entries += $entry
    }
    return [pscustomobject]@{
        TrustedProgramFilesPath = $trustedProgramFilesPath
        InstallDirectory = $candidate
        ExpectedPaths = $expectedPaths
        ExistingEntries = $entries
        FirstMissingIndex = $firstMissingIndex
    }
}

function Get-AmeApplicationTreeFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $trustedProgramFilesPath = Get-AmeBrokerProgramFilesX64
    Assert-AmeApplicationInstallDirectoryFacts `
        -CandidatePath $InstallDirectory `
        -TrustedProgramFilesPath $trustedProgramFilesPath
    $candidate = ConvertTo-AmeBrokerComparablePath -Path $InstallDirectory
    $root = [System.IO.Path]::GetPathRoot($candidate)
    if ([string]::IsNullOrWhiteSpace($root) -or $root.StartsWith("\")) {
        throw "The Ame application directory must be on a local Windows volume"
    }
    $expectedPaths = @($root)
    $currentPath = $root
    $relativePath = $candidate.Substring($root.Length)
    foreach ($component in $relativePath.Split(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.StringSplitOptions]::RemoveEmptyEntries
    )) {
        $currentPath = Join-Path $currentPath $component
        $expectedPaths += $currentPath
    }

    $entries = @()
    $firstMissingIndex = $expectedPaths.Count
    for ($index = 0; $index -lt $expectedPaths.Count; $index += 1) {
        try {
            $entry = Get-AmeBrokerDirectoryIdentityFacts `
                -ExpectedPath $expectedPaths[$index] `
                -IsVolumeRoot ($index -eq 0)
        } catch [System.Management.Automation.ItemNotFoundException] {
            $firstMissingIndex = $index
            break
        }
        $entries += $entry
    }
    return [pscustomobject]@{
        TrustedProgramFilesPath = $trustedProgramFilesPath
        InstallDirectory = $candidate
        ExpectedPaths = $expectedPaths
        ExistingEntries = $entries
        FirstMissingIndex = $firstMissingIndex
    }
}

function Assert-AmeBrokerInstallTreeFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts,
        [switch]$RequireInstallDirectory
    )

    $trustedOwnerSids = @(
        $script:AmeBrokerSystemSid,
        $script:AmeBrokerAdministratorsSid,
        $script:AmeBrokerTrustedInstallerSid
    )
    $replaceMask = [uint64]0x000D0000
    $directoryWriteMask = [uint64]0x500D0156
    for ($index = 0; $index -lt $Facts.ExistingEntries.Count; $index += 1) {
        $entry = $Facts.ExistingEntries[$index]
        $expectedPath = ConvertTo-AmeBrokerComparablePath -Path $entry.ExpectedPath
        $finalPath = ConvertTo-AmeBrokerComparablePath -Path $entry.FinalPath
        if (-not $expectedPath.Equals(
            $finalPath,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "The broker install ancestor resolves outside its fixed physical path"
        }
        if (-not $entry.IsDirectory -or $entry.IsReparsePoint) {
            throw "The broker install ancestor must be a non-reparse directory"
        }
        if ($trustedOwnerSids -notcontains [string]$entry.Acl.OwnerSid) {
            throw "The broker install ancestor has an untrusted owner"
        }

        $dangerousMask = if ($entry.IsVolumeRoot) {
            $replaceMask -bor [uint64]0x40
        } else {
            $directoryWriteMask
        }
        foreach ($rule in $entry.Acl.Rules) {
            $isInheritOnly = ($rule.PropagationFlags -band 2) -ne 0
            if (
                $rule.AccessControlType -cne "Allow" -or
                $isInheritOnly -or
                $trustedOwnerSids -contains [string]$rule.Sid
            ) {
                continue
            }
            if (([uint64]$rule.Rights -band $dangerousMask) -ne 0) {
                throw "The broker install ancestor grants an untrusted write or replacement right"
            }
        }
    }
    if (
        $RequireInstallDirectory -and
        $Facts.FirstMissingIndex -lt $Facts.ExpectedPaths.Count
    ) {
        throw "The fixed broker install directory is missing"
    }
}

function Assert-AmeBrokerFixedInstallTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [switch]$RequireInstallDirectory
    )

    $facts = Get-AmeBrokerInstallTreeFacts -InstallDirectory $InstallDirectory
    Assert-AmeBrokerInstallTreeFacts `
        -Facts $facts `
        -RequireInstallDirectory:$RequireInstallDirectory
    return $facts
}

function Assert-AmeApplicationFixedInstallTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [switch]$RequireInstallDirectory
    )

    $facts = Get-AmeApplicationTreeFacts -InstallDirectory $InstallDirectory
    Assert-AmeBrokerInstallTreeFacts `
        -Facts $facts `
        -RequireInstallDirectory:$RequireInstallDirectory
    return $facts
}

function New-AmeBrokerProtectedInstallTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $createdDirectories = @()
    try {
        $facts = Assert-AmeBrokerFixedInstallTree -InstallDirectory $InstallDirectory
        for (
            $index = $facts.FirstMissingIndex;
            $index -lt $facts.ExpectedPaths.Count;
            $index += 1
        ) {
            $directory = $facts.ExpectedPaths[$index]
            [Ame.JournalBrokerInstaller.NativeMethods]::CreateDirectoryWithSddl(
                $directory,
                $script:AmeBrokerProtectedDirectorySddl
            )
            $createdDirectories += $directory
            Assert-AmeBrokerExactAclFacts `
                -Facts (Get-AmeBrokerAclFacts -Path $directory) `
                -Kind "Directory" `
                -IncludeServiceSid $false
            $facts = Assert-AmeBrokerFixedInstallTree -InstallDirectory $InstallDirectory
        }
        Assert-AmeBrokerFixedInstallTree `
            -InstallDirectory $InstallDirectory `
            -RequireInstallDirectory | Out-Null
    } catch {
        $originalError = $_
        $rollbackFailures = @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories $createdDirectories)
        Throw-AmeBrokerTransactionFailure `
            -OriginalError $originalError `
            -RollbackFailures $rollbackFailures
    }
    return [pscustomobject]@{
        InstallDirectory = $InstallDirectory
        CreatedDirectories = $createdDirectories
    }
}

function Remove-AmeCreatedEmptyDirectories {
    param(
        [string[]]$CreatedDirectories = @()
    )

    $actions = @()
    foreach ($directory in $CreatedDirectories) {
        $capturedDirectory = $directory
        $actions += New-AmeBrokerRollbackAction `
            -Name "remove owned directory $capturedDirectory" `
            -Action ({
                if (
                    (Test-Path -LiteralPath $capturedDirectory -PathType Container) -and
                    @(Get-ChildItem -LiteralPath $capturedDirectory -Force).Count -eq 0
                ) {
                    Remove-Item -LiteralPath $capturedDirectory
                }
            }.GetNewClosure())
    }
    return Invoke-AmeBrokerRollbackActions -Actions $actions
}

function Assert-AmeBrokerSignatureFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Status,
        [AllowNull()]
        [string]$SignerSubject,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    if ($Status -cne "Valid") {
        throw "The broker binary must have a valid Authenticode signature"
    }
    if ([string]::IsNullOrWhiteSpace($ExpectedPublisher)) {
        throw "The expected broker publisher must be explicit"
    }
    if ([string]::IsNullOrWhiteSpace($SignerSubject) -or -not $SignerSubject.Equals(
        $ExpectedPublisher,
        [System.StringComparison]::Ordinal
    )) {
        throw "The broker signer does not match the expected publisher"
    }
}

function Assert-AmeBrokerMachineFacts {
    param(
        [Parameter(Mandatory = $true)]
        [UInt16]$Machine
    )

    if ($Machine -ne 0x8664) {
        throw "The broker binary must be a Windows x64 PE image"
    }
}

function Get-AmeBrokerPeMachine {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::Read
    )
    try {
        if ($stream.Length -lt 64) {
            throw "The broker binary is not a complete PE image"
        }
        $reader = [System.IO.BinaryReader]::new($stream)
        try {
            if ($reader.ReadUInt16() -ne 0x5A4D) {
                throw "The broker binary is missing the DOS PE signature"
            }
            $stream.Position = 0x3C
            $peOffset = $reader.ReadUInt32()
            if ($peOffset -gt $stream.Length - 6) {
                throw "The broker PE header offset is invalid"
            }
            $stream.Position = $peOffset
            if ($reader.ReadUInt32() -ne 0x00004550) {
                throw "The broker binary is missing the NT PE signature"
            }
            return $reader.ReadUInt16()
        } finally {
            $reader.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function Assert-AmeSignedX64Binary {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) {
        throw "The broker binary was not found: $resolvedPath"
    }
    Assert-AmeBrokerMachineFacts -Machine (Get-AmeBrokerPeMachine -Path $resolvedPath)
    $signature = Get-AuthenticodeSignature -LiteralPath $resolvedPath
    $signerSubject = if ($null -eq $signature.SignerCertificate) {
        $null
    } else {
        $signature.SignerCertificate.Subject
    }
    Assert-AmeBrokerSignatureFacts `
        -Status ([string]$signature.Status) `
        -SignerSubject $signerSubject `
        -ExpectedPublisher $ExpectedPublisher
    return $resolvedPath
}

function Assert-AmeBrokerBinary {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $resolvedPath = Assert-AmeSignedX64Binary `
        -Path $Path `
        -ExpectedPublisher $ExpectedPublisher
    Get-AmeBrokerProtocolFacts -Path $resolvedPath | Out-Null
    return $resolvedPath
}

function ConvertTo-AmeBrokerUpperHex {
    param(
        [Parameter(Mandatory = $true)]
        [byte[]]$Bytes
    )

    return ([BitConverter]::ToString($Bytes)).Replace("-", "")
}

function Get-AmeBrokerProtocolFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    $output = @(& $resolvedPath "--protocol-info" 2>&1)
    $exitCode = $LASTEXITCODE
    return Assert-AmeBrokerProtocolOutputFacts -Output $output -ExitCode $exitCode
}

function Assert-AmeBrokerProtocolOutputFacts {
    param(
        [AllowEmptyCollection()]
        [object[]]$Output,
        [Parameter(Mandatory = $true)]
        [int]$ExitCode
    )

    if ($exitCode -ne 0 -or $output.Count -ne 1) {
        throw "The broker protocol probe did not produce one successful record"
    }
    $match = [regex]::Match(
        [string]$output[0],
        '^binary=cedarflake_ame_journal_broker protocol=(\d+) max_frame=(\d+)$',
        [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (-not $match.Success) {
        throw "The broker protocol probe output is malformed"
    }
    $facts = [pscustomobject]@{
        Protocol = [int]$match.Groups[1].Value
        MaximumFrameBytes = [int]$match.Groups[2].Value
    }
    if (
        $facts.Protocol -ne $script:AmeBrokerProtocolVersion -or
        $facts.MaximumFrameBytes -ne $script:AmeBrokerMaximumFrameBytes
    ) {
        throw "The broker protocol probe is incompatible with this installer"
    }
    return $facts
}

function New-AmeBrokerUpgradeOwnership {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$InstalledBinaryExists
    )

    return [pscustomobject]@{
        HadPreviousBinary = $InstalledBinaryExists
        BackupCreated = $false
    }
}

function Remove-AmeBrokerTransactionBackup {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Ownership,
        [Parameter(Mandatory = $true)]
        [string]$BackupPath,
        [scriptblock]$PathExistsProbe = {
            param([string]$Path)
            Test-Path -LiteralPath $Path -PathType Leaf
        },
        [scriptblock]$RemoveAction = {
            param([string]$Path)
            Remove-Item -LiteralPath $Path -Force
        }
    )

    if (-not [bool]$Ownership.BackupCreated) {
        return
    }
    if (-not (& $PathExistsProbe $BackupPath)) {
        throw "The transaction-owned broker backup is unavailable"
    }
    & $RemoveAction $BackupPath
    $Ownership.BackupCreated = $false
}

function New-AmeBrokerIdentityManifest {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$ClientBinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $resolvedPath = Assert-AmeBrokerBinary `
        -Path $BinaryPath `
        -ExpectedPublisher $ExpectedPublisher
    $protocol = Get-AmeBrokerProtocolFacts -Path $resolvedPath
    $signature = Get-AuthenticodeSignature -LiteralPath $resolvedPath
    if ($null -eq $signature.SignerCertificate) {
        throw "The broker signer certificate is unavailable"
    }
    $sha256 = (Get-FileHash -LiteralPath $resolvedPath -Algorithm SHA256).Hash.ToUpperInvariant()
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        $signerSha256 = ConvertTo-AmeBrokerUpperHex `
            -Bytes ($hasher.ComputeHash($signature.SignerCertificate.RawData))
    } finally {
        $hasher.Dispose()
    }
    $publisherBytes = [Text.Encoding]::Unicode.GetBytes($ExpectedPublisher)
    $publisherHex = ConvertTo-AmeBrokerUpperHex -Bytes $publisherBytes
    $expectedClientPath = Get-AmeApplicationInstalledBinaryPath
    $resolvedClientPath = [System.IO.Path]::GetFullPath($ClientBinaryPath)
    if (-not $resolvedClientPath.Equals(
        $expectedClientPath,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The client identity must use the fixed installed Ame executable"
    }
    Assert-AmeBrokerPhysicalFile -ExpectedPath $resolvedClientPath
    Assert-AmeSignedX64Binary `
        -Path $resolvedClientPath `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    $clientSignature = Get-AuthenticodeSignature -LiteralPath $resolvedClientPath
    if ($null -eq $clientSignature.SignerCertificate) {
        throw "The Ame client signer certificate is unavailable"
    }
    $clientSha256 = (
        Get-FileHash -LiteralPath $resolvedClientPath -Algorithm SHA256
    ).Hash.ToUpperInvariant()
    $clientHasher = [Security.Cryptography.SHA256]::Create()
    try {
        $clientSignerSha256 = ConvertTo-AmeBrokerUpperHex `
            -Bytes ($clientHasher.ComputeHash($clientSignature.SignerCertificate.RawData))
    } finally {
        $clientHasher.Dispose()
    }
    $clientPathHex = ConvertTo-AmeBrokerUpperHex `
        -Bytes ([Text.Encoding]::Unicode.GetBytes($resolvedClientPath))
    $manifest = "AMEJBID2|protocol=$($protocol.Protocol)|max_frame=" +
        "$($protocol.MaximumFrameBytes)|sha256=$sha256|signer_sha256=$signerSha256|" +
        "publisher_utf16le=$publisherHex|client_path_utf16le=$clientPathHex|" +
        "client_sha256=$clientSha256|client_signer_sha256=$clientSignerSha256"
    Assert-AmeBrokerIdentityManifestFacts -Manifest $manifest
    return $manifest
}

function Assert-AmeBrokerIdentityManifestFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Manifest
    )

    $match = [regex]::Match(
        $Manifest,
        '^AMEJBID2\|protocol=(\d+)\|max_frame=(\d+)\|sha256=([0-9A-F]{64})' +
            '\|signer_sha256=([0-9A-F]{64})\|publisher_utf16le=([0-9A-F]+)' +
            '\|client_path_utf16le=([0-9A-F]+)\|client_sha256=([0-9A-F]{64})' +
            '\|client_signer_sha256=([0-9A-F]{64})$',
        [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (
        -not $match.Success -or
        [int]$match.Groups[1].Value -ne $script:AmeBrokerProtocolVersion -or
        [int]$match.Groups[2].Value -ne $script:AmeBrokerMaximumFrameBytes -or
        $match.Groups[5].Value.Length % 4 -ne 0 -or
        $match.Groups[6].Value.Length % 4 -ne 0
    ) {
        throw "The broker service identity manifest is invalid or incompatible"
    }
    $clientPathHex = $match.Groups[6].Value
    $clientPathBytes = [byte[]]::new($clientPathHex.Length / 2)
    for ($index = 0; $index -lt $clientPathBytes.Length; $index += 1) {
        $clientPathBytes[$index] = [Convert]::ToByte(
            $clientPathHex.Substring($index * 2, 2),
            16
        )
    }
    $clientPath = [Text.Encoding]::Unicode.GetString($clientPathBytes)
    if (
        $clientPath.IndexOf([char]0) -ge 0 -or
        -not $clientPath.Equals(
            (Get-AmeApplicationInstalledBinaryPath),
            [System.StringComparison]::OrdinalIgnoreCase
        )
    ) {
        throw "The broker service identity manifest does not name the fixed Ame client"
    }
}

function Get-AmeBrokerConfiguredIdentityManifest {
    $serviceKeyPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$script:AmeBrokerServiceName"
    $manifest = [string](Get-ItemProperty -LiteralPath $serviceKeyPath).Description
    Assert-AmeBrokerIdentityManifestFacts -Manifest $manifest
    return $manifest
}

function Assert-AmeBrokerReleaseBinaryName {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if ([System.IO.Path]::GetFileName($Path) -cne $script:AmeBrokerBinaryName) {
        throw "The broker binary must retain its release filename"
    }
}

function Invoke-AmeBrokerSc {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,
        [int[]]$AllowedExitCodes = @(0)
    )

    $output = @(& "$env:SystemRoot\System32\sc.exe" @Arguments 2>&1)
    $exitCode = $LASTEXITCODE
    if ($AllowedExitCodes -notcontains $exitCode) {
        throw "sc.exe $($Arguments[0]) failed with exit code $exitCode"
    }
    return [pscustomobject]@{
        ExitCode = $exitCode
        Output = ($output -join "`n")
    }
}

function New-AmeBrokerRollbackAction {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Action
    )

    return [pscustomobject]@{
        Name = $Name
        Action = $Action
    }
}

function Invoke-AmeBrokerRollbackActions {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Actions
    )

    $failures = [System.Collections.Generic.List[string]]::new()
    for ($index = $Actions.Count - 1; $index -ge 0; $index -= 1) {
        $rollback = $Actions[$index]
        try {
            & $rollback.Action | Out-Null
        } catch {
            $failures.Add("$($rollback.Name): $($_.Exception.Message)")
        }
    }
    return $failures.ToArray()
}

function Throw-AmeBrokerTransactionFailure {
    param(
        [Parameter(Mandatory = $true)]
        [System.Management.Automation.ErrorRecord]$OriginalError,
        [string[]]$RollbackFailures = @()
    )

    if ($RollbackFailures.Count -gt 0) {
        $message = "$($OriginalError.Exception.Message); rollback failures: " +
            ($RollbackFailures -join "; ")
        throw [InvalidOperationException]::new($message, $OriginalError.Exception)
    }
    throw $OriginalError
}

function Remove-AmeBrokerOwnedLeafNoFollow {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string[]]$AllowedPaths
    )

    $full = [IO.Path]::GetFullPath($Path)
    if (-not @($AllowedPaths | Where-Object {
        $full.Equals([IO.Path]::GetFullPath($_), [StringComparison]::OrdinalIgnoreCase)
    }).Count) {
        throw "A transaction leaf is outside its fixed ownership set"
    }
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
        return
    }
    $item = Get-Item -LiteralPath $full -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "A transaction leaf became a reparse point"
    }
    $final = ConvertTo-AmeBrokerComparablePath -Path (
        [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($full)
    )
    if (-not $full.Equals($final, [StringComparison]::OrdinalIgnoreCase)) {
        throw "A transaction leaf resolves outside its fixed path"
    }
    Remove-Item -LiteralPath $full -Force
}

function Test-AmeBrokerBinaryPayloadFactsMatch {
    param(
        [AllowNull()]
        [psobject]$Expected,
        [Parameter(Mandatory = $true)]
        [psobject]$Actual
    )

    if ($null -eq $Expected) {
        return $false
    }
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Expected
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Actual
    return [bool]$Expected.Exists -and [bool]$Actual.Exists -and
        [int64]$Expected.Length -eq [int64]$Actual.Length -and
        [string]$Expected.Sha256 -ceq [string]$Actual.Sha256
}

function Get-AmeBrokerTransactionServiceFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if (-not (Test-AmeBrokerServiceExists)) {
        return [pscustomobject]@{
            Exists = $false
            BinaryPath = $null
            IdentityManifest = $null
        }
    }
    $serviceKeyPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$script:AmeBrokerServiceName"
    $configuration = Get-ItemProperty -LiteralPath $serviceKeyPath
    $match = [regex]::Match(
        [string]$configuration.ImagePath,
        '^"([^"]+)"$',
        [Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (-not $match.Success) {
        throw "The interrupted broker service path is malformed"
    }
    $binaryPath = [IO.Path]::GetFullPath($match.Groups[1].Value)
    $allowed = @(
        [string]$State.InstalledBinary,
        [string]$State.StagedBinary,
        [string]$State.BackupBinary
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    if (-not @($allowed | Where-Object {
        $binaryPath.Equals(
            [IO.Path]::GetFullPath($_),
            [StringComparison]::OrdinalIgnoreCase
        )
    }).Count) {
        throw "The interrupted broker service points outside transaction-owned leaves"
    }
    $manifest = [string]$configuration.Description
    Assert-AmeBrokerServiceConfiguration `
        -BinaryPath $binaryPath `
        -IdentityManifest $manifest
    $binaryFacts = Get-AmeBrokerTransactionBinaryFacts `
        -Path $binaryPath `
        -ExpectedPublisher ([string]$State.ExpectedPublisher)
    if (-not [bool]$binaryFacts.Exists) {
        throw "The interrupted broker service points to a missing binary"
    }
    $isPrevious = Test-AmeBrokerBinaryPayloadFactsMatch `
        -Expected $State.ExpectedPreviousBinary `
        -Actual $binaryFacts
    $isNew = Test-AmeBrokerBinaryPayloadFactsMatch `
        -Expected $State.ExpectedNewBinary `
        -Actual $binaryFacts
    if (($isPrevious -and $manifest -cne [string]$State.PreviousIdentityManifest) -or
        ($isNew -and $manifest -cne [string]$State.NewIdentityManifest) -or
        (-not $isPrevious -and -not $isNew)) {
        throw "The interrupted broker service points to an unknown binary identity"
    }
    return [pscustomobject]@{
        Exists = $true
        BinaryPath = $binaryPath
        IdentityManifest = $manifest
        BinaryFacts = $binaryFacts
        IsPrevious = $isPrevious
        IsNew = $isNew
    }
}

function Get-AmeBrokerTransactionLeafFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    $facts = @{}
    foreach ($path in @(
        [string]$State.InstalledBinary,
        [string]$State.StagedBinary,
        [string]$State.BackupBinary
    )) {
        if ([string]::IsNullOrWhiteSpace($path)) {
            continue
        }
        $binary = Get-AmeBrokerTransactionBinaryFacts `
            -Path $path `
            -ExpectedPublisher ([string]$State.ExpectedPublisher)
        if ([bool]$binary.Exists -and
            -not (Test-AmeBrokerBinaryPayloadFactsMatch `
                -Expected $State.ExpectedPreviousBinary `
                -Actual $binary) -and
            -not (Test-AmeBrokerBinaryPayloadFactsMatch `
                -Expected $State.ExpectedNewBinary `
                -Actual $binary)) {
            throw "A transaction leaf contains an unknown signed broker payload"
        }
        $facts[[IO.Path]::GetFullPath($path)] = $binary
    }
    return $facts
}

function Find-AmeBrokerTransactionLeafByPayload {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$LeafFacts,
        [AllowNull()]
        [psobject]$Expected,
        [string[]]$Preference = @()
    )

    foreach ($path in $Preference) {
        $full = [IO.Path]::GetFullPath($path)
        if ($LeafFacts.ContainsKey($full) -and
            (Test-AmeBrokerBinaryPayloadFactsMatch `
                -Expected $Expected `
                -Actual $LeafFacts[$full])) {
            return $full
        }
    }
    return $null
}

function Repair-AmeBrokerInterruptedBinaryTransaction {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if ($null -eq $State.ExpectedNewBinary) {
        if ($null -ne $State.PendingIntent -or @($State.AppliedOperations).Count -gt 0) {
            throw "A moved binary transaction lacks its expected new identity"
        }
        foreach ($path in @([string]$State.StagedBinary, [string]$State.BackupBinary)) {
            Remove-AmeBrokerOwnedLeafNoFollow `
                -Path $path `
                -AllowedPaths @(
                    [string]$State.InstalledBinary,
                    [string]$State.StagedBinary,
                    [string]$State.BackupBinary
                )
        }
        if ([string]$State.Operation -ceq "upgrade") {
            $installedFacts = Get-AmeBrokerTransactionBinaryFacts `
                -Path ([string]$State.InstalledBinary) `
                -ExpectedPublisher ([string]$State.ExpectedPublisher)
            if (-not (Test-AmeBrokerBinaryFactsMatch `
                -Expected $State.ExpectedPreviousBinary `
                -Actual $installedFacts)) {
                throw "The pre-move upgrade binary no longer matches its journaled identity"
            }
            Assert-AmeBrokerServiceConfiguration `
                -BinaryPath ([string]$State.InstalledBinary) `
                -IdentityManifest ([string]$State.PreviousIdentityManifest)
        }
        Remove-AmeBrokerTransactionMarker
        return
    }
    if ([string]::IsNullOrWhiteSpace([string]$State.NewIdentityManifest)) {
        throw "The interrupted binary transaction lacks its expected new manifest"
    }
    $serviceFacts = Get-AmeBrokerTransactionServiceFacts -State $State
    if (-not [bool]$serviceFacts.Exists) {
        throw "The interrupted binary transaction lost its fixed service"
    }
    if ($null -ne $State.PendingIntent) {
        $pendingSource = Get-AmeBrokerTransactionBinaryFacts `
            -Path ([string]$State.PendingIntent.SourcePath) `
            -ExpectedPublisher ([string]$State.ExpectedPublisher)
        $pendingDestination = Get-AmeBrokerTransactionBinaryFacts `
            -Path ([string]$State.PendingIntent.DestinationPath) `
            -ExpectedPublisher ([string]$State.ExpectedPublisher)
        $pendingResolution = Resolve-AmeBrokerMoveIntentFacts `
            -Intent $State.PendingIntent `
            -SourceFacts $pendingSource `
            -DestinationFacts $pendingDestination
        if ($pendingResolution -ceq "NotApplied" -and
            ([IO.Path]::GetFullPath([string]$serviceFacts.BinaryPath)).Equals(
                [IO.Path]::GetFullPath([string]$State.PendingIntent.SourcePath),
                [StringComparison]::OrdinalIgnoreCase
            )) {
            throw "The broker service still points to a pending move source"
        }
        Resolve-AmeBrokerPendingMove -State $State
        $serviceFacts = Get-AmeBrokerTransactionServiceFacts -State $State
    }
    $leafFacts = Get-AmeBrokerTransactionLeafFacts -State $State
    $installedPath = [IO.Path]::GetFullPath([string]$State.InstalledBinary)
    $stagedPath = [IO.Path]::GetFullPath([string]$State.StagedBinary)
    $backupPath = [IO.Path]::GetFullPath([string]$State.BackupBinary)
    $previousPath = Find-AmeBrokerTransactionLeafByPayload `
        -LeafFacts $leafFacts `
        -Expected $State.ExpectedPreviousBinary `
        -Preference @($installedPath, $backupPath, $stagedPath)
    $newPath = Find-AmeBrokerTransactionLeafByPayload `
        -LeafFacts $leafFacts `
        -Expected $State.ExpectedNewBinary `
        -Preference @($installedPath, $stagedPath, $backupPath)
    $shouldRollback = $null -ne $previousPath
    if (-not $shouldRollback -and $null -eq $newPath) {
        throw "The interrupted binary transaction has no recoverable broker payload"
    }
    $targetPath = if ($shouldRollback) { $previousPath } else { $newPath }
    $targetManifest = if ($shouldRollback) {
        [string]$State.PreviousIdentityManifest
    } else {
        [string]$State.NewIdentityManifest
    }
    if ([string]::IsNullOrWhiteSpace($targetManifest)) {
        throw "The interrupted binary transaction lacks the target service manifest"
    }
    if (-not $targetPath.Equals($installedPath, [StringComparison]::OrdinalIgnoreCase)) {
        $holdingPath = $null
        foreach ($candidate in @($stagedPath, $backupPath)) {
            if ($candidate.Equals($targetPath, [StringComparison]::OrdinalIgnoreCase)) {
                continue
            }
            $candidateFacts = $leafFacts[$candidate]
            if ([bool]$candidateFacts.Exists) {
                $holdingPath = $candidate
                break
            }
        }
        if ($null -eq $holdingPath) {
            $holdingPath = if (-not $stagedPath.Equals(
                $targetPath,
                [StringComparison]::OrdinalIgnoreCase
            )) { $stagedPath } else { $backupPath }
            Copy-Item -LiteralPath $targetPath -Destination $holdingPath
            Set-AmeBrokerFileAcl -Path $holdingPath -IncludeServiceSid $true
            $leafFacts[$holdingPath] = Get-AmeBrokerTransactionBinaryFacts `
                -Path $holdingPath `
                -ExpectedPublisher ([string]$State.ExpectedPublisher)
        }
        $holdingFacts = $leafFacts[$holdingPath]
        $holdingManifest = if (Test-AmeBrokerBinaryPayloadFactsMatch `
            -Expected $State.ExpectedPreviousBinary `
            -Actual $holdingFacts) {
            [string]$State.PreviousIdentityManifest
        } else {
            [string]$State.NewIdentityManifest
        }
        Set-AmeBrokerServiceConfiguration `
            -BinaryPath $holdingPath `
            -IdentityManifest $holdingManifest
        $installedFacts = $leafFacts[$installedPath]
        if ([bool]$installedFacts.Exists) {
            Remove-AmeBrokerOwnedLeafNoFollow `
                -Path $installedPath `
                -AllowedPaths @($installedPath, $stagedPath, $backupPath)
        }
        $targetFacts = Get-AmeBrokerTransactionBinaryFacts `
            -Path $targetPath `
            -ExpectedPublisher ([string]$State.ExpectedPublisher)
        Invoke-AmeBrokerWriteAheadMove `
            -State $State `
            -SourcePath $targetPath `
            -DestinationPath $installedPath `
            -ExpectedSource $targetFacts `
            -ExpectedDestinationBefore (New-AmeBrokerAbsentBinaryFacts) `
            -NextPhase recovery_installed
    }
    Set-AmeBrokerServiceConfiguration `
        -BinaryPath $installedPath `
        -IdentityManifest $targetManifest
    Set-AmeBrokerBinaryAcl `
        -InstallDirectory (Get-AmeBrokerInstallDirectory) `
        -BinaryPath $installedPath
    $finalFacts = Get-AmeBrokerTransactionBinaryFacts `
        -Path $installedPath `
        -ExpectedPublisher ([string]$State.ExpectedPublisher)
    $expectedFinal = if ($shouldRollback) {
        $State.ExpectedPreviousBinary
    } else {
        $State.ExpectedNewBinary
    }
    if (-not (Test-AmeBrokerBinaryPayloadFactsMatch `
        -Expected $expectedFinal `
        -Actual $finalFacts)) {
        throw "The recovered broker payload does not match its journaled identity"
    }
    foreach ($path in @($stagedPath, $backupPath)) {
        if (-not $path.Equals($installedPath, [StringComparison]::OrdinalIgnoreCase)) {
            Remove-AmeBrokerOwnedLeafNoFollow `
                -Path $path `
                -AllowedPaths @($installedPath, $stagedPath, $backupPath)
        }
    }
    Assert-AmeBrokerServiceConfiguration `
        -BinaryPath $installedPath `
        -IdentityManifest $targetManifest
    if ([bool]$State.WasRunning) {
        Start-AmeBrokerServiceBounded
    }
    Remove-AmeBrokerTransactionMarker
}

function Repair-AmeBrokerInterruptedTransaction {
    param(
        [scriptblock]$ProcessProbe,
        [scriptblock]$StateReader = { Read-AmeBrokerTransactionState },
        [scriptblock]$UninstallStateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$UninstallMarkerRemovalAction = { Remove-AmeBrokerTransactionMarker },
        [hashtable]$UninstallActionOverrides
    )

    $state = & $StateReader
    if ($null -eq $state) {
        return
    }
    Assert-AmeBrokerTransactionStateFacts -State $state
    $ownerActive = if ($null -eq $ProcessProbe) {
        Test-AmeBrokerTransactionOwnerActive -State $state
    } else {
        Test-AmeBrokerTransactionOwnerActive -State $state -ProcessProbe $ProcessProbe
    }
    if ($ownerActive) {
        throw "An active broker lifecycle transaction already owns the fixed install tree"
    }
    if ([string]$state.Operation -ceq "uninstall") {
        Complete-AmeBrokerUninstallTransaction `
            -State $state `
            -StateWriter $UninstallStateWriter `
            -MarkerRemovalAction $UninstallMarkerRemovalAction `
            -ActionOverrides $UninstallActionOverrides
        return
    }
    if ([string]$state.Operation -in @("repair", "upgrade")) {
        Repair-AmeBrokerInterruptedBinaryTransaction -State $state
        return
    }
    $installDirectory = Get-AmeBrokerInstallDirectory
    Assert-AmeBrokerFixedInstallTree `
        -InstallDirectory $installDirectory | Out-Null
    $allowedLeaves = @(
        [string]$state.InstalledBinary,
        [string]$state.StagedBinary,
        [string]$state.BackupBinary
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    $failures = [Collections.Generic.List[string]]::new()
    if ([string]$state.Phase -ceq "committed") {
        foreach ($path in @([string]$state.StagedBinary, [string]$state.BackupBinary)) {
            if ([string]::IsNullOrWhiteSpace($path)) {
                continue
            }
            try {
                Remove-AmeBrokerOwnedLeafNoFollow -Path $path -AllowedPaths $allowedLeaves
            } catch {
                $failures.Add("finalize $path`: $($_.Exception.Message)")
            }
        }
        if ($failures.Count -eq 0) {
            Remove-AmeBrokerTransactionMarker
        }
        if ($failures.Count -gt 0) {
            throw "Interrupted committed broker transaction cleanup failed: " +
                ($failures -join "; ")
        }
        return
    }
    if ([bool]$state.WasRunning -or [bool]$state.ConfigurationChanged) {
        try {
            Stop-AmeBrokerServiceBounded
        } catch {
            $failures.Add("stop interrupted broker: $($_.Exception.Message)")
        }
    }
    if ([string]$state.Operation -ceq "install") {
        try {
            if (Test-AmeBrokerServiceExists) {
                Remove-AmeBrokerServiceBounded
            }
        } catch {
            $failures.Add("remove transaction-owned service: $($_.Exception.Message)")
        }
    }
    foreach ($path in @([string]$state.StagedBinary, [string]$state.InstalledBinary)) {
        if ([string]::IsNullOrWhiteSpace($path)) {
            continue
        }
        if ($path -ceq [string]$state.InstalledBinary -and
            -not [bool]$state.InstalledOwned -and
            [string]$state.Operation -cne "install") {
            continue
        }
        try {
            Remove-AmeBrokerOwnedLeafNoFollow -Path $path -AllowedPaths $allowedLeaves
        } catch {
            $failures.Add("remove transaction-owned leaf $path`: $($_.Exception.Message)")
        }
    }
    if ([bool]$state.BackupCreated) {
        try {
            $backup = [string]$state.BackupBinary
            if (-not (Test-Path -LiteralPath $backup -PathType Leaf)) {
                throw "The transaction-owned broker backup is unavailable"
            }
            $backupItem = Get-Item -LiteralPath $backup -Force
            if ($backupItem.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "The transaction-owned broker backup became a reparse point"
            }
            Move-Item -LiteralPath $backup -Destination ([string]$state.InstalledBinary)
            Set-AmeBrokerBinaryAcl `
                -InstallDirectory $installDirectory `
                -BinaryPath ([string]$state.InstalledBinary)
        } catch {
            $failures.Add("restore previous broker binary: $($_.Exception.Message)")
        }
    }
    if ([bool]$state.ConfigurationChanged -and (Test-AmeBrokerServiceExists)) {
        try {
            if ([string]::IsNullOrWhiteSpace([string]$state.PreviousIdentityManifest)) {
                throw "The previous service identity manifest is unavailable"
            }
            Set-AmeBrokerServiceConfiguration `
                -BinaryPath ([string]$state.InstalledBinary) `
                -IdentityManifest ([string]$state.PreviousIdentityManifest)
        } catch {
            $failures.Add("restore previous service configuration: $($_.Exception.Message)")
        }
    }
    if ([bool]$state.WasRunning -and (Test-AmeBrokerServiceExists)) {
        try {
            Start-AmeBrokerServiceBounded
        } catch {
            $failures.Add("restart previous broker: $($_.Exception.Message)")
        }
    }
    if ([bool]$state.ApplicationOwned) {
        try {
            Remove-AmeBrokerOwnedTreeNoFollow `
                -Path ([string]$state.ApplicationDirectory) `
                -ExpectedParent (Get-AmeProductParentDirectory)
        } catch {
            $failures.Add("remove transaction-owned Application bundle: $($_.Exception.Message)")
        }
    }
    try {
        $ownedDirectories = @($state.OwnedDirectories | Sort-Object Length -Descending)
        $directoryFailures = @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories $ownedDirectories)
        foreach ($failure in $directoryFailures) {
            $failures.Add([string]$failure)
        }
    } catch {
        $failures.Add("remove transaction-owned directories: $($_.Exception.Message)")
    }
    try {
        Restore-AmeBrokerParentPrestate -Prestate ([pscustomobject]@{
            Path = [string]$state.ParentPath
            Existed = [bool]$state.ParentExisted
            Sddl = $state.ParentSddl
        })
    } catch {
        $failures.Add("restore Ame parent prestate: $($_.Exception.Message)")
    }
    if ($failures.Count -eq 0) {
        Remove-AmeBrokerTransactionMarker
    }
    if ($failures.Count -gt 0) {
        throw "Interrupted broker transaction recovery failed: " + ($failures -join "; ")
    }
}

function Test-AmeBrokerServiceExists {
    $result = Invoke-AmeBrokerSc `
        -Arguments @("query", $script:AmeBrokerServiceName) `
        -AllowedExitCodes @(0, 1060)
    return $result.ExitCode -eq 0
}

function Stop-AmeBrokerServiceBounded {
    param(
        [int]$TimeoutSeconds = 30
    )

    if (-not (Test-AmeBrokerServiceExists)) {
        return
    }
    $service = Get-Service -Name $script:AmeBrokerServiceName
    try {
        if ($service.Status -eq [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
            return
        }
        Invoke-AmeBrokerSc -Arguments @("stop", $script:AmeBrokerServiceName) | Out-Null
        $service.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Stopped,
            [TimeSpan]::FromSeconds($TimeoutSeconds)
        )
    } finally {
        $service.Dispose()
    }
}

function Start-AmeBrokerServiceBounded {
    param(
        [int]$TimeoutSeconds = 30
    )

    Invoke-AmeBrokerSc -Arguments @("start", $script:AmeBrokerServiceName) | Out-Null
    $service = Get-Service -Name $script:AmeBrokerServiceName
    try {
        $service.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Running,
            [TimeSpan]::FromSeconds($TimeoutSeconds)
        )
    } finally {
        $service.Dispose()
    }
}

function Wait-AmeBrokerServiceDeletion {
    param(
        [int]$TimeoutMilliseconds = 30000,
        [int]$PollMilliseconds = 100,
        [scriptblock]$ServiceExistsProbe = { Test-AmeBrokerServiceExists }
    )

    if ($TimeoutMilliseconds -le 0 -or $PollMilliseconds -le 0) {
        throw "The service deletion wait must be positively bounded"
    }
    $timer = [Diagnostics.Stopwatch]::StartNew()
    try {
        while (& $ServiceExistsProbe) {
            if ($timer.ElapsedMilliseconds -ge $TimeoutMilliseconds) {
                throw "The journal broker service object did not disappear before the deadline"
            }
            $remainingMilliseconds = [Math]::Max(
                1,
                $TimeoutMilliseconds - [int]$timer.ElapsedMilliseconds
            )
            Start-Sleep -Milliseconds ([Math]::Min(
                $PollMilliseconds,
                $remainingMilliseconds
            ))
        }
    } finally {
        $timer.Stop()
    }
}

function Remove-AmeBrokerServiceBounded {
    if (-not (Test-AmeBrokerServiceExists)) {
        return
    }
    Stop-AmeBrokerServiceBounded
    Invoke-AmeBrokerSc -Arguments @("delete", $script:AmeBrokerServiceName) | Out-Null
    Wait-AmeBrokerServiceDeletion
}

function Set-AmeBrokerServiceConfiguration {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$IdentityManifest,
        [switch]$Create,
        [ref]$CreatedState,
        [scriptblock]$ScInvoker
    )

    Assert-AmeBrokerIdentityManifestFacts -Manifest $IdentityManifest

    $quotedBinaryPath = '"{0}"' -f ([System.IO.Path]::GetFullPath($BinaryPath))
    if ($Create) {
        if ($null -eq $CreatedState) {
            throw "Service creation requires an explicit transaction ownership state"
        }
        $CreatedState.Value = $false
        $arguments = @(
            "create",
            $script:AmeBrokerServiceName,
            "binPath= $quotedBinaryPath",
            "type= own",
            "start= demand",
            "error= normal",
            "obj= LocalSystem",
            "DisplayName= $script:AmeBrokerDisplayName"
        )
        if ($null -eq $ScInvoker) {
            Invoke-AmeBrokerSc -Arguments $arguments | Out-Null
        } else {
            & $ScInvoker $arguments | Out-Null
        }
        $CreatedState.Value = $true
    } else {
        $arguments = @(
            "config",
            $script:AmeBrokerServiceName,
            "binPath= $quotedBinaryPath",
            "type= own",
            "start= demand",
            "error= normal",
            "obj= LocalSystem",
            "DisplayName= $script:AmeBrokerDisplayName"
        )
        if ($null -eq $ScInvoker) {
            Invoke-AmeBrokerSc -Arguments $arguments | Out-Null
        } else {
            & $ScInvoker $arguments | Out-Null
        }
    }
    foreach ($command in @(
        [pscustomobject]@{
            Arguments = @("sidtype", $script:AmeBrokerServiceName, "restricted")
        },
        [pscustomobject]@{
            Arguments = @(
                "privs",
                $script:AmeBrokerServiceName,
                $script:AmeBrokerRequiredPrivilege
            )
        },
        [pscustomobject]@{
            Arguments = @("sdset", $script:AmeBrokerServiceName, $script:AmeBrokerServiceDacl)
        },
        [pscustomobject]@{
            Arguments = @("description", $script:AmeBrokerServiceName, $IdentityManifest)
        }
    )) {
        $arguments = [string[]]$command.Arguments
        if ($null -eq $ScInvoker) {
            Invoke-AmeBrokerSc -Arguments $arguments | Out-Null
        } else {
            & $ScInvoker $arguments | Out-Null
        }
    }
}

function Get-AmeBrokerServiceSid {
    return $script:AmeBrokerServiceSid
}

function Assert-AmeBrokerServiceSidFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ActualSid
    )

    if ([string]$ActualSid -cne $script:AmeBrokerServiceSid) {
        throw "The fixed broker service SID does not match the service name"
    }
}

function Get-AmeBrokerCalculatedServiceSid {
    $result = Invoke-AmeBrokerSc `
        -Arguments @("showsid", $script:AmeBrokerServiceName)
    $match = [regex]::Match(
        $result.Output,
        'S-1-5-80-(?:\d+-){4}\d+',
        [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (-not $match.Success) {
        throw "sc.exe did not return the broker service SID"
    }
    return $match.Value
}

function Get-AmeBrokerExpectedAclRules {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid
    )

    $inheritanceFlags = if ($Kind -ceq "Directory") { 3 } else { 0 }
    $rules = @(
        [pscustomobject]@{
            Sid = $script:AmeBrokerSystemSid
            Rights = [uint64]2032127
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerAdministratorsSid
            Rights = [uint64]2032127
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerUsersSid
            Rights = [uint64]131241
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        }
    )
    if ($IncludeServiceSid) {
        $rules += [pscustomobject]@{
            Sid = Get-AmeBrokerServiceSid
            Rights = [uint64]131241
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        }
    }
    return $rules
}

function Assert-AmeBrokerExactAclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts,
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid
    )

    if ([string]$Facts.OwnerSid -cne $script:AmeBrokerAdministratorsSid) {
        throw "The broker $Kind ACL owner must be BUILTIN Administrators"
    }
    if ([string]$Facts.GroupSid -cne $script:AmeBrokerAdministratorsSid) {
        throw "The broker $Kind ACL group must be BUILTIN Administrators"
    }
    if (-not $Facts.AreAccessRulesProtected) {
        throw "The broker $Kind ACL must be protected from inheritance"
    }
    $actualRules = @($Facts.Rules)
    $expectedRules = @(Get-AmeBrokerExpectedAclRules `
        -Kind $Kind `
        -IncludeServiceSid $IncludeServiceSid)
    if ($actualRules.Count -ne $expectedRules.Count) {
        throw "The broker $Kind ACL contains an unexpected access rule"
    }
    foreach ($expected in $expectedRules) {
        $matches = @(
            $actualRules | Where-Object {
                [string]$_.Sid -ceq [string]$expected.Sid -and
                [uint64]$_.Rights -eq [uint64]$expected.Rights -and
                $_.AccessControlType -ceq "Allow" -and
                [int]$_.InheritanceFlags -eq [int]$expected.InheritanceFlags -and
                [int]$_.PropagationFlags -eq [int]$expected.PropagationFlags -and
                -not $_.IsInherited
            }
        )
        if ($matches.Count -ne 1) {
            throw "The broker $Kind ACL does not exactly match its admitted rules"
        }
    }
}

function Get-AmeApplicationExpectedAclRules {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind
    )

    $inheritanceFlags = if ($Kind -ceq "Directory") { 3 } else { 0 }
    return @(
        [pscustomobject]@{
            Sid = $script:AmeBrokerSystemSid
            Rights = [uint64]2032127
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerAdministratorsSid
            Rights = [uint64]2032127
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerUsersSid
            Rights = [uint64]131241
            InheritanceFlags = $inheritanceFlags
            PropagationFlags = 0
        }
    )
}

function Assert-AmeApplicationExactAclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts,
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind
    )

    if ([string]$Facts.OwnerSid -cne $script:AmeBrokerAdministratorsSid) {
        throw "The Ame application $Kind ACL owner must be BUILTIN Administrators"
    }
    if ([string]$Facts.GroupSid -cne $script:AmeBrokerAdministratorsSid) {
        throw "The Ame application $Kind ACL group must be BUILTIN Administrators"
    }
    if (-not $Facts.AreAccessRulesProtected) {
        throw "The Ame application $Kind ACL must be protected from inheritance"
    }
    $actualRules = @($Facts.Rules)
    $expectedRules = @(Get-AmeApplicationExpectedAclRules -Kind $Kind)
    if ($actualRules.Count -ne $expectedRules.Count) {
        throw "The Ame application $Kind ACL contains an unexpected access rule"
    }
    foreach ($expected in $expectedRules) {
        $matches = @(
            $actualRules | Where-Object {
                [string]$_.Sid -ceq [string]$expected.Sid -and
                [uint64]$_.Rights -eq [uint64]$expected.Rights -and
                $_.AccessControlType -ceq "Allow" -and
                [int]$_.InheritanceFlags -eq [int]$expected.InheritanceFlags -and
                [int]$_.PropagationFlags -eq [int]$expected.PropagationFlags -and
                -not $_.IsInherited
            }
        )
        if ($matches.Count -ne 1) {
            throw "The Ame application $Kind ACL does not exactly match its admitted rules"
        }
    }
}

function Set-AmeApplicationPathAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [ValidateSet("Directory", "File")]
        [string]$Kind
    )

    $administratorsSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinAdministratorsSid,
        $null
    )
    $systemSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::LocalSystemSid,
        $null
    )
    $usersSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinUsersSid,
        $null
    )
    if ($Kind -ceq "Directory") {
        $security = [Security.AccessControl.DirectorySecurity]::new()
        $inheritance = [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor `
            [Security.AccessControl.InheritanceFlags]::ObjectInherit
    } else {
        $security = [Security.AccessControl.FileSecurity]::new()
        $inheritance = [Security.AccessControl.InheritanceFlags]::None
    }
    $security.SetAccessRuleProtection($true, $false)
    $security.SetOwner($administratorsSid)
    $security.SetGroup($administratorsSid)
    foreach ($entry in @(
        @($systemSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($administratorsSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($usersSid, [Security.AccessControl.FileSystemRights]::ReadAndExecute)
    )) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            $entry[0],
            $entry[1],
            $inheritance,
            [Security.AccessControl.PropagationFlags]::None,
            [Security.AccessControl.AccessControlType]::Allow
        )
        $security.AddAccessRule($rule) | Out-Null
    }
    Set-Acl -LiteralPath $Path -AclObject $security
    Assert-AmeApplicationExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $Path) `
        -Kind $Kind
}

function Set-AmeBrokerDirectoryAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid
    )

    $systemSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::LocalSystemSid,
        $null
    )
    $administratorsSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinAdministratorsSid,
        $null
    )
    $usersSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinUsersSid,
        $null
    )
    $directorySecurity = [Security.AccessControl.DirectorySecurity]::new()
    $directorySecurity.SetAccessRuleProtection($true, $false)
    $directorySecurity.SetOwner($administratorsSid)
    $directorySecurity.SetGroup($administratorsSid)
    $inheritance = [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor `
        [Security.AccessControl.InheritanceFlags]::ObjectInherit
    $propagation = [Security.AccessControl.PropagationFlags]::None
    $entries = @(
        @($systemSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($administratorsSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($usersSid, [Security.AccessControl.FileSystemRights]::ReadAndExecute)
    )
    if ($IncludeServiceSid) {
        $serviceSid = [Security.Principal.SecurityIdentifier]::new(
            (Get-AmeBrokerServiceSid)
        )
        $entries += ,@(
            $serviceSid,
            [Security.AccessControl.FileSystemRights]::ReadAndExecute
        )
    }
    foreach ($entry in $entries) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            $entry[0],
            $entry[1],
            $inheritance,
            $propagation,
            [Security.AccessControl.AccessControlType]::Allow
        )
        $directorySecurity.AddAccessRule($rule) | Out-Null
    }
    Set-Acl -LiteralPath $Path -AclObject $directorySecurity
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $Path) `
        -Kind "Directory" `
        -IncludeServiceSid $IncludeServiceSid
}

function Set-AmeBrokerFileAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid
    )

    $systemSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::LocalSystemSid,
        $null
    )
    $administratorsSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinAdministratorsSid,
        $null
    )
    $usersSid = [Security.Principal.SecurityIdentifier]::new(
        [Security.Principal.WellKnownSidType]::BuiltinUsersSid,
        $null
    )

    $fileSecurity = [Security.AccessControl.FileSecurity]::new()
    $fileSecurity.SetAccessRuleProtection($true, $false)
    $fileSecurity.SetOwner($administratorsSid)
    $fileSecurity.SetGroup($administratorsSid)
    $entries = @(
        @($systemSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($administratorsSid, [Security.AccessControl.FileSystemRights]::FullControl),
        @($usersSid, [Security.AccessControl.FileSystemRights]::ReadAndExecute)
    )
    if ($IncludeServiceSid) {
        $serviceSid = [Security.Principal.SecurityIdentifier]::new(
            (Get-AmeBrokerServiceSid)
        )
        $entries += ,@(
            $serviceSid,
            [Security.AccessControl.FileSystemRights]::ReadAndExecute
        )
    }
    foreach ($entry in $entries) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            $entry[0],
            $entry[1],
            [Security.AccessControl.AccessControlType]::Allow
        )
        $fileSecurity.AddAccessRule($rule) | Out-Null
    }
    Set-Acl -LiteralPath $Path -AclObject $fileSecurity
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $Path) `
        -Kind "File" `
        -IncludeServiceSid $IncludeServiceSid
}

function Set-AmeBrokerBinaryAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath
    )

    Set-AmeBrokerDirectoryAcl `
        -Path $InstallDirectory `
        -IncludeServiceSid $true
    Set-AmeBrokerFileAcl -Path $BinaryPath -IncludeServiceSid $true
}

function Assert-AmeBrokerBinaryAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath
    )

    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $InstallDirectory) `
        -Kind "Directory" `
        -IncludeServiceSid $true
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $BinaryPath) `
        -Kind "File" `
        -IncludeServiceSid $true
}

function Assert-AmeBrokerPhysicalFile {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPath
    )

    $item = Get-Item -LiteralPath $ExpectedPath -Force
    if ($item.PSIsContainer) {
        throw "The broker binary path must be a file"
    }
    if ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
        throw "The broker binary must not be a reparse point"
    }
    $expected = ConvertTo-AmeBrokerComparablePath -Path $item.FullName
    $final = ConvertTo-AmeBrokerComparablePath -Path (
        [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($item.FullName)
    )
    if (-not $expected.Equals($final, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "The broker binary resolves outside its fixed physical path"
    }
}

function Assert-AmeApplicationBundleSource {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BundlePath,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $resolved = [System.IO.Path]::GetFullPath($BundlePath)
    if (-not (Test-Path -LiteralPath $resolved -PathType Container)) {
        throw "The Ame application bundle was not found: $resolved"
    }
    $rootItem = Get-Item -LiteralPath $resolved -Force
    if ($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
        throw "The Ame application bundle root must not be a reparse point"
    }
    $entries = @(Get-AmeBrokerNoFollowTreeEntries -RootPath $resolved)
    if ($entries.Count -eq 0 -or $entries.Count -gt 4096) {
        throw "The Ame application bundle entry count is empty or unbounded"
    }
    $totalBytes = [uint64]0
    foreach ($entry in $entries) {
        $item = $entry.Item
        if (-not $entry.IsDirectory) {
            if ([uint64]$item.Length -gt ([uint64](2GB) - $totalBytes)) {
                throw "The Ame application bundle exceeds the bounded installer payload"
            }
            $totalBytes += [uint64]$item.Length
            if ($totalBytes -gt [uint64](2GB)) {
                throw "The Ame application bundle exceeds the bounded installer payload"
            }
        }
    }
    $clientBinary = Join-Path $resolved $script:AmeApplicationBinaryName
    Assert-AmeSignedX64Binary `
        -Path $clientBinary `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    return [pscustomobject]@{
        BundlePath = $resolved
        ClientBinaryPath = $clientBinary
        EntryCount = $entries.Count
        TotalBytes = $totalBytes
    }
}

function New-AmeApplicationProtectedInstallTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $createdDirectories = @()
    try {
        $facts = Assert-AmeApplicationFixedInstallTree -InstallDirectory $InstallDirectory
        for (
            $index = $facts.FirstMissingIndex;
            $index -lt $facts.ExpectedPaths.Count;
            $index += 1
        ) {
            $directory = $facts.ExpectedPaths[$index]
            $sddl = if ($index -eq $facts.ExpectedPaths.Count - 1) {
                $script:AmeApplicationProtectedDirectorySddl
            } else {
                $script:AmeBrokerProtectedDirectorySddl
            }
            [Ame.JournalBrokerInstaller.NativeMethods]::CreateDirectoryWithSddl($directory, $sddl)
            $createdDirectories += $directory
            $facts = Assert-AmeApplicationFixedInstallTree -InstallDirectory $InstallDirectory
        }
        Set-AmeApplicationPathAcl -Path $InstallDirectory -Kind "Directory"
        Assert-AmeApplicationFixedInstallTree `
            -InstallDirectory $InstallDirectory `
            -RequireInstallDirectory | Out-Null
    } catch {
        $originalError = $_
        $rollbackFailures = @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories $createdDirectories)
        Throw-AmeBrokerTransactionFailure `
            -OriginalError $originalError `
            -RollbackFailures $rollbackFailures
    }
    return [pscustomobject]@{
        InstallDirectory = $InstallDirectory
        CreatedDirectories = $createdDirectories
    }
}

function Set-AmeApplicationTreeAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $entries = @(Get-AmeBrokerNoFollowTreeEntries -RootPath $InstallDirectory)
    if ($entries.Count -gt 4096) {
        throw "The installed Ame application tree exceeds its bounded entry count"
    }
    Set-AmeApplicationPathAcl -Path $InstallDirectory -Kind "Directory"
    foreach ($entry in $entries) {
        $kind = if ($entry.IsDirectory) { "Directory" } else { "File" }
        Set-AmeApplicationPathAcl -Path $entry.FullName -Kind $kind
    }
}

function Assert-AmeApplicationInstallation {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    Assert-AmeApplicationFixedInstallTree `
        -InstallDirectory $InstallDirectory `
        -RequireInstallDirectory | Out-Null
    $entries = @(
        [pscustomobject]@{
            FullName = (Get-Item -LiteralPath $InstallDirectory -Force).FullName
            IsDirectory = $true
        }
        Get-AmeBrokerNoFollowTreeEntries -RootPath $InstallDirectory
    )
    if ($entries.Count -le 1 -or $entries.Count -gt 4097) {
        throw "The installed Ame application tree is empty or unbounded"
    }
    foreach ($entry in $entries) {
        $expected = ConvertTo-AmeBrokerComparablePath -Path $entry.FullName
        $final = ConvertTo-AmeBrokerComparablePath -Path (
            [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($entry.FullName)
        )
        if (-not $expected.Equals($final, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "The installed Ame application entry resolves outside its fixed physical tree"
        }
        $kind = if ($entry.IsDirectory) { "Directory" } else { "File" }
        Assert-AmeApplicationExactAclFacts `
            -Facts (Get-AmeBrokerAclFacts -Path $entry.FullName) `
            -Kind $kind
    }
    $clientBinary = Join-Path $InstallDirectory $script:AmeApplicationBinaryName
    Assert-AmeBrokerPhysicalFile -ExpectedPath $clientBinary
    Assert-AmeSignedX64Binary `
        -Path $clientBinary `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    return $clientBinary
}

function Copy-AmeApplicationBundleNoFollow {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceBundlePath,
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory
    )

    $source = [IO.Path]::GetFullPath($SourceBundlePath)
    $destination = [IO.Path]::GetFullPath($InstallDirectory)
    $sourcePrefix = "$($source.TrimEnd([IO.Path]::DirectorySeparatorChar))$([IO.Path]::DirectorySeparatorChar)"
    $destinationPrefix = "$($destination.TrimEnd([IO.Path]::DirectorySeparatorChar))$([IO.Path]::DirectorySeparatorChar)"
    $entries = @(
        Get-AmeBrokerNoFollowTreeEntries -RootPath $source |
            Sort-Object { ([string]$_.FullName).Length }
    )
    foreach ($entry in $entries) {
        $live = Get-Item -LiteralPath $entry.FullName -Force
        if ($live.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "The Ame application source changed into a reparse point during copy"
        }
        $sourcePath = [IO.Path]::GetFullPath($live.FullName)
        if (-not $sourcePath.StartsWith($sourcePrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "The Ame application source escaped during copy"
        }
        $relative = $sourcePath.Substring($sourcePrefix.Length)
        $target = [IO.Path]::GetFullPath((Join-Path $destination $relative))
        if (-not $target.StartsWith(
            $destinationPrefix,
            [StringComparison]::OrdinalIgnoreCase
        )) {
            throw "The Ame application destination escaped during copy"
        }
        if ($live.PSIsContainer) {
            if (-not (Test-Path -LiteralPath $target -PathType Container)) {
                New-Item -ItemType Directory -Path $target | Out-Null
            }
        } else {
            Copy-Item -LiteralPath $sourcePath -Destination $target
        }
    }
}

function Install-AmeApplicationBundle {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceBundlePath,
        [Parameter(Mandatory = $true)]
        [string]$InstallDirectory,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $source = Assert-AmeApplicationBundleSource `
        -BundlePath $SourceBundlePath `
        -ExpectedPublisher $ExpectedPublisher
    if (Test-Path -LiteralPath $InstallDirectory) {
        throw "The fixed Ame application directory already exists"
    }
    $treeState = New-AmeApplicationProtectedInstallTree -InstallDirectory $InstallDirectory
    try {
        Copy-AmeApplicationBundleNoFollow `
            -SourceBundlePath $source.BundlePath `
            -InstallDirectory $InstallDirectory
        Set-AmeApplicationTreeAcl -InstallDirectory $InstallDirectory
        $clientBinary = Assert-AmeApplicationInstallation `
            -InstallDirectory $InstallDirectory `
            -ExpectedPublisher $ExpectedPublisher
    } catch {
        $originalError = $_
        $actions = @(
            New-AmeBrokerRollbackAction `
                -Name "remove owned Application bundle" `
                -Action {
                    if (Test-Path -LiteralPath $InstallDirectory -PathType Container) {
                        Assert-AmeApplicationFixedInstallTree `
                            -InstallDirectory $InstallDirectory `
                            -RequireInstallDirectory | Out-Null
                        Remove-AmeBrokerOwnedTreeNoFollow `
                            -Path $InstallDirectory `
                            -ExpectedParent (Get-AmeProductParentDirectory)
                    }
                }
        )
        $rollbackFailures = @(Invoke-AmeBrokerRollbackActions -Actions $actions)
        $parentDirectories = @($treeState.CreatedDirectories | Where-Object {
            -not $_.Equals($InstallDirectory, [StringComparison]::OrdinalIgnoreCase)
        })
        $rollbackFailures += @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories $parentDirectories)
        Throw-AmeBrokerTransactionFailure `
            -OriginalError $originalError `
            -RollbackFailures $rollbackFailures
    }
    return [pscustomobject]@{
        ClientBinaryPath = $clientBinary
        CreatedDirectories = $treeState.CreatedDirectories
    }
}

function Assert-AmeBrokerServiceDaclFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ActualSddl
    )

    $compact = [regex]::Replace($ActualSddl, '\s', '')
    if (-not $compact.Equals(
        $script:AmeBrokerServiceDacl,
        [System.StringComparison]::Ordinal
    )) {
        throw "The broker service DACL does not exactly match the admitted descriptor"
    }
}

function Assert-AmeBrokerServiceConfiguration {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$IdentityManifest
    )

    Assert-AmeBrokerIdentityManifestFacts -Manifest $IdentityManifest

    $serviceKeyPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$script:AmeBrokerServiceName"
    $configuration = Get-ItemProperty -LiteralPath $serviceKeyPath
    $expectedImagePath = '"{0}"' -f ([System.IO.Path]::GetFullPath($BinaryPath))
    if ([string]$configuration.ImagePath -cne $expectedImagePath) {
        throw "The broker service binary path does not match the admitted installation"
    }
    if ([int]$configuration.Type -ne 0x10 -or [int]$configuration.Start -ne 3) {
        throw "The broker service must be own-process and demand-start"
    }
    if ([string]$configuration.ObjectName -cne "LocalSystem") {
        throw "The broker service account must be LocalSystem"
    }
    if ([int]$configuration.ServiceSidType -ne 3) {
        throw "The broker service SID must be restricted"
    }
    if ([string]$configuration.Description -cne $IdentityManifest) {
        throw "The broker service identity manifest does not match the installed binary"
    }
    $resolvedServiceSid = ([Security.Principal.NTAccount]::new(
        "NT SERVICE\$script:AmeBrokerServiceName"
    )).Translate([Security.Principal.SecurityIdentifier]).Value
    Assert-AmeBrokerServiceSidFacts -ActualSid $resolvedServiceSid
    $privileges = @($configuration.RequiredPrivileges)
    if (
        $privileges.Count -ne 1 -or
        [string]$privileges[0] -cne $script:AmeBrokerRequiredPrivilege
    ) {
        throw "The broker service privilege allowlist must contain only SeManageVolumePrivilege"
    }
    $serviceDacl = [Ame.JournalBrokerInstaller.NativeMethods]::GetServiceSecuritySddl(
        $script:AmeBrokerServiceName
    )
    Assert-AmeBrokerServiceDaclFacts -ActualSddl $serviceDacl
}

function Assert-AmeBrokerInstallation {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$ClientBinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $installDirectory = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($BinaryPath))
    $applicationDirectory = [IO.Path]::GetDirectoryName(
        [IO.Path]::GetFullPath($ClientBinaryPath)
    )
    $verifiedClientBinary = Assert-AmeApplicationInstallation `
        -InstallDirectory $applicationDirectory `
        -ExpectedPublisher $ExpectedPublisher
    if (-not $verifiedClientBinary.Equals(
        ([System.IO.Path]::GetFullPath($ClientBinaryPath)),
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The broker client identity does not match the installed Ame application"
    }
    Assert-AmeBrokerFixedInstallTree `
        -InstallDirectory $installDirectory `
        -RequireInstallDirectory | Out-Null
    Assert-AmeBrokerPhysicalFile -ExpectedPath $BinaryPath
    Assert-AmeBrokerBinary -Path $BinaryPath -ExpectedPublisher $ExpectedPublisher | Out-Null
    $identityManifest = New-AmeBrokerIdentityManifest `
        -BinaryPath $BinaryPath `
        -ClientBinaryPath $ClientBinaryPath `
        -ExpectedPublisher $ExpectedPublisher
    Assert-AmeBrokerServiceConfiguration `
        -BinaryPath $BinaryPath `
        -IdentityManifest $identityManifest
    Assert-AmeBrokerBinaryAcl `
        -InstallDirectory $installDirectory `
        -BinaryPath $BinaryPath
}
