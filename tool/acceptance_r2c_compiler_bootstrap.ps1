function Start-AmeR2cRCompilerFaultForGuardrail {
    param([Parameter(Mandatory = $true)][string]$Point)
    if ($null -ne $script:AmeR2cRCompilerFaultPointForGuardrail -or
        $null -ne $script:AmeR2cRCompilerFaultOwnerForGuardrail) {
        throw "R2c-R compiler fault seam was not clean"
    }
    $script:AmeR2cRCompilerFaultPointForGuardrail = $Point
}

function Stop-AmeR2cRCompilerFaultForGuardrail {
    $script:AmeR2cRCompilerFaultPointForGuardrail = $null
}

function Invoke-AmeR2cRCompilerFaultForGuardrail {
    param(
        [Parameter(Mandatory = $true)][string]$Point,
        [Parameter(Mandatory = $true)][object]$Owner
    )
    if ($script:AmeR2cRCompilerFaultPointForGuardrail -cne $Point) { return }
    $script:AmeR2cRCompilerFaultOwnerForGuardrail = $Owner
    throw "R2c-R injected compiler ownership fault: $Point"
}

function Test-AmeR2cRCompilerFaultOwnerClosedForGuardrail {
    $owner = $script:AmeR2cRCompilerFaultOwnerForGuardrail
    if ($null -eq $owner) { return $false }
    if ($owner -is [Microsoft.Win32.SafeHandles.SafeFileHandle]) {
        return $owner.IsClosed
    }
    if ($null -ne $owner.RootHandle -and -not $owner.RootHandle.IsClosed) { return $false }
    foreach ($handle in @($owner.ToolAnchorHandles)) {
        if ($null -ne $handle -and -not $handle.IsClosed) { return $false }
    }
    return $true
}

function Reset-AmeR2cRCompilerFaultOwnerForGuardrail {
    $owner = $script:AmeR2cRCompilerFaultOwnerForGuardrail
    $script:AmeR2cRCompilerFaultOwnerForGuardrail = $null
    if ($owner -is [Microsoft.Win32.SafeHandles.SafeFileHandle] -and -not $owner.IsClosed) {
        $owner.Dispose()
    }
}

function Assert-AmeR2cRManagedNoReparseAncestors {
    param([Parameter(Mandatory = $true)][string]$Path)
    $current = [IO.DirectoryInfo]::new([IO.Path]::GetFullPath($Path))
    while ($null -ne $current) {
        if (-not $current.Exists) {
            throw "R2c-R managed trust anchor does not exist"
        }
        if (($current.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "R2c-R managed trust anchor contains a reparse-point ancestor"
        }
        $current = $current.Parent
    }
}

function Complete-AmeR2cREmittedType {
    param([Parameter(Mandatory = $true)][Reflection.Emit.TypeBuilder]$TypeBuilder)
    $createType = $TypeBuilder.GetType().GetMethod("CreateType", [Type[]]@())
    if ($null -ne $createType) { return $TypeBuilder.CreateType() }
    return $TypeBuilder.CreateTypeInfo().AsType()
}

function Add-AmeR2cREmittedStruct {
    param(
        [Parameter(Mandatory = $true)][Reflection.Emit.ModuleBuilder]$ModuleBuilder,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][object[]]$Fields
    )
    $attributes = [Reflection.TypeAttributes]::Public -bor
        [Reflection.TypeAttributes]::Sealed -bor
        [Reflection.TypeAttributes]::SequentialLayout -bor
        [Reflection.TypeAttributes]::BeforeFieldInit
    $builder = $ModuleBuilder.DefineType($Name, $attributes, [ValueType])
    foreach ($field in $Fields) {
        $builder.DefineField(
            [string]$field.Name,
            [type]$field.Type,
            [Reflection.FieldAttributes]::Public
        ) | Out-Null
    }
    return Complete-AmeR2cREmittedType -TypeBuilder $builder
}

