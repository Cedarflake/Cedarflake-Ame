$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:AmeInstallerLanguages = [ordered]@{ 'en-US' = 1033; 'zh-CN' = 2052; 'ja-JP' = 1041 }
$script:AmeInstallerPresentation = [ordered]@{
    Dialog = @('Title')
    Control = @('Text', 'Help')
    TextStyle = @('FaceName')
    UIText = @('Text')
    Error = @('Message')
    ActionText = @('Description', 'Template')
    RadioButton = @('Text', 'Help')
    ComboBox = @('Text')
    ListBox = @('Text')
    ListView = @('Text')
}

function Read-AmeInstallerPresentationTable {
    param($Database, [string]$Table)

    $schema = $Database.Tables[$Table]
    $order = ($schema.PrimaryKeys | ForEach-Object { '`' + $_ + '`' }) -join ', '
    $view = $Database.OpenView(($schema.SqlSelectString + ' ORDER BY ' + $order), [object[]]@())
    $rows = [Collections.Generic.List[object]]::new()
    try {
        $view.Execute()
        while ($null -ne ($record = $view.Fetch())) {
            try {
                if ($rows.Count -ge 4096) { throw 'Installer presentation table exceeds its row bound' }
                $row = [ordered]@{}
                for ($index = 0; $index -lt $schema.Columns.Count; $index++) {
                    $value = $null
                    if (-not $record.IsNull($index + 1)) { $value = $record.GetString($index + 1) }
                    $row[$schema.Columns[$index].Name] = $value
                }
                $rows.Add([pscustomobject]$row)
            } finally { $record.Dispose() }
        }
    } finally { $view.Dispose() }
    return ,$rows.ToArray()
}

function Read-AmeInstallerLocalizedRows {
    param($Database, [int]$Language, [string]$Table)

    if (-not $script:AmeInstallerPresentation.Contains($Table) -or
        $Language -notin $script:AmeInstallerLanguages.Values) { throw 'Unknown installer language table' }
    $columns = $Database.Tables[$Table].Columns
    $query = 'SELECT `Row`, `Field`, `Value` FROM `AmeInstallerText` WHERE `Language` = ? AND `Table` = ? ORDER BY `Row`, `Field`'
    $view = $Database.OpenView($query, [object[]]@())
    $parameters = [WixToolset.Dtf.WindowsInstaller.Record]::new(2)
    $parameters.SetInteger(1, $Language)
    $parameters.SetString(2, $Table)
    $current = 0
    $field = 1
    $row = $null
    $cells = 0
    try {
        $view.Execute($parameters)
        while ($null -ne ($record = $view.Fetch())) {
            try {
                if (++$cells -gt 12000) { throw 'Installer language table exceeds its cell bound' }
                $number = $record.GetInteger(1)
                if ($number -ne $current) {
                    if ($number -ne $current + 1) { throw 'Installer language rows are incomplete' }
                    if ($null -ne $row) {
                        if ($field -ne $columns.Count + 1) { throw 'Installer language fields are incomplete' }
                        [pscustomobject]$row
                    }
                    $current = $number
                    $field = 1
                    $row = [ordered]@{}
                }
                if ($record.GetInteger(2) -ne $field -or $field -gt $columns.Count) {
                    throw 'Installer language field order is invalid'
                }
                $row[$columns[$field - 1].Name] = $record.GetString(3)
                $field++
            } finally { $record.Dispose() }
        }
        if ($null -ne $row) {
            if ($field -ne $columns.Count + 1) { throw 'Installer language fields are incomplete' }
            [pscustomobject]$row
        }
    } finally { $parameters.Dispose(); $view.Dispose() }
}

function Assert-AmeInstallerPresentationStructure {
    param($Baseline, $Localized, [string]$Table, [string[]]$TranslatedColumns)

    if ($Baseline.Count -ne $Localized.Count) { throw "Localization changes $Table row count" }
    for ($index = 0; $index -lt $Baseline.Count; $index++) {
        foreach ($column in $Baseline[$index].PSObject.Properties.Name) {
            if ($column -cin $TranslatedColumns) { continue }
            if ($Baseline[$index].$column -cne $Localized[$index].$column) {
                throw "Localization changes behavior in $Table.$column"
            }
        }
    }
}

