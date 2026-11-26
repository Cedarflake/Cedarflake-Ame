$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_update_manifest.ps1')

Invoke-AmeAcceptanceScratch -Name 'update-manifest' -Action {
    param($directory)
    $pubspec = Join-Path $directory 'pubspec.yaml'
    $cargo = Join-Path $directory 'Cargo.toml'
    [IO.File]::WriteAllText($pubspec, "name: fixture`nversion: 1.2.3-beta.1+8`n", [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText($cargo, "[package]`nname = `"fixture`"`nversion = `"1.2.3-beta.1`"`n", [Text.UTF8Encoding]::new($false))
    $installer = Join-Path $directory 'Cedarflake-Ame-v1.2.3-beta.1-windows-x64.msi'
    $portable = Join-Path $directory 'CedarflakeAme-1.2.3-beta.1-full.nupkg'
    [IO.File]::WriteAllText($installer, 'installer fixture', [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText($portable, 'portable fixture', [Text.UTF8Encoding]::new($false))
    $path = Write-AmeUpdateManifest -Tag v1.2.3-beta.1 -PubspecPath $pubspec -CargoManifestPath $cargo `
        -Architecture x64 -InstallerPath $installer -PortablePackagePath $portable -OutputDirectory $directory
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    $manifest = Assert-AmeUpdateManifest -Directory $directory -Tag v1.2.3-beta.1 -Architecture x64 -ExpectedSha256 $hash
    if ($manifest.build -ne 8 -or $manifest.portable.size -ne 16) { throw 'Update manifest lost build or package size' }
    foreach ($case in @('changed-package', 'wrong-tag', 'wrong-architecture', 'changed-manifest')) {
        $parameters = @{ Directory=$directory; Tag='v1.2.3-beta.1'; Architecture='x64'; ExpectedSha256=$hash }
        switch ($case) {
            'changed-package' { [IO.File]::WriteAllText($portable, 'changed', [Text.UTF8Encoding]::new($false)) }
            'wrong-tag' { $parameters.Tag='v1.2.4' }
            'wrong-architecture' { $parameters.Architecture='arm64' }
            'changed-manifest' { $parameters.ExpectedSha256='0' * 64 }
        }
        $rejected = $false
        try { Assert-AmeUpdateManifest @parameters | Out-Null } catch { $rejected = $true }
        if (-not $rejected) { throw "Update metadata admitted $case" }
        [IO.File]::WriteAllText($portable, 'portable fixture', [Text.UTF8Encoding]::new($false))
    }
}
Write-Host 'Application update metadata fixtures passed'
