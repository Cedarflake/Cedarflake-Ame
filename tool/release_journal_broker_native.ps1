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