function Merge-AmeInstallerLanguages {
    param(
        [Parameter(Mandatory = $true)][Collections.IDictionary]$Packages,
        [Parameter(Mandatory = $true)][string]$DtfAssemblyPath
    )

    Add-Type -Path $DtfAssemblyPath
    $catalogs = @{}
    foreach ($culture in $script:AmeInstallerLanguages.Keys) {
        $database = [WixToolset.Dtf.WindowsInstaller.Database]::new(
            $Packages[$culture], [WixToolset.Dtf.WindowsInstaller.DatabaseOpenMode]::ReadOnly)
        try {
            $catalog = [ordered]@{}
            foreach ($table in $script:AmeInstallerPresentation.Keys + @(
                    'ControlEvent', 'ControlCondition', 'EventMapping', 'LaunchCondition', 'CustomAction')) {
                if ($database.Tables.Contains($table)) {
                    $catalog[$table] = Read-AmeInstallerPresentationTable $database $table
                }
            }
            $catalogs[$culture] = $catalog
        } finally { $database.Dispose() }
    }
    $baseline = $catalogs['en-US']
    foreach ($culture in @('zh-CN', 'ja-JP')) {
        foreach ($table in $baseline.Keys) {
            if (-not $catalogs[$culture].Contains($table)) { throw "Missing $culture table $table" }
            $translated = @()
            if ($script:AmeInstallerPresentation.Contains($table)) { $translated = $script:AmeInstallerPresentation[$table] }
            if ($table -ceq 'LaunchCondition') { $translated = @('Description') }
            if ($table -ceq 'CustomAction') {
                $native = @($baseline[$table] | Where-Object Type -CNE '19')
                $localized = @($catalogs[$culture][$table] | Where-Object Type -CNE '19')
                Assert-AmeInstallerPresentationStructure $native $localized $table @()
                $translated = @('Target')
            }
            Assert-AmeInstallerPresentationStructure $baseline[$table] $catalogs[$culture][$table] $table $translated
        }
    }

    $database = [WixToolset.Dtf.WindowsInstaller.Database]::new(
        $Packages['en-US'], [WixToolset.Dtf.WindowsInstaller.DatabaseOpenMode]::Transact)
    try {
        $database.Execute('CREATE TABLE `AmeInstallerText` (`Language` SHORT NOT NULL, `Table` CHAR(32) NOT NULL, `Row` SHORT NOT NULL, `Field` SHORT NOT NULL, `Value` CHAR(0) PRIMARY KEY `Language`, `Table`, `Row`, `Field`)', [object[]]@())
        $database.Execute('CREATE TABLE `AmeInstallerMessage` (`Language` SHORT NOT NULL, `Property` CHAR(72) NOT NULL, `Value` CHAR(0) NOT NULL PRIMARY KEY `Language`, `Property`)', [object[]]@())
        $database.Execute('CREATE TABLE `AmeInstallerLocale` (`Language` SHORT NOT NULL PRIMARY KEY `Language`)', [object[]]@())
        $insert = $database.OpenView('INSERT INTO `AmeInstallerText` (`Language`, `Table`, `Row`, `Field`, `Value`) VALUES (?, ?, ?, ?, ?)', [object[]]@())
        try {
            foreach ($culture in $script:AmeInstallerLanguages.Keys) {
                foreach ($table in $script:AmeInstallerPresentation.Keys) {
                    if (-not $baseline.Contains($table)) { continue }
                    $rows = $catalogs[$culture][$table]
                    for ($index = 0; $index -lt $rows.Count; $index++) {
                        $column = 0
                        foreach ($field in $rows[$index].PSObject.Properties) {
                            $record = [WixToolset.Dtf.WindowsInstaller.Record]::new(5)
                            try {
                                $record.SetInteger(1, $script:AmeInstallerLanguages[$culture])
                                $record.SetString(2, $table)
                                $record.SetInteger(3, $index + 1)
                                $record.SetInteger(4, ++$column)
                                if ($null -ne $field.Value) { $record.SetString(5, $field.Value) }
                                $insert.Execute($record)
                            } finally { $record.Dispose() }
                        }
                    }
                }
            }
        } finally { $insert.Dispose() }

        foreach ($table in $script:AmeInstallerPresentation.Keys) {
            if ($baseline.Contains($table)) { $database.Execute(('DELETE FROM `' + $table + '`'), [object[]]@()) }
        }
        $messageId = 0
        foreach ($table in @('LaunchCondition', 'CustomAction')) {
            $key = if ($table -ceq 'LaunchCondition') { 'Condition' } else { 'Action' }
            $field = if ($table -ceq 'LaunchCondition') { 'Description' } else { 'Target' }
            foreach ($row in $baseline[$table]) {
                if ($table -ceq 'CustomAction' -and $row.Type -cne '19') { continue }
                $property = 'AmeMessage' + (++$messageId)
                foreach ($culture in $script:AmeInstallerLanguages.Keys) {
                    $translated = @($catalogs[$culture][$table] | Where-Object { $_.$key -ceq $row.$key })
                    if ($translated.Count -ne 1) { throw 'Ambiguous installer message' }
                    $record = [WixToolset.Dtf.WindowsInstaller.Record]::new(3)
                    try {
                        $record.SetInteger(1, $script:AmeInstallerLanguages[$culture])
                        $record.SetString(2, $property)
                        $record.SetString(3, $translated[0].$field)
                        $database.Execute('INSERT INTO `AmeInstallerMessage` (`Language`, `Property`, `Value`) VALUES (?, ?, ?)', $record)
                    } finally { $record.Dispose() }
                }
                $query = 'UPDATE `' + $table + '` SET `' + $field + '` = ? WHERE `' + $key + '` = ?'
                $record = [WixToolset.Dtf.WindowsInstaller.Record]::new(2)
                try {
                    $record.SetString(1, "[$property]")
                    $record.SetString(2, $row.$key)
                    $database.Execute($query, $record)
                } finally { $record.Dispose() }
            }
        }
        $database.Commit()
    } finally { $database.Dispose() }
}
