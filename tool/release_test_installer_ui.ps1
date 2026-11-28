[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$PackagePath,
    [Parameter(Mandatory = $true)][string]$DtfAssemblyPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot 'release_installer_database.ps1')
. (Join-Path $PSScriptRoot 'release_installer_ui.ps1')
$model = Read-AmeInstallerDatabase $PackagePath $DtfAssemblyPath
Assert-AmeInstallerUi $model
$previousUi = [WixToolset.Dtf.WindowsInstaller.Installer]::SetInternalUI(
    [WixToolset.Dtf.WindowsInstaller.InstallUIOptions]::Silent)
try {
    foreach ($language in @(1033, 2052, 1041)) {
        $localized = Read-AmeInstallerDatabase $PackagePath $DtfAssemblyPath -Language $language
        $session = [WixToolset.Dtf.WindowsInstaller.Installer]::OpenPackage(
            [IO.Path]::GetFullPath($PackagePath), $false)
        try {
            $session['AME_INSTALLER_LANGUAGE'] = [string]$language
            $session.DoAction('AmeLocalizeInstaller')
            $session.DoAction('AmeLocalizeInstaller')
            $actual = @(Read-AmeInstallerRows $session.Database 'Control' @('Dialog_', 'Control', 'Text'))
            if ($actual.Count -ne $localized.Control.Count) { throw 'Session did not load its complete language controls' }
            foreach ($control in $localized.Control) {
                $row = @($actual | Where-Object { $_.Dialog_ -ceq $control.Dialog_ -and $_.Control -ceq $control.Control })
                if ($row.Count -ne 1 -or $row[0].Text -cne $control.Text) {
                    throw 'Displayed controls do not match the selected installer language'
                }
            }
            $errors = @(Read-AmeInstallerRows $session.Database 'Error' @('Error', 'Message'))
            foreach ($number in @('25001', '25002', '25003')) {
                if (@($errors | Where-Object Error -CEQ $number).Count -ne 1) { throw 'Localized native failure is missing' }
            }
            if ($session['AME_REMOVE_USER_DATA'] -cne '') { throw 'Language initialization selected data deletion' }
        } finally { $session.Dispose() }
    }
    $initialize = @($model.InstallUISequence | Where-Object {
        $_.Action -cin @('SetAmeStartMenuChoice', 'SetAmeDesktopChoice')
    } | Sort-Object { [int]$_.Sequence })
    foreach ($property in @('AmeStartMenuChoice', 'AmeDesktopChoice')) {
        $actions = @($model.CustomAction | Where-Object Action -CEQ "Set$property")
        if ($actions.Count -ne 1 -or $actions[0].Type -cne '51' -or
            $actions[0].Source -cne $property -or $actions[0].Target -cne '1') {
            throw 'The option initializer must be an exact session-property action'
        }
    }
    $choices = @($model.ControlEvent | Where-Object {
        $_.Dialog_ -ceq 'AmeInstallOptionsDlg' -and $_.Control_ -ceq 'Next' -and
        $_.Event -cin @('AddLocal', 'Remove')
    } | Sort-Object { [int]$_.Ordering })
    $absentState = [WixToolset.Dtf.WindowsInstaller.InstallState]::Absent
    $localState = [WixToolset.Dtf.WindowsInstaller.InstallState]::Local
    $states = @($absentState, $localState)
    $caseCount = 0
    foreach ($initialStart in $states) {
        foreach ($initialDesktop in $states) {
            # Costing and property actions are session-local; never execute the installation sequence.
            $session = [WixToolset.Dtf.WindowsInstaller.Installer]::OpenPackage(
                [IO.Path]::GetFullPath($PackagePath), $false)
            try {
                $session.DoAction('AmeLocalizeInstaller')
                foreach ($action in @('CostInitialize', 'FileCost', 'CostFinalize')) { $session.DoAction($action) }
                $session.Features['AmeStartMenuFeature'].RequestState = $initialStart
                $session.Features['AmeDesktopFeature'].RequestState = $initialDesktop
                foreach ($action in $initialize) {
                    if ($session.EvaluateCondition($action.Condition)) { $session.DoAction($action.Action) }
                }
                if (($session['AmeStartMenuChoice'] -ceq '1') -ne ($initialStart -eq $localState) -or
                    ($session['AmeDesktopChoice'] -ceq '1') -ne ($initialDesktop -eq $localState)) {
                    throw 'Checkbox initialization discarded a costed or migrated feature choice'
                }
                foreach ($selectedStart in $states) {
                    foreach ($selectedDesktop in $states) {
                        $session['AmeStartMenuChoice'] = ''
                        $session['AmeDesktopChoice'] = ''
                        if ($selectedStart -eq $localState) { $session['AmeStartMenuChoice'] = '1' }
                        if ($selectedDesktop -eq $localState) { $session['AmeDesktopChoice'] = '1' }
                        $selected = @{}
                        foreach ($event in $choices) {
                            if (-not $session.EvaluateCondition($event.Condition)) { continue }
                            if ($selected.ContainsKey($event.Argument)) { throw 'Multiple decisions for one shortcut' }
                            $state = $absentState
                            if ($event.Event -ceq 'AddLocal') { $state = $localState }
                            $selected[$event.Argument] = $state
                        }
                        if ($selected.Count -ne 2 -or $selected.AmeStartMenuFeature -ne $selectedStart -or
                            $selected.AmeDesktopFeature -ne $selectedDesktop -or
                            $session.Features['Ame'].RequestState -ne $localState) {
                            throw 'Shortcut decisions differ from the checkboxes or change the mandatory product'
                        }
                        $caseCount++
                    }
                }
            } finally { $session.Dispose() }
        }
    }
    Write-Host "Installer native condition evaluation: $caseCount checkbox transitions passed without installation"
} finally {
    [void][WixToolset.Dtf.WindowsInstaller.Installer]::SetInternalUI($previousUi)
}