function Add-AmeR2cREmittedPInvoke {
    param(
        [Parameter(Mandatory = $true)][Reflection.Emit.TypeBuilder]$TypeBuilder,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Library,
        [Parameter(Mandatory = $true)][type]$ReturnType,
        [Parameter(Mandatory = $true)][type[]]$ParameterTypes,
        [Parameter(Mandatory = $true)][Runtime.InteropServices.CharSet]$CharacterSet
    )
    $attributes = [Reflection.MethodAttributes]::Public -bor
        [Reflection.MethodAttributes]::Static -bor
        [Reflection.MethodAttributes]::PinvokeImpl
    $method = $TypeBuilder.DefinePInvokeMethod(
        $Name,
        $Library,
        $attributes,
        [Reflection.CallingConventions]::Standard,
        $ReturnType,
        $ParameterTypes,
        [Runtime.InteropServices.CallingConvention]::Winapi,
        $CharacterSet
    )
    $method.SetImplementationFlags(
        $method.GetMethodImplementationFlags() -bor
            [Reflection.MethodImplAttributes]::PreserveSig
    )
}

function Initialize-AmeR2cRBootstrapNativeTypes {
    if ($null -ne ("AmeR2cRBootstrapNative" -as [type])) { return }

    $assemblyName = [Reflection.AssemblyName]::new(
        "AmeR2cRBootstrapNative-$([Guid]::NewGuid().ToString('N'))"
    )
    try {
        $assembly = [Reflection.Emit.AssemblyBuilder]::DefineDynamicAssembly(
            $assemblyName,
            [Reflection.Emit.AssemblyBuilderAccess]::Run
        )
    } catch [Management.Automation.MethodException] {
        $assembly = [AppDomain]::CurrentDomain.DefineDynamicAssembly(
            $assemblyName,
            [Reflection.Emit.AssemblyBuilderAccess]::Run
        )
    }
    $module = $assembly.DefineDynamicModule($assemblyName.Name)
    $fileInformation = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapFileInformation" -Fields @(
            [pscustomobject]@{ Name = "FileAttributes"; Type = [uint32] },
            [pscustomobject]@{ Name = "CreationTimeLow"; Type = [uint32] },
            [pscustomobject]@{ Name = "CreationTimeHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "LastAccessTimeLow"; Type = [uint32] },
            [pscustomobject]@{ Name = "LastAccessTimeHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "LastWriteTimeLow"; Type = [uint32] },
            [pscustomobject]@{ Name = "LastWriteTimeHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "VolumeSerialNumber"; Type = [uint32] },
            [pscustomobject]@{ Name = "FileSizeHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "FileSizeLow"; Type = [uint32] },
            [pscustomobject]@{ Name = "NumberOfLinks"; Type = [uint32] },
            [pscustomobject]@{ Name = "FileIndexHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "FileIndexLow"; Type = [uint32] }
        )
    $unicodeString = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapUnicodeString" -Fields @(
            [pscustomobject]@{ Name = "Length"; Type = [uint16] },
            [pscustomobject]@{ Name = "MaximumLength"; Type = [uint16] },
            [pscustomobject]@{ Name = "Buffer"; Type = [IntPtr] }
        )
    $objectAttributes = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapObjectAttributes" -Fields @(
            [pscustomobject]@{ Name = "Length"; Type = [uint32] },
            [pscustomobject]@{ Name = "RootDirectory"; Type = [IntPtr] },
            [pscustomobject]@{ Name = "ObjectName"; Type = [IntPtr] },
            [pscustomobject]@{ Name = "Attributes"; Type = [uint32] },
            [pscustomobject]@{ Name = "SecurityDescriptor"; Type = [IntPtr] },
            [pscustomobject]@{ Name = "SecurityQualityOfService"; Type = [IntPtr] }
        )
    $ioStatusBlock = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapIoStatusBlock" -Fields @(
            [pscustomobject]@{ Name = "Status"; Type = [IntPtr] },
            [pscustomobject]@{ Name = "Information"; Type = [UIntPtr] }
        )
    $disposition = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapDisposition" -Fields @(
            [pscustomobject]@{ Name = "DeleteFile"; Type = [int32] }
        )
    $caseSensitiveInformation = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapCaseSensitiveInformation" -Fields @(
            [pscustomobject]@{ Name = "Flags"; Type = [uint32] }
        )

    $builder = $module.DefineType(
        "AmeR2cRBootstrapNative",
        [Reflection.TypeAttributes]::Public -bor
            [Reflection.TypeAttributes]::Sealed -bor
            [Reflection.TypeAttributes]::Abstract
    )
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "CreateFileW" `
        -Library "kernel32.dll" `
        -ReturnType ([Microsoft.Win32.SafeHandles.SafeFileHandle]) `
        -ParameterTypes @(
            [string], [uint32], [uint32], [IntPtr], [uint32], [uint32], [IntPtr]
        ) -CharacterSet Unicode
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "GetFileInformationByHandle" `
        -Library "kernel32.dll" -ReturnType ([bool]) `
        -ParameterTypes @(
            [Microsoft.Win32.SafeHandles.SafeFileHandle],
            $fileInformation.MakeByRefType()
        ) -CharacterSet None
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "GetFileInformationByHandleEx" `
        -Library "kernel32.dll" -ReturnType ([bool]) `
        -ParameterTypes @(
            [Microsoft.Win32.SafeHandles.SafeFileHandle],
            [uint32],
            $caseSensitiveInformation.MakeByRefType(),
            [uint32]
        ) -CharacterSet None
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "GetFinalPathNameByHandleW" `
        -Library "kernel32.dll" -ReturnType ([uint32]) `
        -ParameterTypes @(
            [Microsoft.Win32.SafeHandles.SafeFileHandle],
            [Text.StringBuilder],
            [uint32],
            [uint32]
        ) -CharacterSet Unicode
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "SetFileInformationByHandle" `
        -Library "kernel32.dll" -ReturnType ([bool]) `
        -ParameterTypes @(
            [Microsoft.Win32.SafeHandles.SafeFileHandle],
            [uint32],
            $disposition.MakeByRefType(),
            [uint32]
        ) -CharacterSet None
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "NtCreateFile" `
        -Library "ntdll.dll" -ReturnType ([int32]) `
        -ParameterTypes @(
            [IntPtr].MakeByRefType(),
            [uint32],
            $objectAttributes.MakeByRefType(),
            $ioStatusBlock.MakeByRefType(),
            [IntPtr],
            [uint32],
            [uint32],
            [uint32],
            [uint32],
            [IntPtr],
            [uint32]
        ) -CharacterSet None
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "RtlNtStatusToDosError" `
        -Library "ntdll.dll" -ReturnType ([uint32]) `
        -ParameterTypes @([int32]) -CharacterSet None
    Complete-AmeR2cREmittedType -TypeBuilder $builder | Out-Null
}

function Get-AmeR2cRBootstrapHandleEvidence {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.SafeHandles.SafeFileHandle]$Handle,
        [Parameter(Mandatory = $true)][string]$Label,
        [switch]$AllowReparse
    )
    if ($Handle.IsInvalid -or $Handle.IsClosed) {
        throw "R2c-R $Label handle is not held"
    }
    $informationType = "AmeR2cRBootstrapFileInformation" -as [type]
    $information = [Activator]::CreateInstance($informationType)
    if (-not [AmeR2cRBootstrapNative]::GetFileInformationByHandle(
        $Handle,
        [ref]$information
    )) {
        throw "R2c-R could not inspect the $Label handle"
    }
    if (($information.FileAttributes -band [uint32]0x10) -eq 0) {
        throw "R2c-R $Label is not a directory"
    }
    $isReparse = ($information.FileAttributes -band [uint32]0x400) -ne 0
    if ($isReparse -and -not $AllowReparse) {
        throw "R2c-R $Label is a reparse point"
    }
    $buffer = [Text.StringBuilder]::new(32768)
    $length = [AmeR2cRBootstrapNative]::GetFinalPathNameByHandleW(
        $Handle,
        $buffer,
        [uint32]$buffer.Capacity,
        0
    )
    if ($length -eq 0 -or $length -ge $buffer.Capacity) {
        throw "R2c-R could not resolve the $Label handle"
    }
    $resolved = $buffer.ToString()
    if ($resolved.StartsWith('\\?\UNC\', [StringComparison]::OrdinalIgnoreCase)) {
        $resolved = '\\' + $resolved.Substring(8)
    } elseif ($resolved.StartsWith('\\?\', [StringComparison]::OrdinalIgnoreCase)) {
        $resolved = $resolved.Substring(4)
    }
    return [pscustomobject]@{
        Attributes = [uint32]$information.FileAttributes
        Identity = "{0:X8}:{1:X8}:{2:X8}" -f (
            [uint32]$information.VolumeSerialNumber,
            [uint32]$information.FileIndexHigh,
            [uint32]$information.FileIndexLow
        )
        IsReparse = $isReparse
        Path = $resolved.TrimEnd('\')
        VolumeSerialNumber = [uint32]$information.VolumeSerialNumber
    }
}

function Test-AmeR2cRBootstrapDirectoryCaseSensitive {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.SafeHandles.SafeFileHandle]$Handle
    )
    $informationType = "AmeR2cRBootstrapCaseSensitiveInformation" -as [type]
    $information = [Activator]::CreateInstance($informationType)
    if (-not [AmeR2cRBootstrapNative]::GetFileInformationByHandleEx(
        $Handle,
        [uint32]23,
        [ref]$information,
        [uint32][Runtime.InteropServices.Marshal]::SizeOf($information)
    )) {
        throw "R2c-R could not query compiler-anchor case semantics"
    }
    return ([uint32]$information.Flags -band [uint32]1) -ne 0
}

function Open-AmeR2cRBootstrapRelativeDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.SafeHandles.SafeFileHandle]$ParentHandle,
        [Parameter(Mandatory = $true)][string]$LeafName,
        [Parameter(Mandatory = $true)][bool]$Create,
        [Parameter(Mandatory = $true)][bool]$RequestDelete,
        [switch]$RequestWriteAttributes,
        [switch]$ExistingComponent
    )
    if ($ExistingComponent) {
        if ([string]::IsNullOrEmpty($LeafName) -or $LeafName -in @(".", "..") -or
            $LeafName.IndexOfAny([char[]]@('\', '/', ':', [char]0)) -ge 0 -or
            $LeafName.EndsWith('.', [StringComparison]::Ordinal) -or
            $LeafName.EndsWith(' ', [StringComparison]::Ordinal)) {
            throw "R2c-R compiler anchor component is not one normalized NT name"
        }
    } elseif ($LeafName -notmatch '^(?:\.ame-r2c-r-bootstrap-[0-9a-f]{32}|\.ame-r2c-r-audit-[0-9a-f]{32}\.ps1)$') {
        throw "R2c-R compiler or audited-script guardrail leaf is outside the owned-name contract"
    }
    $unicodeType = "AmeR2cRBootstrapUnicodeString" -as [type]
    $attributesType = "AmeR2cRBootstrapObjectAttributes" -as [type]
    $statusType = "AmeR2cRBootstrapIoStatusBlock" -as [type]
    $nameBuffer = [Runtime.InteropServices.Marshal]::StringToHGlobalUni($LeafName)
    $nameStructure = [IntPtr]::Zero
    $rawHandle = [IntPtr]::Zero
    try {
        $unicodeName = [Activator]::CreateInstance($unicodeType)
        $unicodeName.Length = [uint16]($LeafName.Length * 2)
        $unicodeName.MaximumLength = [uint16](($LeafName.Length + 1) * 2)
        $unicodeName.Buffer = $nameBuffer
        $nameStructure = [Runtime.InteropServices.Marshal]::AllocHGlobal(
            [Runtime.InteropServices.Marshal]::SizeOf($unicodeName)
        )
        [Runtime.InteropServices.Marshal]::StructureToPtr(
            $unicodeName,
            $nameStructure,
            $false
        )
        $attributes = [Activator]::CreateInstance($attributesType)
        $attributes.Length = [uint32][Runtime.InteropServices.Marshal]::SizeOf($attributes)
        $attributes.RootDirectory = $ParentHandle.DangerousGetHandle()
        $attributes.ObjectName = $nameStructure
        $attributes.Attributes = [uint32]0
        if (-not (Test-AmeR2cRBootstrapDirectoryCaseSensitive -Handle $ParentHandle)) {
            $attributes.Attributes = [uint32]($attributes.Attributes -bor 0x40)
        }
        $statusBlock = [Activator]::CreateInstance($statusType)
        $desiredAccess = [uint32](0x80 -bor 0x100000)
        if (-not $ExistingComponent) {
            $desiredAccess = [uint32]($desiredAccess -bor 0x1)
        }
        if ($RequestDelete) { $desiredAccess = [uint32]($desiredAccess -bor 0x10000) }
        if ($RequestWriteAttributes) { $desiredAccess = [uint32]($desiredAccess -bor 0x100) }
        $status = [AmeR2cRBootstrapNative]::NtCreateFile(
            [ref]$rawHandle,
            $desiredAccess,
            [ref]$attributes,
            [ref]$statusBlock,
            [IntPtr]::Zero,
            [uint32]0x10,
            [uint32](0x1 -bor 0x2),
            [uint32]$(if ($Create) { 2 } else { 1 }),
            [uint32](0x1 -bor 0x20 -bor 0x4000 -bor 0x200000),
            [IntPtr]::Zero,
            0
        )
        if ($status -lt 0) {
            $win32 = [AmeR2cRBootstrapNative]::RtlNtStatusToDosError($status)
            $operation = if ($Create) { "create" } else { "reopen" }
            throw (
                "R2c-R could not $operation relative directory '$LeafName' from its held " +
                "parent; NTSTATUS=0x$($status.ToString('X8')) Win32=$win32"
            )
        }
        $handle = [Microsoft.Win32.SafeHandles.SafeFileHandle]::new($rawHandle, $true)
        $rawHandle = [IntPtr]::Zero
        return $handle
    } finally {
        if ($rawHandle -ne [IntPtr]::Zero -and $rawHandle -ne [IntPtr](-1)) {
            [Microsoft.Win32.SafeHandles.SafeFileHandle]::new($rawHandle, $true).Dispose()
        }
        if ($nameStructure -ne [IntPtr]::Zero) {
            [Runtime.InteropServices.Marshal]::FreeHGlobal($nameStructure)
        }
        [Runtime.InteropServices.Marshal]::FreeHGlobal($nameBuffer)
    }
}

function Set-AmeR2cRBootstrapCaseSensitivityForGuardrail {
    param(
        [Parameter(Mandatory = $true)][psobject]$Bootstrap,
        [Parameter(Mandatory = $true)][bool]$Enabled
    )
    Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $Bootstrap
    $caseHandle = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $Bootstrap.ToolHandle `
        -LeafName ([string]$Bootstrap.LeafName) `
        -Create $false `
        -RequestDelete $false `
        -RequestWriteAttributes
    try {
        $caseEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $caseHandle `
            -Label "case-sensitive source guardrail"
        if ($caseEvidence.Identity -cne $Bootstrap.RootIdentity) {
            throw "R2c-R case-sensitive source guardrail encountered a replacement identity"
        }
        $informationType = "AmeR2cRBootstrapDisposition" -as [type]
        $information = [Activator]::CreateInstance($informationType)
        $information.DeleteFile = if ($Enabled) { 1 } else { 0 }
        if (-not [AmeR2cRBootstrapNative]::SetFileInformationByHandle(
            $caseHandle,
            23,
            [ref]$information,
            [uint32][Runtime.InteropServices.Marshal]::SizeOf($information)
        )) {
            $win32 = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            throw "R2c-R could not set case-sensitive source guardrail semantics; Win32=$win32"
        }
        if ((Test-AmeR2cRBootstrapDirectoryCaseSensitive -Handle $caseHandle) -ne $Enabled) {
            throw "R2c-R case-sensitive source guardrail semantics did not take effect"
        }
    } finally { $caseHandle.Dispose() }
}

