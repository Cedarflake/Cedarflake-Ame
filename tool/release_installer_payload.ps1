$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$script:AmeInstallerUpgradeCode = "C284E6B0-1484-4926-A94B-C3EC3840DBBD"
$script:AmeInstallerNamespace = "http://wixtoolset.org/schemas/v4/wxs"
$script:AmeInstallerDirectorySddl = "O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200A9;;;BU)"
$script:AmeInstallerFileSddl = "O:BAG:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)(A;;0x1200A9;;;BU)"

function Get-AmeInstallerVersion {
    param([Parameter(Mandatory = $true)][string]$Tag)

    $match = [regex]::Match($Tag, '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$')
    if (-not $match.Success) {
        throw "MSI requires a v-prefixed three-part release version without prerelease or build metadata"
    }
    $bounds = @(255, 255, 65535)
    for ($index = 0; $index -lt 3; $index++) {
        $number = 0L
        if (-not [long]::TryParse($match.Groups[$index + 1].Value, [ref]$number) -or
            $number -gt $bounds[$index]) {
            throw "Release version exceeds Windows Installer version limits"
        }
    }
    return $Tag.Substring(1)
}

function Get-AmeInstallerGuid {
    param([Parameter(Mandatory = $true)][string]$Name)

    $namespaceBytes = ([guid]$script:AmeInstallerUpgradeCode).ToByteArray()
    [Array]::Reverse($namespaceBytes, 0, 4)
    [Array]::Reverse($namespaceBytes, 4, 2)
    [Array]::Reverse($namespaceBytes, 6, 2)
    $nameBytes = [Text.Encoding]::UTF8.GetBytes($Name)
    $sha1 = [Security.Cryptography.SHA1]::Create()
    try { $digest = $sha1.ComputeHash($namespaceBytes + $nameBytes) } finally { $sha1.Dispose() }
    $digest[6] = ($digest[6] -band 0x0F) -bor 0x50
    $digest[8] = ($digest[8] -band 0x3F) -bor 0x80
    $hex = -join @($digest[0..15] | ForEach-Object { $_.ToString("X2") })
    return "{0}-{1}-{2}-{3}-{4}" -f $hex.Substring(0, 8), $hex.Substring(8, 4),
        $hex.Substring(12, 4), $hex.Substring(16, 4), $hex.Substring(20, 12)
}

