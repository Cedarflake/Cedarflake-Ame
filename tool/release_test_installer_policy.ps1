$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Test-AmeInstallerPolicyRegressions {
    param([Parameter(Mandatory = $true)]$Model, [Parameter(Mandatory = $true)]$Receipt)

    Assert-AmeInstallerPolicy $Model $Receipt
    $encoded = $Model | ConvertTo-Json -Depth 12
    $rejections = [ordered]@{
        wrong_architecture = {
            param($value)
            $value.Template = if ($value.Template -ceq 'x64;2052') { 'Arm64;2052' } else { 'x64;2052' }
        }
        foreign_upgrade_family = { param($value) ($value.Property | Where-Object Property -CEQ 'UpgradeCode').Value = [guid]::Empty.ToString('B') }
        foreign_service = { param($value) $value.ServiceInstall[0].Name = 'OtherService' }
        broad_privileges = { param($value) ($value.MsiServiceConfig | Where-Object ConfigType -CEQ '6').Argument = 'SeManageVolumePrivilege[~]SeImpersonatePrivilege[~]SeDebugPrivilege' }
        missing_caller_impersonation = { param($value) ($value.MsiServiceConfig | Where-Object ConfigType -CEQ '6').Argument = 'SeManageVolumePrivilege' }
        unrestricted_sid = { param($value) $value.MsiServiceConfig[0].Argument = '1' }
        pretransaction_removal = { param($value) ($value.InstallExecuteSequence | Where-Object Action -CEQ 'RemoveExistingProducts').Sequence = '1499' }
        late_previous_service_recovery = {
            param($value)
            ($value.InstallExecuteSequence | Where-Object Action -CEQ 'AmeRestorePreviousBroker').Sequence = '1600'
        }
        late_upgrade_execution = {
            param($value)
            ($value.InstallExecuteSequence | Where-Object Action -CEQ 'InstallExecute').Sequence = '6500'
        }
        previous_service_forward_action = {
            param($value)
            ($value.CustomAction | Where-Object Action -CEQ 'AmeRestorePreviousBroker').Type = '3073'
        }
        unbound_previous_service_recovery = {
            param($value)
            ($value.InstallExecuteSequence | Where-Object Action -CEQ 'AmeRestorePreviousBroker').Condition = '1'
        }
        early_service_start = { param($value) ($value.InstallExecuteSequence | Where-Object Action -CEQ 'AmeStartInstalledService').Sequence = '5849' }
        disabled_admission = { param($value) ($value.InstallExecuteSequence | Where-Object Action -CEQ 'AmeValidateInstallation').Condition = '0' }
        shell_action = { param($value) $value.CustomAction[0].Type = '34'; $value.CustomAction[0].Target = 'powershell.exe' }
        writable_broker = { param($value) ($value.MsiLockPermissionsEx | Where-Object LockObject -CEQ 'AmeBrokerExecutable').SDDLText += '(A;;FA;;;BU)' }
        writable_broker_runtime = {
            param($value)
            ($value.MsiLockPermissionsEx | Where-Object LockObject -CEQ 'AmeBrokerRuntime_vcruntime140_dll_file').SDDLText += '(A;;FA;;;BU)'
        }
        missing_broker_runtime = {
            param($value)
            $value.File = @($value.File | Where-Object File -CNE 'AmeBrokerRuntime_vcruntime140_dll_file')
        }
        misplaced_broker_runtime = {
            param($value)
            ($value.Component | Where-Object Component -CEQ 'AmeBrokerRuntime_vcruntime140_dll').Directory_ = 'AMEAPPDIR'
        }
        external_destination = { param($value) ($value.Directory | Where-Object Directory -CEQ 'AmeProductFolder').Directory_Parent = 'LocalAppDataFolder' }
        source_deletion = { param($value) $value.RemoveFile[0].FileName = '*' }
        optional_broker = { param($value) ($value.Feature | Where-Object Feature -CEQ 'Ame').Attributes = '0' }
        wrong_native_module = { param($value) $value.AdmissionHash = '0' * 64 }
        default_data_deletion = {
            param($value)
            $value.Property += [pscustomobject]@{ Property = 'AME_REMOVE_USER_DATA'; Value = '1' }
        }
        lost_elevated_data_choice = {
            param($value)
            $property = $value.Property | Where-Object Property -CEQ 'SecureCustomProperties'
            $property.Value = (($property.Value -split ';') | Where-Object { $_ -cne 'AME_REMOVE_USER_DATA' }) -join ';'
        }
        upgrade_data_deletion = {
            param($value)
            ($value.InstallExecuteSequence | Where-Object Action -CEQ 'AmeRemoveUserData').Condition = 'Installed'
        }
        precommit_data_deletion = {
            param($value)
            ($value.CustomAction | Where-Object Action -CEQ 'AmeRemoveUserData').Type = '1025'
        }
        system_data_deletion = {
            param($value)
            ($value.CustomAction | Where-Object Action -CEQ 'AmeRemoveUserData').Type = '3649'
        }
        detached_payload = { param($value) $value.Media[0].Cabinet = 'external.cab' }
        detached_last_cabinet = { param($value) $value.Media[-1].Cabinet = 'external.cab' }
        missing_payload = { param($value) $value.Media = @() }
        missing_maintenance = { param($value) ($value.Dialog | Where-Object Dialog -CEQ 'MaintenanceTypeDlg').Dialog = 'Missing' }
        mandatory_desktop_shortcut = { param($value) ($value.Feature | Where-Object Feature -CEQ 'AmeDesktopFeature').Attributes = '24' }
        foreign_registry_write = { param($value) $value.Registry[0].Key = 'SOFTWARE\OtherProduct' }
        missing_choice_page = {
            param($value)
            ($value.InstallUISequence | Where-Object Action -CEQ 'AmeInstallOptionsDlg').Condition = '0'
        }
        uneditable_installation_path = {
            param($value)
            ($value.Control | Where-Object { $_.Dialog_ -ceq 'AmeInstallOptionsDlg' -and $_.Control -ceq 'InstallPath' }).Type = 'Text'
        }
        missing_shortcut_checkbox = {
            param($value)
            ($value.Control | Where-Object { $_.Dialog_ -ceq 'AmeInstallOptionsDlg' -and $_.Control -ceq 'Desktop' }).Type = 'Text'
        }
        missing_unchecked_choice = {
            param($value)
            ($value.ControlEvent | Where-Object {
                $_.Dialog_ -ceq 'AmeInstallOptionsDlg' -and $_.Event -ceq 'Remove' -and $_.Argument -ceq 'AmeDesktopFeature'
            }).Condition = '0'
        }
        unconditional_shortcut = {
            param($value)
            ($value.ControlEvent | Where-Object {
                $_.Dialog_ -ceq 'AmeInstallOptionsDlg' -and $_.Event -ceq 'AddLocal' -and $_.Argument -ceq 'AmeStartMenuFeature'
            }).Condition = '1'
        }
        reset_migrated_shortcut = {
            param($value)
            ($value.InstallUISequence | Where-Object Action -CEQ 'SetAmeStartMenuChoice').Condition = '1'
        }
        initialization_after_choices = {
            param($value)
            ($value.InstallUISequence | Where-Object Action -CEQ 'SetAmeDesktopChoice').Sequence = '1399'
        }
        next_before_path_validation = {
            param($value)
            ($value.ControlEvent | Where-Object {
                $_.Dialog_ -ceq 'AmeInstallOptionsDlg' -and $_.Event -ceq 'NewDialog'
            }).Ordering = '0'
        }
        missing_start_rollback = { param($value) ($value.CustomAction | Where-Object Action -CEQ 'AmeStopInstalledService').Type = '3073' }
    }
    foreach ($name in $rejections.Keys) {
        $changed = $encoded | ConvertFrom-Json
        try { & $rejections[$name] $changed } catch {
            throw "Installer policy fixture $name could not be constructed: $($_.Exception.Message)"
        }
        $rejected = $false
        try { Assert-AmeInstallerPolicy $changed $Receipt } catch {
            if ($_.Exception.Message -notlike 'Installer policy:*') { throw }
            $rejected = $true
        }
        if (-not $rejected) { throw "Installer policy failed to reject $name" }
    }
    Write-Host "Installer database policy: valid package and $($rejections.Count) rejecting cases passed"
}