function Open-AmeR2cRBootstrapDirectoryChain {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )
    $fullPath = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    $volumeRoot = [IO.Path]::GetPathRoot($fullPath)
    if ([string]::IsNullOrEmpty($volumeRoot) -or $volumeRoot -notmatch '^[A-Za-z]:\\$') {
        throw "R2c-R $Label must be on a local drive-letter volume"
    }
    $handles = [Collections.Generic.List[Microsoft.Win32.SafeHandles.SafeFileHandle]]::new()
    try {
        $volumeHandle = $null
        try {
            $volumeHandle = [AmeR2cRBootstrapNative]::CreateFileW(
                $volumeRoot,
                [uint32](0x80 -bor 0x100000),
                [uint32](0x1 -bor 0x2),
                [IntPtr]::Zero,
                3,
                [uint32](0x200000 -bor 0x2000000),
                [IntPtr]::Zero
            )
            if ($volumeHandle.IsInvalid) {
                throw "R2c-R could not hold the $Label volume root"
            }
            Invoke-AmeR2cRCompilerFaultForGuardrail `
                -Point "volume-post-open-pre-transfer" `
                -Owner $volumeHandle
            $volumeEvidence = Get-AmeR2cRBootstrapHandleEvidence `
                -Handle $volumeHandle -Label "$Label volume root"
            $handles.Add($volumeHandle)
            $parent = $volumeHandle
            $volumeHandle = $null
        } finally {
            if ($null -ne $volumeHandle) { $volumeHandle.Dispose() }
        }
        $relative = $fullPath.Substring($volumeRoot.Length)
        foreach ($component in @(
            $relative.Split([char[]]@('\'), [StringSplitOptions]::RemoveEmptyEntries)
        )) {
            $child = Open-AmeR2cRBootstrapRelativeDirectory `
                -ParentHandle $parent `
                -LeafName $component `
                -Create $false `
                -RequestDelete $false `
                -ExistingComponent
            try {
                $childEvidence = Get-AmeR2cRBootstrapHandleEvidence `
                    -Handle $child -Label "$Label component"
                if ($childEvidence.VolumeSerialNumber -ne $volumeEvidence.VolumeSerialNumber) {
                    throw "R2c-R $Label crossed its trusted volume boundary"
                }
                $handles.Add($child)
                $parent = $child
                $child = $null
            } finally {
                if ($null -ne $child) { $child.Dispose() }
            }
        }
        $terminalEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $parent -Label "$Label terminal"
        return [pscustomobject]@{
            Handles = $handles
            Identity = [string]$terminalEvidence.Identity
            Path = [string]$terminalEvidence.Path
            Terminal = $parent
            VolumeSerialNumber = [uint32]$terminalEvidence.VolumeSerialNumber
        }
    } catch {
        for ($index = $handles.Count - 1; $index -ge 0; $index--) {
            $handles[$index].Dispose()
        }
        throw
    }
}

function Remove-AmeR2cRBootstrapExpectedDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.SafeHandles.SafeFileHandle]$ParentHandle,
        [Parameter(Mandatory = $true)][string]$LeafName,
        [Parameter(Mandatory = $true)][string]$ExpectedIdentity,
        [Parameter(Mandatory = $true)][bool]$ExpectReparse
    )
    $deleteHandle = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $ParentHandle `
        -LeafName $LeafName `
        -Create $false `
        -RequestDelete $true
    try {
        $deleteEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $deleteHandle `
            -Label "compiler guardrail cleanup" `
            -AllowReparse
        if ($deleteEvidence.Identity -cne $ExpectedIdentity -or
            [bool]$deleteEvidence.IsReparse -ne $ExpectReparse) {
            throw "R2c-R compiler guardrail cleanup encountered an unknown identity"
        }
        $dispositionType = "AmeR2cRBootstrapDisposition" -as [type]
        $disposition = [Activator]::CreateInstance($dispositionType)
        $disposition.DeleteFile = 1
        if (-not [AmeR2cRBootstrapNative]::SetFileInformationByHandle(
            $deleteHandle,
            4,
            [ref]$disposition,
            [uint32][Runtime.InteropServices.Marshal]::SizeOf($disposition)
        )) {
            throw "R2c-R could not delete the exact compiler guardrail directory"
        }
    } finally { $deleteHandle.Dispose() }
}

function Assert-AmeR2cRCompilerBootstrapHeld {
    param([Parameter(Mandatory = $true)][psobject]$Bootstrap)
    if ($null -eq $Bootstrap.ToolAnchorHandles -or
        $Bootstrap.ToolAnchorHandles.Count -ne $Bootstrap.ToolAnchorIdentities.Count) {
        throw "R2c-R compiler bootstrap lost its held anchor chain"
    }
    for ($index = 0; $index -lt $Bootstrap.ToolAnchorHandles.Count; $index++) {
        $chainEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $Bootstrap.ToolAnchorHandles[$index] `
            -Label "compiler tool anchor chain"
        if ($chainEvidence.Identity -cne $Bootstrap.ToolAnchorIdentities[$index]) {
            throw "R2c-R compiler tool anchor chain identity changed while held"
        }
    }
    $toolEvidence = Get-AmeR2cRBootstrapHandleEvidence `
        -Handle $Bootstrap.ToolHandle -Label "compiler tool anchor"
    $rootEvidence = Get-AmeR2cRBootstrapHandleEvidence `
        -Handle $Bootstrap.RootHandle -Label "compiler bootstrap"
    if ($toolEvidence.Identity -cne $Bootstrap.ToolIdentity -or
        $rootEvidence.Identity -cne $Bootstrap.RootIdentity) {
        throw "R2c-R compiler bootstrap identity changed while held"
    }
    if ($rootEvidence.VolumeSerialNumber -ne $toolEvidence.VolumeSerialNumber) {
        throw "R2c-R compiler bootstrap escaped its held tool anchor"
    }
    $relativeRoot = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $Bootstrap.ToolHandle `
        -LeafName ([string]$Bootstrap.LeafName) `
        -Create $false `
        -RequestDelete $false
    try {
        $relativeEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $relativeRoot -Label "compiler bootstrap relative identity"
        if ($relativeEvidence.Identity -cne $Bootstrap.RootIdentity) {
            throw "R2c-R compiler bootstrap is no longer the held tool child"
        }
    } finally { $relativeRoot.Dispose() }
}

function New-AmeR2cRCompilerBootstrap {
    param(
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [string]$LeafName = ".ame-r2c-r-bootstrap-$([Guid]::NewGuid().ToString('N'))"
    )
    Initialize-AmeR2cRBootstrapNativeTypes
    $toolPath = [IO.Path]::GetFullPath((Join-Path $RepositoryRoot "tool")).TrimEnd('\')
    $logicalChain = $null
    $physicalChain = $null
    $rootHandle = $null
    try {
        $logicalChain = Open-AmeR2cRBootstrapDirectoryChain `
            -Path $toolPath -Label "compiler logical tool anchor"
        $physicalChain = Open-AmeR2cRBootstrapDirectoryChain `
            -Path ([string]$logicalChain.Path) -Label "compiler physical tool anchor"
        if ($logicalChain.Identity -cne $physicalChain.Identity -or
            $logicalChain.VolumeSerialNumber -ne $physicalChain.VolumeSerialNumber) {
            throw "R2c-R compiler tool anchor changed during read-only physical binding"
        }
        $toolHandle = $physicalChain.Terminal
        $toolEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $toolHandle -Label "compiler tool anchor"
        $rootHandle = Open-AmeR2cRBootstrapRelativeDirectory `
            -ParentHandle $toolHandle -LeafName $LeafName -Create $true -RequestDelete $false
        $rootEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $rootHandle -Label "compiler bootstrap"
        if ($rootEvidence.VolumeSerialNumber -ne $toolEvidence.VolumeSerialNumber) {
            throw "R2c-R compiler bootstrap escaped its held physical tool anchor"
        }
        $anchorHandles = [Collections.Generic.List[Microsoft.Win32.SafeHandles.SafeFileHandle]]::new()
        $anchorIdentities = [Collections.Generic.List[string]]::new()
        foreach ($handle in @($logicalChain.Handles) + @($physicalChain.Handles)) {
            $anchorHandles.Add($handle)
            $anchorIdentities.Add([string](
                Get-AmeR2cRBootstrapHandleEvidence `
                    -Handle $handle -Label "compiler retained anchor"
            ).Identity)
        }
        return [pscustomobject]@{
            LeafName = $LeafName
            Path = $rootEvidence.Path
            RootHandle = $rootHandle
            RootIdentity = $rootEvidence.Identity
            ToolHandle = $toolHandle
            ToolAnchorHandles = $anchorHandles
            ToolAnchorIdentities = $anchorIdentities
            ToolIdentity = $toolEvidence.Identity
            ToolPath = $toolEvidence.Path
        }
    } catch {
        if ($null -ne $rootHandle) { $rootHandle.Dispose() }
        foreach ($chain in @($physicalChain, $logicalChain)) {
            if ($null -ne $chain) {
                for ($index = $chain.Handles.Count - 1; $index -ge 0; $index--) {
                    $chain.Handles[$index].Dispose()
                }
            }
        }
        throw
    }
}

function Remove-AmeR2cRCompilerBootstrap {
    param([Parameter(Mandatory = $true)][psobject]$Bootstrap)
    $deleteHandle = $null
    try {
        Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $Bootstrap
        $entries = @(
            [IO.DirectoryInfo]::new([string]$Bootstrap.Path).EnumerateFileSystemInfos()
        )
        if ($entries.Count -ne 0) {
            throw (
                "R2c-R compiler bootstrap contains unknown residue and was retained " +
                "without traversal: $($Bootstrap.Path)"
            )
        }
        $Bootstrap.RootHandle.Dispose()
        $Bootstrap.RootHandle = $null
        $deleteHandle = Open-AmeR2cRBootstrapRelativeDirectory `
            -ParentHandle $Bootstrap.ToolHandle `
            -LeafName ([string]$Bootstrap.LeafName) `
            -Create $false `
            -RequestDelete $true
        $deleteEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $deleteHandle -Label "compiler bootstrap cleanup"
        if ($deleteEvidence.Identity -cne $Bootstrap.RootIdentity) {
            throw "R2c-R compiler bootstrap cleanup encountered a replacement identity"
        }
        $dispositionType = "AmeR2cRBootstrapDisposition" -as [type]
        $disposition = [Activator]::CreateInstance($dispositionType)
        $disposition.DeleteFile = 1
        if (-not [AmeR2cRBootstrapNative]::SetFileInformationByHandle(
            $deleteHandle,
            4,
            [ref]$disposition,
            [uint32][Runtime.InteropServices.Marshal]::SizeOf($disposition)
        )) {
            throw "R2c-R could not delete the exact compiler bootstrap by held identity"
        }
    } finally {
        if ($null -ne $deleteHandle) { $deleteHandle.Dispose() }
        if ($null -ne $Bootstrap.RootHandle) { $Bootstrap.RootHandle.Dispose() }
        if ($null -ne $Bootstrap.ToolAnchorHandles) {
            for ($index = $Bootstrap.ToolAnchorHandles.Count - 1; $index -ge 0; $index--) {
                $Bootstrap.ToolAnchorHandles[$index].Dispose()
            }
        }
        $Bootstrap.ToolHandle = $null
    }
}
