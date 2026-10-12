$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_installer_layout.ps1')
. (Join-Path $PSScriptRoot 'release_installer_ui.ps1')

function Assert-AmeInstallerPolicy {
    param([Parameter(Mandatory = $true)]$Model, [Parameter(Mandatory = $true)]$Receipt)

    function Require([bool]$Condition, [string]$Reason) {
        if (-not $Condition) { throw "Installer policy: $Reason" }
    }

    $properties = @{}
    foreach ($row in $Model.Property) { $properties[$row.Property] = $row.Value }
    Require ($Model.Template -ceq 'x64;2052') 'package must target Windows x64 with Chinese UI'
    Require ($properties.ProductCode -ceq ('{' + $Receipt.productCode + '}') -and
        $properties.UpgradeCode -ceq '{C284E6B0-1484-4926-A94B-C3EC3840DBBD}' -and
        $properties.ProductVersion -ceq $Receipt.productVersion -and $properties.ALLUSERS -ceq '1') 'product identity changed'
    Require ($properties.REBOOT -ceq 'ReallySuppress' -and
        $properties.MSIRESTARTMANAGERCONTROL -ceq 'Disable' -and
        $properties.MSIFASTINSTALL -ceq '1') 'restart or restore-point policy changed'
    Require ($Model.AdmissionHash -ceq $Receipt.admissionSha256) 'native admission module differs from its receipt'
    foreach ($table in @('MsiServiceConfigFailureActions', 'RemoveRegistry', 'Environment',
            'IniFile', 'RemoveIniFile', 'DuplicateFile', 'MoveFiles', 'SelfReg', 'Class', 'TypeLib')) {
        Require ($Model.Tables -cnotcontains $table) "unexpected mutating table $table"
    }

    Require ($Model.ServiceInstall.Count -eq 1 -and $Model.ServiceControl.Count -eq 1 -and
        $Model.MsiServiceConfig.Count -eq 2) 'service rows are incomplete or ambiguous'
    $service = $Model.ServiceInstall[0]
    Require ($service.Name -ceq 'CedarflakeAmeJournalBroker' -and $service.ServiceType -ceq '16' -and
        $service.StartType -ceq '3' -and $service.ErrorControl -ceq '32769' -and
        $service.StartName -ceq 'LocalSystem' -and $service.Password -ceq '' -and
        $service.Arguments -ceq '' -and $service.Component_ -ceq 'AmeBroker' -and
        $service.Description -ceq $Receipt.brokerIdentity) 'service identity changed'
    $control = $Model.ServiceControl[0]
    Require ($control.Name -ceq $service.Name -and $control.Event -ceq '162' -and
        $control.Wait -ceq '1' -and $control.Arguments -ceq '' -and $control.Component_ -ceq 'AmeBroker') 'service lifecycle changed'
    $types = @{}
    foreach ($row in $Model.MsiServiceConfig) {
        Require ($row.Name -ceq $service.Name -and $row.Event -ceq '5' -and
            $row.Component_ -ceq 'AmeBroker' -and -not $types.ContainsKey($row.ConfigType)) 'invalid service configuration row'
        $types[$row.ConfigType] = $row.Argument
    }
    Require ($types['5'] -ceq '3' -and $types['6'] -ceq 'SeManageVolumePrivilege[~]SeImpersonatePrivilege') 'service SID or privileges changed'

    $allowedActions = @{
        SetARPINSTALLLOCATION = @('51', 'ARPINSTALLLOCATION', '[AMEAPPDIR]')
        SetAMEAPPDIR = @('51', 'AMEAPPDIR', '[AME_PREVIOUSAPPDIR]')
        SetREINSTALLMODE = @('51', 'REINSTALLMODE', 'amus')
        SetAmeStartMenuChoice = @('51', 'AmeStartMenuChoice', '1')
        SetAmeDesktopChoice = @('51', 'AmeDesktopChoice', '1')
        AmeValidateInstallation = @('1', 'AmeInstallerAdmission', 'AmeValidateInstallation')
        AmeRequireRollback = @('19', '', $null)
        SetAmeVerifyInstalledService = @('51', 'AmeVerifyInstalledService', $Receipt.brokerIdentity)
        AmeVerifyInstalledService = @('3073', 'AmeInstallerAdmission', 'AmeVerifyInstalledService')
        SetAmeStopInstalledService = @('51', 'AmeStopInstalledService', $Receipt.brokerIdentity)
        AmeStopInstalledService = @('3329', 'AmeInstallerAdmission', 'AmeStopInstalledService')
        SetAmeStartInstalledService = @('51', 'AmeStartInstalledService', $Receipt.brokerIdentity)
        AmeStartInstalledService = @('3073', 'AmeInstallerAdmission', 'AmeStartInstalledService')
    }
    Require ($Model.CustomAction.Count -eq $allowedActions.Count) 'unexpected custom action count'
    foreach ($row in $Model.CustomAction) {
        Require ($allowedActions.ContainsKey($row.Action)) 'unrecognized custom action'
        $expected = $allowedActions[$row.Action]
        $targetMatches = $row.Target -ceq $expected[2]
        if ($null -eq $expected[2]) { $targetMatches = $row.Target.Length -gt 0 }
        Require ($row.Type -ceq $expected[0] -and $row.Source -ceq $expected[1] -and
            $targetMatches) "unsafe custom action $($row.Action)"
    }
    $sequence = @{}
    foreach ($row in $Model.InstallExecuteSequence) { $sequence[$row.Action] = $row }
    $orderedActions = @('AmeValidateInstallation', 'InstallValidate', 'InstallInitialize',
        'AmeRequireRollback', 'RemoveExistingProducts', 'ProcessComponents', 'StopServices',
        'DeleteServices', 'RemoveFiles', 'InstallFiles', 'InstallServices', 'MsiConfigureServices',
        'SetAmeVerifyInstalledService', 'AmeVerifyInstalledService',
        'SetAmeStopInstalledService', 'AmeStopInstalledService',
        'SetAmeStartInstalledService', 'AmeStartInstalledService', 'InstallFinalize')
    $previous = 0
    foreach ($action in $orderedActions) {
        Require ($sequence.ContainsKey($action)) "missing sequenced action $action"
        $position = [int]$sequence[$action].Sequence
        Require ($position -gt $previous) "unsafe action ordering at $action"
        $previous = $position
    }
    Require ($sequence.AmeValidateInstallation.Condition -ceq '' -and
        $sequence.AmeRequireRollback.Condition -ceq 'RollbackDisabled OR DISABLEROLLBACK' -and
        $sequence.AmeVerifyInstalledService.Condition -ceq '$AmeBroker = 3' -and
        $sequence.AmeStartInstalledService.Condition -ceq '$AmeBroker = 3' -and
        $sequence.AmeStopInstalledService.Condition -ceq '$AmeBroker = 3') 'admission conditions changed'
    Require ($sequence.SetAMEAPPDIR.Condition -ceq 'NOT AMEAPPDIR AND AME_PREVIOUSAPPDIR' -and
        [int]$sequence.SetAMEAPPDIR.Sequence -lt [int]$sequence.CostInitialize.Sequence) 'registered location restoration changed'
    Require ([int]$sequence.SetREINSTALLMODE.Sequence -lt [int]$sequence.CostInitialize.Sequence) 'repair must reinstall owned files'
    Require (@($Model.LaunchCondition | Where-Object { $_.Condition -ceq 'NOT RollbackDisabled AND NOT DISABLEROLLBACK' }).Count -eq 1) 'rollback launch guard missing'
    Require (@($Model.LaunchCondition | Where-Object { $_.Condition -ceq 'VersionNT64 AND MsiNTProductType = 1 AND AME_WINDOWS_BUILD >= 22000' }).Count -eq 1) 'supported Windows launch guard missing'
    Require ($Model.Upgrade.Count -eq 2) 'upgrade recognition is incomplete'
    $older = @($Model.Upgrade | Where-Object ActionProperty -CEQ 'WIX_UPGRADE_DETECTED')
    $newer = @($Model.Upgrade | Where-Object ActionProperty -CEQ 'WIX_DOWNGRADE_DETECTED')
    Require ($older.Count -eq 1 -and $newer.Count -eq 1 -and
        $older[0].UpgradeCode -ceq $properties.UpgradeCode -and $newer[0].UpgradeCode -ceq $properties.UpgradeCode -and
        $older[0].VersionMin -ceq '' -and $older[0].VersionMax -ceq $Receipt.productVersion -and
        $older[0].Attributes -ceq '1' -and $older[0].Remove -ceq '' -and
        $newer[0].VersionMin -ceq $Receipt.productVersion -and $newer[0].VersionMax -ceq '' -and
        $newer[0].Attributes -ceq '2') 'version upgrade policy changed'
    Require (@($Model.LaunchCondition | Where-Object { $_.Condition -ceq 'NOT WIX_DOWNGRADE_DETECTED' }).Count -eq 1) 'downgrade rejection missing'

    Assert-AmeInstallerLayout $Model $Receipt
    Assert-AmeInstallerUi $Model
}
