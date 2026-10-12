$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Assert-AmeInstallerLayout {
    param([Parameter(Mandatory = $true)]$Model, [Parameter(Mandatory = $true)]$Receipt)

    function Require([bool]$Condition, [string]$Reason) {
        if (-not $Condition) { throw "Installer policy: $Reason" }
    }
    $directories = @{}
    foreach ($row in $Model.Directory) { $directories[$row.Directory] = $row }
    Require ($directories.AmeProductFolder.Directory_Parent -ceq 'ProgramFiles64Folder' -and
        $directories.AmeProductFolder.DefaultDir.Split('|')[-1] -ceq 'Cedarflake Ame' -and
        $directories.AMEAPPDIR.Directory_Parent -ceq 'AmeProductFolder' -and
        $directories.AMEAPPDIR.DefaultDir.Split('|')[-1] -ceq 'Application' -and
        $directories.AmeBrokerFolder.Directory_Parent -ceq 'AmeProductFolder' -and
        $directories.AmeBrokerFolder.DefaultDir.Split('|')[-1] -ceq 'Journal Broker') 'fixed destination changed'
    $components = @{}
    foreach ($row in $Model.Component) {
        $attributes = '256'
        if ($row.Component -cin @('AmeInstallationSettings', 'AmeStartMenu', 'AmeDesktop')) { $attributes = '260' }
        Require ($row.Attributes -ceq $attributes -and $row.Condition -ceq '') 'conditional or non-x64 component'
        $directory = $row.Directory_
        $depth = 0
        while ($directory -cne 'AmeProductFolder') {
            Require (++$depth -le 32 -and $directories.ContainsKey($directory)) 'component is outside the fixed product tree'
            $directory = $directories[$directory].Directory_Parent
        }
        $components[$row.Component] = $row
    }
    Require ($components.AmeBroker.Directory_ -ceq 'AmeBrokerFolder' -and
        $components.AmeBroker.KeyPath -ceq 'AmeBrokerExecutable') 'broker component destination changed'
    $features = @{}
    foreach ($feature in $Model.Feature) { $features[$feature.Feature] = $feature }
    Require ($features.Count -eq 3 -and $features.Ame.Level -ceq '1' -and
        $features.Ame.Attributes -ceq '24' -and $features.Ame.Directory_ -ceq 'AMEAPPDIR' -and
        $features.AmeStartMenuFeature.Level -ceq '1' -and $features.AmeStartMenuFeature.Attributes -ceq '8' -and
        $features.AmeDesktopFeature.Level -ceq '2' -and $features.AmeDesktopFeature.Attributes -ceq '8' -and
        $features.AmeStartMenuFeature.Feature_Parent -ceq 'Ame' -and $features.AmeDesktopFeature.Feature_Parent -ceq 'Ame' -and
        $Model.FeatureComponents.Count -eq $components.Count) 'mandatory product or optional shortcut choices changed'
    $assigned = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($row in $Model.FeatureComponents) {
        $owner = 'Ame'
        if ($row.Component_ -ceq 'AmeStartMenu') { $owner = 'AmeStartMenuFeature' }
        if ($row.Component_ -ceq 'AmeDesktop') { $owner = 'AmeDesktopFeature' }
        Require ($row.Feature_ -ceq $owner -and $components.ContainsKey($row.Component_) -and
            $assigned.Add($row.Component_)) 'unowned or multiply assigned feature component'
    }
    $payload = @{}
    foreach ($file in $Receipt.payload.files) { $payload[$file.relativePath] = $file }
    Require ($Model.File.Count -eq $payload.Count) 'package file count differs from verified payload'
    $installed = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $Model.File) {
        $component = $components[$file.Component_]
        Require ($null -ne $component -and $component.KeyPath -ceq $file.File) 'file is not its component key'
        $name = $file.FileName.Split('|')[-1]
        $directory = $component.Directory_
        $depth = 0
        while ($directory -notin @('AMEAPPDIR', 'AmeBrokerFolder')) {
            Require (++$depth -le 32 -and $directories.ContainsKey($directory)) 'payload destination is outside owned directories'
            $parent = $directories[$directory]
            $name = $parent.DefaultDir.Split('|')[-1] + '/' + $name
            $directory = $parent.Directory_Parent
        }
        Require ($directory -cne 'AmeBrokerFolder' -or $name -ceq 'cedarflake_ame_journal_broker.exe') 'unexpected broker payload'
        Require ($payload.ContainsKey($name) -and $installed.Add($name)) 'unknown or duplicate installed file'
        Require ([long]$file.FileSize -eq [long]$payload[$name].length) 'installed file length differs from verified payload'
    }
    foreach ($row in $Model.RemoveFile) {
        Require ($row.FileName -ceq '' -and $row.InstallMode -ceq '2' -and
            $components.ContainsKey($row.Component_) -and $directories.ContainsKey($row.DirProperty)) 'uninstall may remove only owned empty folders'
    }
    Require ($Model.Shortcut.Count -eq 2) 'shortcut choices are incomplete'
    foreach ($shortcut in $Model.Shortcut) {
        $destination = @{ AmeStartMenu = 'ProgramMenuFolder'; AmeDesktop = 'DesktopFolder' }
        Require ($destination.ContainsKey($shortcut.Shortcut) -and
            $shortcut.Directory_ -ceq $destination[$shortcut.Shortcut] -and
            $shortcut.Component_ -ceq $shortcut.Shortcut -and $shortcut.Target -ceq '[#AmeApplicationExecutable]' -and
            $shortcut.Arguments -ceq '' -and $shortcut.WkDir -ceq 'AMEAPPDIR') 'application shortcut changed'
    }
    Require ($Model.Registry.Count -eq 3) 'unexpected installation registry ownership'
    foreach ($entry in $Model.Registry) {
        $settings = @{
            AmeInstallationSettings = @('InstallFolder', '[AMEAPPDIR]')
            AmeStartMenu = @('StartMenu', '#1')
            AmeDesktop = @('Desktop', '#1')
        }
        Require ($settings.ContainsKey($entry.Component_)) 'unowned installation setting'
        $expected = $settings[$entry.Component_]
        Require ($entry.Root -ceq '2' -and $entry.Key -ceq 'SOFTWARE\Cedarflake\Ame\Installer' -and
            $entry.Name -ceq $expected[0] -and $entry.Value -ceq $expected[1] -and
            $components[$entry.Component_].KeyPath -ceq $entry.Registry) 'installation setting differs from exact ownership'
    }
    Require ($Model.Media.Count -eq 1 -and $Model.Media[0].Cabinet.StartsWith('#')) 'payload must be embedded'

    $directoryAcl = 'O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200A9;;;BU)'
    $fileAcl = 'O:BAG:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)(A;;0x1200A9;;;BU)'
    $brokerSid = 'S-1-5-80-2098581772-366539847-2170527201-1913264099-3006267874'
    $permissions = @{}
    foreach ($row in $Model.MsiLockPermissionsEx) {
        $key = $row.Table + '/' + $row.LockObject
        Require (-not $permissions.ContainsKey($key) -and $row.Condition -ceq '') 'ambiguous or conditional permissions'
        $expected = ''
        switch -CaseSensitive ($row.Table) {
            'File' {
                $expected = $fileAcl
                if ($row.LockObject -ceq 'AmeBrokerExecutable') { $expected += "(A;;0x1200A9;;;$brokerSid)" }
            }
            'CreateFolder' {
                $expected = $directoryAcl
                if ($row.LockObject -ceq 'AmeBrokerFolder') { $expected += "(A;OICI;0x1200A9;;;$brokerSid)" }
            }
            'ServiceInstall' { $expected = 'O:BAG:BAD:(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCRPLO;;;IU)' }
            default { throw "Installer policy: unexpected permission target" }
        }
        Require ($row.SDDLText -ceq $expected) 'permissions differ from exact allowlist'
        $permissions[$key] = $true
    }
    foreach ($file in $Model.File) { Require ($permissions.ContainsKey('File/' + $file.File)) 'missing file permissions' }
    foreach ($component in $Model.Component) {
        Require ($permissions.ContainsKey('CreateFolder/' + $component.Directory_)) 'missing directory permissions'
    }
    Require ($permissions.ContainsKey('ServiceInstall/AmeJournalBroker')) 'missing service permissions'
    foreach ($dialog in @('WelcomeDlg', 'CustomizeDlg', 'BrowseDlg', 'MaintenanceWelcomeDlg', 'MaintenanceTypeDlg', 'VerifyReadyDlg', 'ProgressDlg', 'ExitDialog')) {
        Require (@($Model.Dialog | Where-Object { $_.Dialog -ceq $dialog }).Count -eq 1) "missing normal workflow dialog $dialog"
    }
    Require (@($Model.Dialog | Where-Object { $_.Dialog -match 'License|Eula' }).Count -eq 0) 'unexpected license dialog'
    foreach ($navigation in @(@('WelcomeDlg', 'CustomizeDlg'), @('CustomizeDlg', 'VerifyReadyDlg'))) {
        Require (@($Model.ControlEvent | Where-Object {
            $_.Dialog_ -ceq $navigation[0] -and $_.Control_ -ceq 'Next' -and $_.Event -ceq 'NewDialog' -and
            $_.Argument -ceq $navigation[1] -and $_.Condition -ceq '1'
        }).Count -eq 1) 'installation choices are disconnected'
    }
    foreach ($mode in @('Repair', 'Remove')) {
        Require (@($Model.ControlEvent | Where-Object {
            $_.Dialog_ -ceq 'MaintenanceTypeDlg' -and $_.Control_ -ceq ($mode + 'Button') -and
            $_.Event -ceq 'NewDialog' -and $_.Argument -ceq 'VerifyReadyDlg' -and $_.Condition -ceq '1'
        }).Count -eq 1) "maintenance action $mode is disconnected"
    }
    Require (@($Model.ControlEvent | Where-Object {
        $_.Dialog_ -ceq 'VerifyReadyDlg' -and $_.Control_ -ceq 'Repair' -and
        $_.Event -ceq 'Reinstall' -and $_.Argument -ceq 'All'
    }).Count -eq 1) 'repair does not select the complete product'
}
