$script:AmeBrokerAcceptanceToken = "CEDARFLAKE_AME_WINDOWS_JOURNAL_BROKER_ACCEPTANCE_V1"

if (-not ("Ame.JournalBrokerAcceptance.NativeMethods" -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.IO;
using Microsoft.Win32.SafeHandles;
using System.Runtime.InteropServices;
using System.Text;

namespace Ame.JournalBrokerAcceptance
{
    public sealed class ClientProcessFacts
    {
        public uint ProcessId { get; set; }
        public string ImagePath { get; set; }
        public bool IsElevated { get; set; }
        public long CreationTimeUtcFileTime { get; set; }
    }

    public static class NativeMethods
    {
        private const uint PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;
        private const uint TOKEN_QUERY = 0x0008;
        private const int TokenElevation = 20;
        private const uint DELETE = 0x00010000;
        private const uint FILE_READ_ATTRIBUTES = 0x00000080;
        private const uint SYNCHRONIZE = 0x00100000;
        private const uint FILE_SHARE_READ = 0x00000001;
        private const uint FILE_SHARE_WRITE = 0x00000002;
        private const uint FILE_SHARE_DELETE = 0x00000004;
        private const uint OPEN_EXISTING = 3;
        private const uint FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000;
        private const uint FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
        private const uint FILE_ATTRIBUTE_DIRECTORY = 0x00000010;
        private const uint FILE_ATTRIBUTE_REPARSE_POINT = 0x00000400;
        private const int FileDispositionInfo = 4;
        private const int FileAttributeTagInfo = 9;
        private static readonly IntPtr INVALID_HANDLE_VALUE = new IntPtr(-1);

        [StructLayout(LayoutKind.Sequential)]
        private struct TOKEN_ELEVATION
        {
            public int TokenIsElevated;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct FILETIME
        {
            public uint Low;
            public uint High;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct FILE_ATTRIBUTE_TAG_INFO
        {
            public uint FileAttributes;
            public uint ReparseTag;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct FILE_DISPOSITION_INFO
        {
            [MarshalAs(UnmanagedType.Bool)]
            public bool DeleteFile;
        }

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool GetNamedPipeClientProcessId(
            IntPtr pipe,
            out uint clientProcessId
        );

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr OpenProcess(
            uint desiredAccess,
            bool inheritHandle,
            uint processId
        );

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool QueryFullProcessImageName(
            IntPtr process,
            uint flags,
            StringBuilder path,
            ref uint pathLength
        );

        [DllImport("advapi32.dll", SetLastError = true)]
        private static extern bool OpenProcessToken(
            IntPtr process,
            uint desiredAccess,
            out IntPtr token
        );

        [DllImport("advapi32.dll", SetLastError = true)]
        private static extern bool GetTokenInformation(
            IntPtr token,
            int informationClass,
            out TOKEN_ELEVATION information,
            int informationLength,
            out int returnLength
        );

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool GetProcessTimes(
            IntPtr process,
            out FILETIME creation,
            out FILETIME exit,
            out FILETIME kernel,
            out FILETIME user
        );

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CreateFile(
            string fileName,
            uint desiredAccess,
            uint shareMode,
            IntPtr securityAttributes,
            uint creationDisposition,
            uint flagsAndAttributes,
            IntPtr templateFile
        );

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool GetFileInformationByHandleEx(
            IntPtr file,
            int informationClass,
            out FILE_ATTRIBUTE_TAG_INFO information,
            uint bufferSize
        );

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern uint GetFinalPathNameByHandle(
            IntPtr file,
            StringBuilder path,
            uint pathLength,
            uint flags
        );

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern uint GetLongPathName(
            string shortPath,
            StringBuilder longPath,
            uint bufferLength
        );

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool SetFileInformationByHandle(
            IntPtr file,
            int informationClass,
            ref FILE_DISPOSITION_INFO information,
            uint bufferSize
        );

        [DllImport("kernel32.dll")]
        private static extern bool CloseHandle(IntPtr handle);

        public static ClientProcessFacts GetClientProcessFacts(IntPtr pipe)
        {
            uint processId;
            if (!GetNamedPipeClientProcessId(pipe, out processId) || processId == 0)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            IntPtr process = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                processId
            );
            if (process == IntPtr.Zero)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            IntPtr token = IntPtr.Zero;
            try
            {
                StringBuilder path = new StringBuilder(32768);
                uint pathLength = (uint)path.Capacity;
                if (!QueryFullProcessImageName(process, 0, path, ref pathLength))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                if (!OpenProcessToken(process, TOKEN_QUERY, out token))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                TOKEN_ELEVATION elevation;
                int returned;
                if (!GetTokenInformation(
                    token,
                    TokenElevation,
                    out elevation,
                    Marshal.SizeOf(typeof(TOKEN_ELEVATION)),
                    out returned
                ))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                FILETIME creation;
                FILETIME exit;
                FILETIME kernel;
                FILETIME user;
                if (!GetProcessTimes(process, out creation, out exit, out kernel, out user))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                long creationTime = ((long)creation.High << 32) | creation.Low;
                return new ClientProcessFacts
                {
                    ProcessId = processId,
                    ImagePath = path.ToString(),
                    IsElevated = elevation.TokenIsElevated != 0,
                    CreationTimeUtcFileTime = creationTime
                };
            }
            finally
            {
                if (token != IntPtr.Zero)
                {
                    CloseHandle(token);
                }
                CloseHandle(process);
            }
        }

        private static string NormalizeFinalPath(string path)
        {
            string normalized = path;
            if (normalized.StartsWith(@"\\?\UNC\", StringComparison.OrdinalIgnoreCase))
            {
                normalized = @"\\" + normalized.Substring(8);
            }
            else if (normalized.StartsWith(@"\\?\", StringComparison.OrdinalIgnoreCase))
            {
                normalized = normalized.Substring(4);
            }
            normalized = Path.GetFullPath(normalized);
            StringBuilder longPath = new StringBuilder(32768);
            uint length = GetLongPathName(normalized, longPath, (uint)longPath.Capacity);
            if (length > 0 && length < longPath.Capacity)
            {
                normalized = longPath.ToString();
            }
            return normalized.TrimEnd(Path.DirectorySeparatorChar);
        }

        private static string GetFinalPath(IntPtr file)
        {
            StringBuilder path = new StringBuilder(32768);
            uint length = GetFinalPathNameByHandle(file, path, (uint)path.Capacity, 0);
            if (length == 0 || length >= path.Capacity)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            return NormalizeFinalPath(path.ToString());
        }

        public static void DeletePhysicalEntry(
            string path,
            string expectedFinalPath,
            bool expectDirectory
        )
        {
            string expected = NormalizeFinalPath(expectedFinalPath);
            IntPtr file = CreateFile(
                path,
                DELETE | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                IntPtr.Zero,
                OPEN_EXISTING,
                FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
                IntPtr.Zero
            );
            if (file == IntPtr.Zero || file == INVALID_HANDLE_VALUE)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            try
            {
                FILE_ATTRIBUTE_TAG_INFO attributes;
                if (!GetFileInformationByHandleEx(
                    file,
                    FileAttributeTagInfo,
                    out attributes,
                    (uint)Marshal.SizeOf(typeof(FILE_ATTRIBUTE_TAG_INFO))
                ))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                bool isDirectory =
                    (attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
                if ((attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0)
                {
                    throw new InvalidOperationException(
                        "Acceptance cleanup refuses a reparse-point handle"
                    );
                }
                if (isDirectory != expectDirectory)
                {
                    throw new InvalidOperationException(
                        "Acceptance cleanup entry kind changed before deletion"
                    );
                }
                string actual = GetFinalPath(file);
                if (!actual.Equals(expected, StringComparison.OrdinalIgnoreCase))
                {
                    throw new InvalidOperationException(
                        "Acceptance cleanup handle escaped its exact path"
                    );
                }
                FILE_DISPOSITION_INFO disposition = new FILE_DISPOSITION_INFO
                {
                    DeleteFile = true
                };
                if (!SetFileInformationByHandle(
                    file,
                    FileDispositionInfo,
                    ref disposition,
                    (uint)Marshal.SizeOf(typeof(FILE_DISPOSITION_INFO))
                ))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
            }
            finally
            {
                CloseHandle(file);
            }
        }

        public static SafeFileHandle OpenPinnedPhysicalDirectory(string path)
        {
            IntPtr file = CreateFile(
                path,
                FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                IntPtr.Zero,
                OPEN_EXISTING,
                FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
                IntPtr.Zero
            );
            if (file == IntPtr.Zero || file == INVALID_HANDLE_VALUE)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            try
            {
                FILE_ATTRIBUTE_TAG_INFO attributes;
                if (!GetFileInformationByHandleEx(
                    file,
                    FileAttributeTagInfo,
                    out attributes,
                    (uint)Marshal.SizeOf(typeof(FILE_ATTRIBUTE_TAG_INFO))
                ))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                if ((attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0 ||
                    (attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0)
                {
                    throw new InvalidOperationException(
                        "Acceptance workspace pin requires a physical directory"
                    );
                }
                SafeFileHandle owner = new SafeFileHandle(file, true);
                file = IntPtr.Zero;
                return owner;
            }
            finally
            {
                if (file != IntPtr.Zero)
                {
                    CloseHandle(file);
                }
            }
        }

        public static string GetPinnedFinalPath(SafeFileHandle directory)
        {
            if (directory == null || directory.IsInvalid || directory.IsClosed)
            {
                throw new InvalidOperationException("Acceptance workspace pin is unavailable");
            }
            return GetFinalPath(directory.DangerousGetHandle());
        }
    }
}
'@
}

function New-AmeBrokerAcceptanceRandomHex {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateRange(16, 32)]
        [int]$ByteCount
    )

    $bytes = [byte[]]::new($ByteCount)
    $generator = [Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $generator.GetBytes($bytes)
    } finally {
        $generator.Dispose()
    }
    return ([BitConverter]::ToString($bytes)).Replace("-", "")
}

function Get-AmeBrokerAcceptanceClientProtocolFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $resolved = [IO.Path]::GetFullPath($Path)
    $output = @(& $resolved "--acceptance-protocol-info" 2>&1)
    if (
        $LASTEXITCODE -ne 0 -or
        $output.Count -ne 1 -or
        [string]$output[0] -cne `
            "binary=cedarflake_ame_broker_acceptance_client protocol=1"
    ) {
        throw "The dedicated broker acceptance client protocol is incompatible"
    }
}

function Assert-AmeBrokerAcceptanceClientBinary {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $resolved = Assert-AmeSignedX64Binary `
        -Path $Path `
        -ExpectedPublisher $ExpectedPublisher
    Get-AmeBrokerAcceptanceClientProtocolFacts -Path $resolved
    return $resolved
}

function New-AmeBrokerLimitedResultServer {
    param(
        [Parameter(Mandatory = $true)]
        [ValidatePattern('\ACedarflakeAme\.Acceptance\.[0-9A-F]{32}\z')]
        [string]$PipeName,
        [Parameter(Mandatory = $true)]
        [Security.Principal.SecurityIdentifier]$UserSid
    )

    $security = [System.IO.Pipes.PipeSecurity]::new()
    $security.SetAccessRuleProtection($true, $false)
    foreach ($rule in @(
        [System.IO.Pipes.PipeAccessRule]::new(
            [Security.Principal.SecurityIdentifier]::new("S-1-5-18"),
            [System.IO.Pipes.PipeAccessRights]::FullControl,
            [Security.AccessControl.AccessControlType]::Allow
        ),
        [System.IO.Pipes.PipeAccessRule]::new(
            [Security.Principal.SecurityIdentifier]::new("S-1-5-32-544"),
            [System.IO.Pipes.PipeAccessRights]::FullControl,
            [Security.AccessControl.AccessControlType]::Allow
        ),
        [System.IO.Pipes.PipeAccessRule]::new(
            $UserSid,
            [System.IO.Pipes.PipeAccessRights]::ReadWrite,
            [Security.AccessControl.AccessControlType]::Allow
        )
    )) {
        $security.AddAccessRule($rule)
    }
    return [System.IO.Pipes.NamedPipeServerStream]::new(
        $PipeName,
        [System.IO.Pipes.PipeDirection]::InOut,
        1,
        [System.IO.Pipes.PipeTransmissionMode]::Byte,
        [System.IO.Pipes.PipeOptions]::Asynchronous,
        4096,
        65536,
        $security,
        [System.IO.HandleInheritability]::None
    )
}

function Read-AmeBrokerPipeExact {
    param(
        [Parameter(Mandatory = $true)]
        [System.IO.Pipes.NamedPipeServerStream]$Pipe,
        [Parameter(Mandatory = $true)]
        [ValidateRange(1, 65536)]
        [int]$Length,
        [int]$TimeoutMilliseconds = 10000
    )

    $buffer = [byte[]]::new($Length)
    $offset = 0
    while ($offset -lt $buffer.Length) {
        $pending = $Pipe.BeginRead($buffer, $offset, $buffer.Length - $offset, $null, $null)
        try {
            if (-not $pending.AsyncWaitHandle.WaitOne($TimeoutMilliseconds)) {
                throw "The protected limited-client result read timed out"
            }
            $read = $Pipe.EndRead($pending)
        } finally {
            $pending.AsyncWaitHandle.Dispose()
        }
        if ($read -le 0) {
            throw "The protected limited-client result ended before its declared length"
        }
        $offset += $read
    }
    return $buffer
}

function Read-AmeBrokerLimitedResultRecord {
    param(
        [Parameter(Mandatory = $true)]
        [System.IO.Pipes.NamedPipeServerStream]$Pipe
    )

    $lengthBytes = Read-AmeBrokerPipeExact -Pipe $Pipe -Length 4
    $length = [BitConverter]::ToUInt32($lengthBytes, 0)
    if ($length -eq 0 -or $length -gt 65536) {
        throw "The protected limited-client result length is invalid"
    }
    $recordBytes = Read-AmeBrokerPipeExact -Pipe $Pipe -Length ([int]$length)
    return [Text.UTF8Encoding]::new($false, $true).GetString($recordBytes)
}

function Assert-AmeBrokerLimitedResultFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Record,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedNonce,
        [Parameter(Mandatory = $true)]
        [ValidatePattern('\A(first|second)\z')]
        [string]$ExpectedPhase,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedInstance,
        [Parameter(Mandatory = $true)]
        [uint32]$ExpectedProcessId
    )

    $match = [regex]::Match(
        $Record,
        '^AME_BROKER_LIMITED_RESULT_V1 nonce=([0-9A-F]{64}) ' +
            'phase=(first|second) instance=([0-9A-F]{32}) pid=([1-9][0-9]{0,9}) ' +
            'status=(passed|failed) payload_hex=([0-9a-f]*)$',
        [Text.RegularExpressions.RegexOptions]::CultureInvariant -bor
            [Text.RegularExpressions.RegexOptions]::IgnoreCase
    )
    if (-not $match.Success) {
        throw "The protected limited-client result record is malformed"
    }
    $processId = [uint32]::Parse(
        $match.Groups[4].Value,
        [Globalization.CultureInfo]::InvariantCulture
    )
    if (
        $match.Groups[1].Value -cne $ExpectedNonce -or
        $match.Groups[2].Value -cne $ExpectedPhase -or
        $match.Groups[3].Value -cne $ExpectedInstance -or
        $processId -ne $ExpectedProcessId
    ) {
        throw "The protected limited-client result is not bound to this task instance"
    }
    $payloadHex = $match.Groups[6].Value
    if ($payloadHex.Length % 2 -ne 0 -or $payloadHex.Length -gt 131072) {
        throw "The protected limited-client payload is invalid"
    }
    $payloadBytes = [byte[]]::new($payloadHex.Length / 2)
    for ($index = 0; $index -lt $payloadBytes.Length; $index += 1) {
        $payloadBytes[$index] = [Convert]::ToByte($payloadHex.Substring($index * 2, 2), 16)
    }
    return [pscustomobject]@{
        Status = $match.Groups[5].Value.ToLowerInvariant()
        Payload = [Text.UTF8Encoding]::new($false, $true).GetString($payloadBytes)
        ProcessId = $processId
    }
}

function Assert-AmeBrokerLimitedClientFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ActualPath,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPath,
        [Parameter(Mandatory = $true)]
        [bool]$IsElevated,
        [Parameter(Mandatory = $true)]
        [string]$ActualSha256,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedSha256,
        [Parameter(Mandatory = $true)]
        [Int64]$CreationTimeUtcFileTime,
        [Parameter(Mandatory = $true)]
        [Int64]$StartedAfterUtcFileTime
    )

    if (
        -not ([IO.Path]::GetFullPath($ActualPath)).Equals(
            [IO.Path]::GetFullPath($ExpectedPath),
            [StringComparison]::OrdinalIgnoreCase
        ) -or
        $IsElevated -or
        $ActualSha256 -cne $ExpectedSha256 -or
        $CreationTimeUtcFileTime -lt $StartedAfterUtcFileTime
    ) {
        throw "The result pipe client is not the exact protected limited task binary"
    }
}

function Assert-AmeBrokerTaskCompletionFacts {
    param(
        [Parameter(Mandatory = $true)]
        [uint32]$LastTaskResult,
        [Parameter(Mandatory = $true)]
        [DateTime]$LastRunTimeUtc,
        [Parameter(Mandatory = $true)]
        [DateTime]$StartedAfterUtc
    )

    if ($LastTaskResult -ne 0 -or $LastRunTimeUtc -lt $StartedAfterUtc) {
        throw "The exact limited-client task instance did not complete successfully"
    }
}

function Assert-AmeBrokerAcceptanceToken {
    param(
        [Parameter(Mandatory = $true)]
        [string]$AuthorizationToken
    )

    if ($AuthorizationToken -cne $script:AmeBrokerAcceptanceToken) {
        throw "The exact current Windows journal broker acceptance token is required"
    }
}

function Get-AmeBrokerAcceptanceFullPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "Acceptance paths must not be empty"
    }
    return [System.IO.Path]::GetFullPath($Path).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
}

function Assert-AmeBrokerAcceptanceAncestorFacts {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Facts
    )

    if ($Facts.Count -lt 2) {
        throw "The disposable workspace ancestor proof is incomplete"
    }
    foreach ($fact in $Facts) {
        if (
            -not $fact.IsDirectory -or
            $fact.IsReparsePoint -or
            -not ([string]$fact.ExpectedPath).Equals(
                [string]$fact.FinalPath,
                [StringComparison]::OrdinalIgnoreCase
            )
        ) {
            throw "A disposable workspace ancestor is redirected or not a physical directory"
        }
    }
}

function Get-AmeBrokerAcceptanceAncestorFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $candidate = ConvertTo-AmeBrokerComparablePath -Path $Path
    $root = [IO.Path]::GetPathRoot($candidate)
    if ([string]::IsNullOrWhiteSpace($root) -or $root.StartsWith("\")) {
        throw "The disposable workspace must use a local physical volume"
    }
    $paths = @($root)
    $current = $root
    foreach ($component in $candidate.Substring($root.Length).Split(
        [IO.Path]::DirectorySeparatorChar,
        [StringSplitOptions]::RemoveEmptyEntries
    )) {
        $current = Join-Path $current $component
        $paths += $current
    }
    $facts = @(
        for ($index = 0; $index -lt $paths.Count; $index += 1) {
            Get-AmeBrokerDirectoryIdentityFacts `
                -ExpectedPath $paths[$index] `
                -IsVolumeRoot ($index -eq 0)
        }
    )
    Assert-AmeBrokerAcceptanceAncestorFacts -Facts $facts
    return $facts
}

function Assert-AmeBrokerDisposablePathFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$AcceptanceWorkspace,
        [Parameter(Mandatory = $true)]
        [string]$DisposableRoot,
        [Parameter(Mandatory = $true)]
        [string]$ExternalSiblingRoot
    )

    $workspace = Get-AmeBrokerAcceptanceFullPath -Path $AcceptanceWorkspace
    if (-not (Test-Path -LiteralPath $workspace -PathType Container)) {
        throw "The disposable acceptance workspace must be a pre-created empty directory"
    }
    $workspaceItem = Get-Item -LiteralPath $workspace -Force
    if ($workspaceItem.Attributes.HasFlag([System.IO.FileAttributes]::ReparsePoint)) {
        throw "The disposable acceptance workspace must not be a reparse point"
    }
    if (@(Get-ChildItem -LiteralPath $workspace -Force).Count -ne 0) {
        throw "The disposable acceptance workspace must be empty"
    }
    $temporaryRoot = Get-AmeBrokerAcceptanceFullPath -Path ([System.IO.Path]::GetTempPath())
    $temporaryPrefix = "$temporaryRoot$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $workspace.StartsWith(
        $temporaryPrefix,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The disposable acceptance workspace must be below the current temporary directory"
    }
    $root = Get-AmeBrokerAcceptanceFullPath -Path $DisposableRoot
    $external = Get-AmeBrokerAcceptanceFullPath -Path $ExternalSiblingRoot
    foreach ($candidate in @($root, $external)) {
        $parent = Split-Path -Parent $candidate
        if (-not $parent.Equals($workspace, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Every disposable root must be a direct workspace child"
        }
        if (Test-Path -LiteralPath $candidate) {
            throw "Disposable roots must not exist before acceptance"
        }
    }
    if ($root.Equals($external, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Disposable root and external sibling paths must be distinct"
    }
    return [pscustomobject]@{
        Workspace = $workspace
        Root = $root
        External = $external
    }
}

function Get-AmeBrokerAcceptanceWorkspaceAclPrestate {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [Security.Principal.SecurityIdentifier]$UserSid
    )

    $resolved = Get-AmeBrokerAcceptanceFullPath -Path $Path
    $item = Get-Item -LiteralPath $resolved -Force
    if (-not $item.PSIsContainer -or
        $item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
        throw "The acceptance workspace must remain one physical directory"
    }
    if ($UserSid.Value -in @("S-1-5-18", "S-1-5-32-544")) {
        throw "The limited acceptance identity must be an ordinary interactive user"
    }
    $root = [IO.Path]::GetPathRoot($resolved)
    $ancestorPaths = @($root)
    $current = $root
    foreach ($component in $resolved.Substring($root.Length).Split(
        [IO.Path]::DirectorySeparatorChar,
        [StringSplitOptions]::RemoveEmptyEntries
    )) {
        $current = Join-Path $current $component
        $ancestorPaths += $current
    }
    $pins = [Collections.Generic.List[Microsoft.Win32.SafeHandles.SafeFileHandle]]::new()
    try {
        foreach ($ancestorPath in $ancestorPaths) {
            $pins.Add(
                [Ame.JournalBrokerAcceptance.NativeMethods]::OpenPinnedPhysicalDirectory(
                    $ancestorPath
                )
            )
        }
        $finalPath = [Ame.JournalBrokerAcceptance.NativeMethods]::GetPinnedFinalPath(
            $pins[$pins.Count - 1]
        )
        return [pscustomobject]@{
            Path = $resolved
            FinalPath = $finalPath
            Sddl = [string](Get-Acl -LiteralPath $resolved).Sddl
            UserSid = $UserSid.Value
            PinnedAncestorHandles = $pins.ToArray()
        }
    } catch {
        $pinArray = $pins.ToArray()
        for ($index = $pinArray.Count - 1; $index -ge 0; $index -= 1) {
            $pinArray[$index].Dispose()
        }
        throw
    }
}

function Close-AmeBrokerAcceptanceWorkspacePins {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate
    )

    $failures = [Collections.Generic.List[string]]::new()
    $pins = @($Prestate.PinnedAncestorHandles)
    for ($index = $pins.Count - 1; $index -ge 0; $index -= 1) {
        try {
            $pins[$index].Dispose()
        } catch {
            $failures.Add($_.Exception.Message)
        }
    }
    if ($failures.Count -gt 0) {
        throw "Acceptance workspace pin cleanup failed: $($failures -join '; ')"
    }
}

function Assert-AmeBrokerAcceptanceWorkspaceAcl {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [Security.Principal.SecurityIdentifier]$UserSid
    )

    $facts = Get-AmeBrokerAclFacts -Path $Path
    if (-not $facts.AreAccessRulesProtected -or
        [string]$facts.OwnerSid -cne "S-1-5-32-544" -or
        [string]$facts.GroupSid -cne "S-1-5-32-544" -or
        @($facts.Rules).Count -ne 3) {
        throw "The acceptance workspace ACL is not the exact protected allowlist"
    }
    $expectedRights = @{
        "S-1-5-18" = [uint64][Security.AccessControl.FileSystemRights]::FullControl
        "S-1-5-32-544" = [uint64][Security.AccessControl.FileSystemRights]::FullControl
        $UserSid.Value = [uint64][Security.AccessControl.FileSystemRights]::ReadAndExecute
    }
    foreach ($rule in @($facts.Rules)) {
        $sid = [string]$rule.Sid
        if (-not $expectedRights.ContainsKey($sid) -or
            [string]$rule.AccessControlType -cne "Allow" -or
            [uint64]$rule.Rights -ne [uint64]$expectedRights[$sid] -or
            [int]$rule.InheritanceFlags -ne 3 -or
            [int]$rule.PropagationFlags -ne 0 -or
            [bool]$rule.IsInherited) {
            throw "The acceptance workspace ACL grants unexpected authority"
        }
    }
}

function Set-AmeBrokerAcceptanceWorkspaceAcl {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate
    )

    $administrators = [Security.Principal.SecurityIdentifier]::new("S-1-5-32-544")
    $system = [Security.Principal.SecurityIdentifier]::new("S-1-5-18")
    $user = [Security.Principal.SecurityIdentifier]::new([string]$Prestate.UserSid)
    $inheritance = (
        [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor
        [Security.AccessControl.InheritanceFlags]::ObjectInherit
    )
    $security = [Security.AccessControl.DirectorySecurity]::new()
    $security.SetOwner($administrators)
    $security.SetGroup($administrators)
    $security.SetAccessRuleProtection($true, $false)
    foreach ($rule in @(
        [Security.AccessControl.FileSystemAccessRule]::new(
            $system,
            [Security.AccessControl.FileSystemRights]::FullControl,
            $inheritance,
            [Security.AccessControl.PropagationFlags]::None,
            [Security.AccessControl.AccessControlType]::Allow
        ),
        [Security.AccessControl.FileSystemAccessRule]::new(
            $administrators,
            [Security.AccessControl.FileSystemRights]::FullControl,
            $inheritance,
            [Security.AccessControl.PropagationFlags]::None,
            [Security.AccessControl.AccessControlType]::Allow
        ),
        [Security.AccessControl.FileSystemAccessRule]::new(
            $user,
            [Security.AccessControl.FileSystemRights]::ReadAndExecute,
            $inheritance,
            [Security.AccessControl.PropagationFlags]::None,
            [Security.AccessControl.AccessControlType]::Allow
        )
    )) {
        $security.AddAccessRule($rule) | Out-Null
    }
    Set-Acl -LiteralPath ([string]$Prestate.Path) -AclObject $security
    $liveFinalPath = [Ame.JournalBrokerAcceptance.NativeMethods]::GetPinnedFinalPath(
        $Prestate.PinnedAncestorHandles[$Prestate.PinnedAncestorHandles.Count - 1]
    )
    if (-not $liveFinalPath.Equals(
        [string]$Prestate.FinalPath,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The acceptance workspace changed identity while its ACL was protected"
    }
    Assert-AmeBrokerAcceptanceWorkspaceAcl `
        -Path ([string]$Prestate.Path) `
        -UserSid $user
}

function Restore-AmeBrokerAcceptanceWorkspaceAcl {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate
    )

    $security = [Security.AccessControl.DirectorySecurity]::new()
    $liveFinalPath = [Ame.JournalBrokerAcceptance.NativeMethods]::GetPinnedFinalPath(
        $Prestate.PinnedAncestorHandles[$Prestate.PinnedAncestorHandles.Count - 1]
    )
    if (-not $liveFinalPath.Equals(
        [string]$Prestate.FinalPath,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The acceptance workspace changed identity before ACL restoration"
    }
    $security.SetSecurityDescriptorSddlForm([string]$Prestate.Sddl)
    Set-Acl -LiteralPath ([string]$Prestate.Path) -AclObject $security
    if (-not ([string](Get-Acl -LiteralPath ([string]$Prestate.Path)).Sddl).Equals(
        [string]$Prestate.Sddl,
        [StringComparison]::Ordinal
    )) {
        throw "The acceptance workspace ACL prestate was not restored exactly"
    }
}

function Assert-AmeBrokerProtectedSourceFacts {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Facts
    )

    $trustedOwnerSids = @(
        "S-1-5-18",
        "S-1-5-32-544",
        "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"
    )
    $dangerousMask = [uint64]0x500D0156
    $volumeRootDangerousMask = [uint64]0x000D0040
    if ($Facts.Count -lt 3) {
        throw "The pre-signed acceptance bundle physical proof is incomplete"
    }
    foreach ($fact in $Facts) {
        if (
            $fact.IsReparsePoint -or
            -not ([string]$fact.ExpectedPath).Equals(
                [string]$fact.FinalPath,
                [StringComparison]::OrdinalIgnoreCase
            )
        ) {
            throw "The pre-signed acceptance bundle contains a redirected path"
        }
        if ($trustedOwnerSids -notcontains [string]$fact.Acl.OwnerSid) {
            throw "The pre-signed acceptance bundle has an untrusted owner"
        }
        foreach ($rule in @($fact.Acl.Rules)) {
            $isInheritOnly = ([int]$rule.PropagationFlags -band 2) -ne 0
            if (
                [string]$rule.AccessControlType -cne "Allow" -or
                $isInheritOnly -or
                $trustedOwnerSids -contains [string]$rule.Sid
            ) {
                continue
            }
            $applicableMask = if ([bool]$fact.IsVolumeRoot) {
                $volumeRootDangerousMask
            } else {
                $dangerousMask
            }
            if (([uint64]$rule.Rights -band $applicableMask) -ne 0) {
                throw "The pre-signed acceptance bundle grants an untrusted mutation right"
            }
        }
    }
}

function Get-AmeBrokerNoFollowEntries {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootPath,
        [int]$MaximumEntries = 4096
    )

    $pending = [System.Collections.Generic.Stack[string]]::new()
    $pending.Push([IO.Path]::GetFullPath($RootPath))
    $entries = [System.Collections.Generic.List[object]]::new()
    while ($pending.Count -gt 0) {
        $directory = $pending.Pop()
        foreach ($entry in @(Get-ChildItem -LiteralPath $directory -Force)) {
            if ($entries.Count -ge $MaximumEntries) {
                throw "The pre-signed acceptance bundle entry count is unbounded"
            }
            if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "The pre-signed acceptance bundle must not contain reparse points"
            }
            $entries.Add($entry)
            if ($entry.PSIsContainer) {
                $pending.Push($entry.FullName)
            }
        }
    }
    return @($entries)
}

function Assert-AmeBrokerPretrustedAcceptanceBundle {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BundlePath,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $bundle = Get-AmeBrokerAcceptanceFullPath -Path $BundlePath
    if (-not (Test-Path -LiteralPath $bundle -PathType Container)) {
        throw "The externally pre-signed acceptance bundle does not exist"
    }
    $applicationBundle = Join-Path $bundle "Application"
    $brokerDirectory = Join-Path $bundle "Broker"
    $brokerBinary = Join-Path $brokerDirectory "cedarflake_ame_journal_broker.exe"
    if (-not (Test-Path -LiteralPath $applicationBundle -PathType Container) -or
        -not (Test-Path -LiteralPath $brokerDirectory -PathType Container) -or
        -not (Test-Path -LiteralPath $brokerBinary -PathType Leaf)) {
        throw "The pre-signed bundle must contain Application and Broker payloads"
    }

    $ancestorFacts = @(Get-AmeBrokerAcceptanceAncestorFacts -Path $bundle)
    $payloadEntries = @(Get-AmeBrokerNoFollowEntries -RootPath $bundle)
    $payloadFacts = @(
        Get-AmeBrokerDirectoryIdentityFacts -ExpectedPath $bundle -IsVolumeRoot $false
        foreach ($entry in $payloadEntries) {
            $finalPath = [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($entry.FullName)
            [pscustomobject]@{
                ExpectedPath = ConvertTo-AmeBrokerComparablePath -Path $entry.FullName
                FinalPath = ConvertTo-AmeBrokerComparablePath -Path $finalPath
                IsReparsePoint = [bool]($entry.Attributes -band [IO.FileAttributes]::ReparsePoint)
                Acl = Get-AmeBrokerAclFacts -Path $entry.FullName
            }
        }
    )
    Assert-AmeBrokerProtectedSourceFacts -Facts @($ancestorFacts + $payloadFacts)
    $application = Assert-AmeApplicationBundleSource `
        -BundlePath $applicationBundle `
        -ExpectedPublisher $ExpectedPublisher
    Assert-AmeBrokerBinary `
        -Path $brokerBinary `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    Assert-AmeBrokerAcceptanceClientBinary `
        -Path $application.ClientBinaryPath `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    $protocol = Get-AmeBrokerProtocolFacts -Path $brokerBinary
    return [pscustomobject]@{
        BundlePath = $bundle
        ApplicationBundlePath = $applicationBundle
        ApplicationClientPath = $application.ClientBinaryPath
        BrokerBinaryPath = $brokerBinary
        BrokerSha256 = (
            Get-FileHash -LiteralPath $brokerBinary -Algorithm SHA256
        ).Hash.ToUpperInvariant()
        ClientSha256 = (
            Get-FileHash -LiteralPath $application.ClientBinaryPath -Algorithm SHA256
        ).Hash.ToUpperInvariant()
        Protocol = [int]$protocol.Protocol
        MaximumFrameBytes = [int]$protocol.MaximumFrameBytes
    }
}

function Assert-AmeBrokerAcceptanceBundlePairFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Version1,
        [Parameter(Mandatory = $true)]
        [psobject]$Version2
    )

    foreach ($bundle in @($Version1, $Version2)) {
        if ([string]$bundle.BrokerSha256 -notmatch '^[0-9A-F]{64}$' -or
            [string]$bundle.ClientSha256 -notmatch '^[0-9A-F]{64}$' -or
            [int]$bundle.Protocol -le 0 -or
            [int]$bundle.MaximumFrameBytes -le 0) {
            throw "A pre-signed acceptance bundle lacks bounded version identity"
        }
    }
    if ([string]$Version1.BrokerSha256 -ceq [string]$Version2.BrokerSha256) {
        throw "The V2 acceptance broker must have a different SHA-256 from V1"
    }
    if ([string]$Version1.ClientSha256 -cne [string]$Version2.ClientSha256) {
        throw "The current broker-only upgrade requires the V1 and V2 client SHA-256 to match"
    }
    if ([int]$Version1.Protocol -ne [int]$Version2.Protocol -or
        [int]$Version1.MaximumFrameBytes -ne [int]$Version2.MaximumFrameBytes) {
        throw "The V1 and V2 bundles must remain protocol-compatible with one installed client"
    }
}

function Get-AmeBrokerFixtureState {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $root = Get-Item -LiteralPath $Path -Force
    $rootPrefix = "$($root.FullName.TrimEnd([System.IO.Path]::DirectorySeparatorChar))$([System.IO.Path]::DirectorySeparatorChar)"
    $entries = @(
        Get-AmeBrokerNoFollowEntries -RootPath $Path |
            Sort-Object FullName |
            ForEach-Object {
                if (-not $_.FullName.StartsWith(
                    $rootPrefix,
                    [System.StringComparison]::OrdinalIgnoreCase
                )) {
                    throw "Fixture enumeration escaped the disposable root"
                }
                $relative = $_.FullName.Substring($rootPrefix.Length)
                if ($_.PSIsContainer) {
                    [pscustomobject]@{
                        RelativePath = $relative
                        Kind = "directory"
                        Length = $null
                        LastWriteTimeUtcTicks = $_.LastWriteTimeUtc.Ticks
                        Sha256 = $null
                    }
                } else {
                    [pscustomobject]@{
                        RelativePath = $relative
                        Kind = "file"
                        Length = $_.Length
                        LastWriteTimeUtcTicks = $_.LastWriteTimeUtc.Ticks
                        Sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
                    }
                }
            }
    )
    return [pscustomobject]@{
        RootLastWriteTimeUtcTicks = $root.LastWriteTimeUtc.Ticks
        EntryCount = $entries.Count
        Entries = $entries
    } | ConvertTo-Json -Depth 5 -Compress
}