function Assert-AmeInstallerPayloadNames {
    param([Parameter(Mandatory = $true)][object[]]$Files)

    if ($Files.Count -eq 0 -or $Files.Count -gt 4096) {
        throw "Installer payload must contain between 1 and 4096 files"
    }
    $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $total = 0L
    foreach ($file in $Files) {
        $name = [string]$file.relativePath
        $parts = $name.Split('/')
        if ([IO.Path]::IsPathRooted($name) -or $name.Contains('\') -or $parts.Count -gt 32 -or
            @($parts | Where-Object { $_ -in @("", ".", "..") -or $_ -match '[<>:"|?*\x00-\x1F]' -or $_ -match '[ .]$' }).Count -gt 0 -or
            -not $names.Add($name)) {
            throw "Installer payload has an unsafe or duplicate relative path"
        }
        if ([long]$file.length -le 0 -or [string]$file.sha256 -cnotmatch '^[0-9a-fA-F]{64}$') {
            throw "Installer payload has an invalid length or digest"
        }
        $total += [long]$file.length
        if ($total -gt 2GB) { throw "Installer payload exceeds 2 GiB" }
    }
    foreach ($required in @(
            "cedarflake_ame.exe", "cedarflake_ame_journal_broker.exe",
            "rust_lib_cedarflake_ame.dll", "flutter_windows.dll", "data/app.so", "data/icudtl.dat"
        )) {
        if (-not $names.Contains($required)) { throw "Installer payload is missing $required" }
    }
    if (@($Files | Where-Object { ([string]$_.relativePath).StartsWith("data/flutter_assets/") }).Count -eq 0) {
        throw "Installer payload is missing Flutter assets"
    }
}

function Add-AmeInstallerXmlElement {
    param(
        [Parameter(Mandatory = $true)][System.Xml.XmlElement]$Parent,
        [Parameter(Mandatory = $true)][string]$Name,
        [hashtable]$Attributes = @{}
    )

    $element = $Parent.OwnerDocument.CreateElement($Name, $script:AmeInstallerNamespace)
    foreach ($key in @($Attributes.Keys | Sort-Object)) {
        $element.SetAttribute($key, [string]$Attributes[$key])
    }
    [void]$Parent.AppendChild($element)
    return $element
}

function New-AmeInstallerApplicationSource {
    param(
        [Parameter(Mandatory = $true)][object[]]$Files,
        [Parameter(Mandatory = $true)][string]$BundlePath,
        [Parameter(Mandatory = $true)][string]$OutputPath
    )

    Assert-AmeInstallerPayloadNames -Files $Files
    $document = [Xml.XmlDocument]::new()
    $root = $document.CreateElement("Wix", $script:AmeInstallerNamespace)
    [void]$document.AppendChild($root)
    $fragment = Add-AmeInstallerXmlElement $root "Fragment"
    $directory = Add-AmeInstallerXmlElement $fragment "DirectoryRef" @{ Id = "AMEAPPDIR" }
    $group = Add-AmeInstallerXmlElement $fragment "ComponentGroup" @{ Id = "AmeApplicationFiles" }
    $directories = @{ "" = $directory }
    $directoryNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    [void]$directoryNames.Add("")
    $applicationFiles = @($Files | Where-Object { $_.relativePath -cne "cedarflake_ame_journal_broker.exe" } | Sort-Object relativePath)
    foreach ($file in $applicationFiles) {
        $parts = ([string]$file.relativePath).Split('/')
        for ($length = 1; $length -lt $parts.Count; $length++) {
            [void]$directoryNames.Add(($parts[0..($length - 1)] -join '/'))
        }
    }
    foreach ($relativeDirectory in @($directoryNames | Sort-Object)) {
        $id = "dir_" + (Get-AmeInstallerGuid "directory/$relativeDirectory").Replace("-", "")
        if ($relativeDirectory -ne "") {
            $separator = $relativeDirectory.LastIndexOf('/')
            $parent = ""
            if ($separator -ge 0) { $parent = $relativeDirectory.Substring(0, $separator) }
            $name = $relativeDirectory.Substring($separator + 1)
            $directories[$relativeDirectory] = Add-AmeInstallerXmlElement $directories[$parent] "Directory" @{ Id = $id; Name = $name }
        }
        $component = Add-AmeInstallerXmlElement $directories[$relativeDirectory] "Component" @{
            Id = "acl_$id"; Guid = Get-AmeInstallerGuid "directory-acl/$relativeDirectory"; Bitness = "always64"
        }
        $create = Add-AmeInstallerXmlElement $component "CreateFolder"
        [void](Add-AmeInstallerXmlElement $create "PermissionEx" @{ Sddl = $script:AmeInstallerDirectorySddl })
        [void](Add-AmeInstallerXmlElement $component "RemoveFolder" @{ Id = "remove_$id"; On = "uninstall" })
        [void](Add-AmeInstallerXmlElement $group "ComponentRef" @{ Id = "acl_$id" })
    }
    foreach ($file in $applicationFiles) {
        $relative = [string]$file.relativePath
        $separator = $relative.LastIndexOf('/')
        $parent = ""
        if ($separator -ge 0) { $parent = $relative.Substring(0, $separator) }
        $id = "file_" + (Get-AmeInstallerGuid "file/$relative").Replace("-", "")
        $component = Add-AmeInstallerXmlElement $directories[$parent] "Component" @{
            Id = "component_$id"; Guid = Get-AmeInstallerGuid "component/$relative"; Bitness = "always64"
        }
        $fileId = $id
        if ($relative -ceq "cedarflake_ame.exe") { $fileId = "AmeApplicationExecutable" }
        $entry = Add-AmeInstallerXmlElement $component "File" @{
            Id = $fileId; Source = Join-Path $BundlePath $relative; KeyPath = "yes"
        }
        [void](Add-AmeInstallerXmlElement $entry "PermissionEx" @{ Sddl = $script:AmeInstallerFileSddl })
        [void](Add-AmeInstallerXmlElement $group "ComponentRef" @{ Id = "component_$id" })
    }
    foreach ($shortcut in @(
            @{ Id = "AmeStartMenu"; Directory = "ProgramMenuFolder"; Setting = "StartMenu" },
            @{ Id = "AmeDesktop"; Directory = "DesktopFolder"; Setting = "Desktop" }
        )) {
        $shortcutGroup = Add-AmeInstallerXmlElement $fragment "ComponentGroup" @{ Id = $shortcut.Id + "Files" }
        $component = Add-AmeInstallerXmlElement $directory "Component" @{
            Id = $shortcut.Id; Guid = Get-AmeInstallerGuid ("shortcut/" + $shortcut.Id); Bitness = "always64"
        }
        [void](Add-AmeInstallerXmlElement $component "Shortcut" @{
            Id = $shortcut.Id; Directory = $shortcut.Directory; Name = "Cedarflake Ame"
            Target = "[#AmeApplicationExecutable]"; Icon = "AmeIcon"; WorkingDirectory = "AMEAPPDIR"
        })
        [void](Add-AmeInstallerXmlElement $component "RegistryValue" @{
            Root = "HKLM"; Key = "SOFTWARE\Cedarflake\Ame\Installer"; Name = $shortcut.Setting
            Type = "integer"; Value = "1"; KeyPath = "yes"
        })
        [void](Add-AmeInstallerXmlElement $shortcutGroup "ComponentRef" @{ Id = $shortcut.Id })
    }
    $settings = [Xml.XmlWriterSettings]::new()
    $settings.Encoding = [Text.UTF8Encoding]::new($false)
    $settings.Indent = $true
    $settings.IndentChars = "  "
    $settings.NewLineChars = "`n"
    $writer = [Xml.XmlWriter]::Create($OutputPath, $settings)
    try { $document.Save($writer) } finally { $writer.Dispose() }
}
