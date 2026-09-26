function Get-AmeWindowsEngineInputs {
    param([object]$Toolchain)

    $sdk = Split-Path -Parent (Split-Path -Parent $Toolchain.Flutter)
    $cache = Join-Path $sdk "bin/cache"
    $engine = Join-Path $cache "artifacts/engine/windows-x64"
    $dart = Join-Path $cache "dart-sdk/bin/dart.exe"
    $required = @(
        "engine.stamp", "engine-dart-sdk.stamp", "flutter_sdk.stamp", "windows-sdk.stamp",
        "engine_stamp.stamp", "flutter_tools.stamp", "flutter_tools.snapshot",
        "dart-sdk/bin/dart.exe", "dart-sdk/bin/dartaotruntime.exe",
        "dart-sdk/bin/snapshots/frontend_server_aot.dart.snapshot",
        "artifacts/engine/common/flutter_patched_sdk/platform_strong.dill",
        "artifacts/engine/windows-x64/flutter_windows.dll",
        "artifacts/engine/windows-x64/flutter_windows.dll.lib",
        "artifacts/engine/windows-x64/flutter_windows.h",
        "artifacts/engine/windows-x64/icudtl.dat",
        "artifacts/engine/windows-x64/cpp_client_wrapper/flutter_engine.cc",
        "artifacts/engine/windows-x64/cpp_client_wrapper/flutter_view_controller.cc"
    )
    foreach ($relative in $required) {
        $path = Join-Path $cache $relative
        $file = Get-Item -LiteralPath $path -ErrorAction Stop
        if ($file.PSIsContainer -or $file.Length -eq 0) {
            throw "Prepare the pinned SDK before the offline engine gate: $relative"
        }
    }
    $version = [IO.File]::ReadAllText((Join-Path $cache "engine.stamp")).Trim()
    if ($version -cnotmatch '^[0-9a-f]{40}$') { throw "Invalid prepared engine stamp" }
    foreach ($stamp in @("engine-dart-sdk.stamp", "flutter_sdk.stamp", "windows-sdk.stamp", "engine_stamp.stamp")) {
        if ([IO.File]::ReadAllText((Join-Path $cache $stamp)).Trim() -cne $version) {
            throw "The prepared engine artifacts have incompatible stamps: $stamp"
        }
    }
    return [pscustomobject]@{
        Dart = $dart
        EngineRoot = $engine
        IcuPath = Join-Path $engine "icudtl.dat"
        EngineRevision = $version
    }
}

function Assert-AmeWindowsEngineFixture {
    param([string]$SourceRoot)

    $entrypoint = [IO.File]::ReadAllText((Join-Path $SourceRoot "entrypoint.dart")).Trim()
    if ($entrypoint -cne "void main() {}") {
        throw "The engine lifecycle fixture must remain a no-op Dart entrypoint"
    }
    $pubspec = [IO.File]::ReadAllText((Join-Path $SourceRoot "pubspec.yaml"))
    if ($pubspec -cnotmatch '(?m)^name: ame_engine_lifecycle_fixture\r?$' -or
        $pubspec -cnotmatch '(?m)^publish_to: "none"\r?$' -or
        $pubspec -match '(?m)^(dependencies|dev_dependencies|dependency_overrides|workspace|resolution|flutter):') {
        throw "The isolated engine fixture may not declare packages, workspaces, or Flutter assets"
    }
    foreach ($line in $pubspec -split '\r?\n') {
        if ($line -cnotmatch '^(name: ame_engine_lifecycle_fixture|description: [^\r\n]+|publish_to: "none"|environment:|  sdk: \^[0-9]+\.[0-9]+\.[0-9]+|\s*)$') {
            throw "Unexpected engine fixture package configuration"
        }
    }
}

function Assert-AmeWindowsEnginePackage {
    param([string]$PackageRoot)

    $configPath = Join-Path $PackageRoot ".dart_tool/package_config.json"
    $config = [IO.File]::ReadAllText($configPath) | ConvertFrom-Json
    $packages = @($config.packages)
    if ($config.configVersion -ne 2 -or $packages.Count -ne 1 -or
        $packages[0].name -cne "ame_engine_lifecycle_fixture" -or
        $packages[0].packageUri -cne "lib/") {
        throw "The offline engine fixture must resolve only its own package"
    }
    $baseUri = [Uri]::new([IO.Path]::GetFullPath($configPath))
    $rootUri = [Uri]::new($baseUri, [string]$packages[0].rootUri)
    if (-not $rootUri.IsFile -or
        [IO.Path]::GetFullPath($rootUri.LocalPath).TrimEnd('\', '/') -ine
        [IO.Path]::GetFullPath($PackageRoot).TrimEnd('\', '/')) {
        throw "The no-op package resolved outside the isolated package root"
    }
}

function Get-AmeWindowsEngineKernel {
    param([string]$PackageRoot)

    $build = Join-Path $PackageRoot ".dart_tool/flutter_build"
    $stamps = @(Get-ChildItem -LiteralPath $build -Directory | ForEach-Object {
        $stamp = Join-Path $_.FullName "kernel_snapshot_program.stamp"
        if (Test-Path -LiteralPath $stamp -PathType Leaf) { Get-Item -LiteralPath $stamp }
    })
    if ($stamps.Count -ne 1 -or $stamps[0].Length -gt 1MB) {
        throw "The fresh kernel build must produce exactly one bounded target stamp"
    }
    $stamp = [IO.File]::ReadAllText($stamps[0].FullName) | ConvertFrom-Json
    $outputs = @($stamp.outputs | Select-Object -Unique)
    $expected = Join-Path $stamps[0].DirectoryName "app.dill"
    if ($outputs.Count -ne 1 -or -not ($outputs[0] -is [string]) -or
        -not [IO.Path]::IsPathRooted($outputs[0]) -or
        [IO.Path]::GetFullPath($outputs[0]) -ine [IO.Path]::GetFullPath($expected)) {
        throw "The kernel stamp must identify only its current app.dill output: expected=$expected; outputs=$($outputs -join ',')"
    }
    $kernel = Get-Item -LiteralPath $expected -ErrorAction Stop
    if ($kernel.PSIsContainer -or $kernel.Length -eq 0 -or $kernel.Length -gt 1MB -or
        ($kernel.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "The no-op engine kernel is missing, indirect, or exceeds its bound"
    }
    return $kernel.FullName
}
