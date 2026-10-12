$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Test-AmeInstallerPolicyRegressions {
    param([Parameter(Mandatory = $true)]$Model, [Parameter(Mandatory = $true)]$Receipt)

    Assert-AmeInstallerPolicy $Model $Receipt
    $encoded = $Model | ConvertTo-Json -Depth 12
    $rejections = [ordered]@{
        foreign_service = { param($value) $value.ServiceInstall[0].Name = 'OtherService' }
        broad_privileges = { param($value) ($value.MsiServiceConfig | Where-Object ConfigType -CEQ '6').Argument = 'SeManageVolumePrivilege[~]SeImpersonatePrivilege[~]SeDebugPrivilege' }
        missing_caller_impersonation = { param($value) ($value.MsiServiceConfig | Where-Object ConfigType -CEQ '6').Argument = 'SeManageVolumePrivilege' }
        unrestricted_sid = { param($value) $value.MsiServiceConfig[0].Argument = '1' }
        pretransaction_removal = { param($value) ($value.InstallExecuteSequence | Where-Object Action -CEQ 'RemoveExistingProducts').Sequence = '1499' }
        early_service_start = { param($value) ($value.InstallExecuteSequence | Where-Object Action -CEQ 'AmeStartInstalledService').Sequence = '5849' }
        disabled_admission = { param($value) ($value.InstallExecuteSequence | Where-Object Action -CEQ 'AmeValidateInstallation').Condition = '0' }
        shell_action = { param($value) $value.CustomAction[0].Type = '34'; $value.CustomAction[0].Target = 'powershell.exe' }
        writable_broker = { param($value) ($value.MsiLockPermissionsEx | Where-Object LockObject -CEQ 'AmeBrokerExecutable').SDDLText += '(A;;FA;;;BU)' }
        external_destination = { param($value) ($value.Directory | Where-Object Directory -CEQ 'AmeProductFolder').Directory_Parent = 'LocalAppDataFolder' }
        source_deletion = { param($value) $value.RemoveFile[0].FileName = '*' }
        optional_broker = { param($value) ($value.Feature | Where-Object Feature -CEQ 'Ame').Attributes = '0' }
        wrong_native_module = { param($value) $value.AdmissionHash = '0' * 64 }
        detached_payload = { param($value) $value.Media[0].Cabinet = 'external.cab' }
        missing_maintenance = { param($value) ($value.Dialog | Where-Object Dialog -CEQ 'MaintenanceTypeDlg').Dialog = 'Missing' }
        mandatory_desktop_shortcut = { param($value) ($value.Feature | Where-Object Feature -CEQ 'AmeDesktopFeature').Attributes = '24' }
        foreign_registry_write = { param($value) $value.Registry[0].Key = 'SOFTWARE\OtherProduct' }
        missing_choice_page = { param($value) ($value.ControlEvent | Where-Object { $_.Dialog_ -ceq 'WelcomeDlg' -and $_.Control_ -ceq 'Next' -and $_.Event -ceq 'NewDialog' }).Argument = 'VerifyReadyDlg' }
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
