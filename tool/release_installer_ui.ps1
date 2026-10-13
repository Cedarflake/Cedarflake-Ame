$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Assert-AmeInstallerUi {
    param([Parameter(Mandatory = $true)]$Model)

    function Require([bool]$Condition, [string]$Reason) {
        if (-not $Condition) { throw "Installer policy: $Reason" }
    }

    $optionsDialog = 'AmeInstallOptionsDlg'
    foreach ($dialog in @($optionsDialog, 'BrowseDlg', 'MaintenanceWelcomeDlg',
            'MaintenanceTypeDlg', 'VerifyReadyDlg', 'ProgressDlg', 'ExitDialog')) {
        Require (@($Model.Dialog | Where-Object { $_.Dialog -ceq $dialog }).Count -eq 1) `
            "missing normal workflow dialog $dialog"
    }
    Require (@($Model.Dialog | Where-Object {
        $_.Dialog -match 'License|Eula' -or $_.Dialog -cin @('WelcomeDlg', 'CustomizeDlg', 'PrepareDlg')
    }).Count -eq 0) 'unexpected introductory, feature-tree or license dialog'
    Require (@($Model.Control | Where-Object Type -CEQ 'SelectionTree').Count -eq 0) `
        'installation choices must not use a feature tree'

    $dialog = @($Model.Dialog | Where-Object Dialog -CEQ $optionsDialog)[0]
    Require ($dialog.Control_First -ceq 'InstallPath' -and $dialog.Control_Default -ceq 'Next' -and
        $dialog.Control_Cancel -ceq 'Cancel') 'options focus or default action changed'
    $controls = @{}
    foreach ($control in $Model.Control | Where-Object Dialog_ -CEQ $optionsDialog) {
        $controls[$control.Control] = $control
    }
    Require ($controls.InstallPath.Type -ceq 'PathEdit' -and
        $controls.InstallPath.Property -ceq 'AMEAPPDIR') 'installation path is not directly editable'
    $focusOrder = @('InstallPath', 'Browse', 'StartMenu', 'Desktop', 'Next', 'Cancel')
    for ($index = 0; $index -lt $focusOrder.Count; $index++) {
        $control = $controls[$focusOrder[$index]]
        Require ($null -ne $control -and ([int]$control.Attributes -band 3) -eq 3 -and
            ([int]$control.Attributes -band 8) -eq 0 -and
            $control.Control_Next -ceq $focusOrder[($index + 1) % $focusOrder.Count]) `
            'options must use enabled native controls in keyboard order'
    }
    foreach ($name in @('StartMenu', 'Desktop')) {
        $property = "Ame${name}Choice"
        Require ($controls[$name].Type -ceq 'CheckBox' -and $controls[$name].Property -ceq $property -and
            @($Model.CheckBox | Where-Object { $_.Property -ceq $property -and $_.Value -ceq '1' }).Count -eq 1) `
            "shortcut $name is not an independent checkbox"
        Require (@($Model.Property | Where-Object Property -CEQ $property).Count -eq 0) `
            'shortcut choices must initialize from the costed feature states'
        Require (@($Model.InstallExecuteSequence | Where-Object Action -CEQ "Set$property").Count -eq 0) `
            'UI choices must not override silent installation or repair'
    }

    $sequence = @{}
    foreach ($row in $Model.InstallUISequence) { $sequence[$row.Action] = $row }
    $previous = 0
    foreach ($action in @('CostFinalize', 'MigrateFeatureStates', 'SetAmeStartMenuChoice',
            'SetAmeDesktopChoice', $optionsDialog, 'ProgressDlg', 'ExecuteAction')) {
        Require ($sequence.ContainsKey($action) -and [int]$sequence[$action].Sequence -gt $previous) `
            "unsafe options initialization order at $action"
        $previous = [int]$sequence[$action].Sequence
    }
    Require ($sequence[$optionsDialog].Condition -ceq 'NOT Installed' -and
        $sequence.SetAmeStartMenuChoice.Condition -ceq '&AmeStartMenuFeature = 3' -and
        $sequence.SetAmeDesktopChoice.Condition -ceq '&AmeDesktopFeature = 3') `
        'options must retain fresh defaults and migrated shortcut states'

    $nextEvents = @(
        @{ Event = 'CheckTargetPath'; Argument = 'AMEAPPDIR'; Condition = '1' }
        @{ Event = 'SetTargetPath'; Argument = 'AMEAPPDIR'; Condition = '1' }
        @{ Event = 'AddLocal'; Argument = 'AmeStartMenuFeature'; Condition = 'AmeStartMenuChoice = "1"' }
        @{ Event = 'Remove'; Argument = 'AmeStartMenuFeature'; Condition = 'AmeStartMenuChoice <> "1"' }
        @{ Event = 'AddLocal'; Argument = 'AmeDesktopFeature'; Condition = 'AmeDesktopChoice = "1"' }
        @{ Event = 'Remove'; Argument = 'AmeDesktopFeature'; Condition = 'AmeDesktopChoice <> "1"' }
        @{ Event = 'NewDialog'; Argument = 'VerifyReadyDlg'; Condition = '1' }
    )
    $actualNext = @($Model.ControlEvent | Where-Object {
        $_.Dialog_ -ceq $optionsDialog -and $_.Control_ -ceq 'Next'
    } | Sort-Object { [int]$_.Ordering })
    Require ($actualNext.Count -eq $nextEvents.Count) 'options action count changed'
    for ($index = 0; $index -lt $nextEvents.Count; $index++) {
        $actual = $actualNext[$index]
        $expected = $nextEvents[$index]
        Require ($actual.Event -ceq $expected.Event -and $actual.Argument -ceq $expected.Argument -and
            $actual.Condition -ceq $expected.Condition -and [int]$actual.Ordering -eq $index + 1) `
            'options must validate the path and apply both shortcut choices before continuing'
    }

    $navigationEvents = @(
        @{ Dialog = $optionsDialog; Control = 'Browse'; Event = '[_BrowseProperty]'
            Argument = 'AMEAPPDIR'; Condition = '1'; Ordering = '1' }
        @{ Dialog = $optionsDialog; Control = 'Browse'; Event = 'SpawnDialog'
            Argument = 'BrowseDlg'; Condition = '1'; Ordering = '2' }
        @{ Dialog = 'BrowseDlg'; Control = 'OK'; Event = 'CheckTargetPath'
            Argument = '[_BrowseProperty]'; Condition = '1'; Ordering = '1' }
        @{ Dialog = 'BrowseDlg'; Control = 'OK'; Event = 'SetTargetPath'
            Argument = '[_BrowseProperty]'; Condition = '1'; Ordering = '2' }
        @{ Dialog = 'BrowseDlg'; Control = 'OK'; Event = 'EndDialog'
            Argument = 'Return'; Condition = '1'; Ordering = '3' }
        @{ Dialog = 'VerifyReadyDlg'; Control = 'Back'; Event = 'NewDialog'
            Argument = $optionsDialog; Condition = 'NOT Installed'; Ordering = '1' }
    )
    foreach ($event in $navigationEvents) {
        Require (@($Model.ControlEvent | Where-Object {
            $_.Dialog_ -ceq $event.Dialog -and $_.Control_ -ceq $event.Control -and
            $_.Event -ceq $event.Event -and $_.Argument -ceq $event.Argument -and
            $_.Condition -ceq $event.Condition -and $_.Ordering -ceq $event.Ordering
        }).Count -eq 1) 'directory browsing or return to selected options is disconnected'
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
