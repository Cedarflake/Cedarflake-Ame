$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_bridge_contracts.ps1")

$contract = [AmeAsyncBridgeContract]::new("catalog", "load", "load", "crateLoad", "String", "")
$fixture = @{
    RustApi = 'pub fn load() -> Result<String, Error> { Ok("} frb(sync) {".to_owned()) }'
    DartApi = 'Future<String> load() => RustLib.instance.api.crateLoad();'
    DartGenerated = @'
@override Future<String> crateLoad() {
  return handler.executeNormal(NormalTask(() { return "} handler.executeSync() {"; }));
}
@override Future<String> nextMethod() { return handler.executeNormal(NormalTask()); }
'@
    RustGenerated = @'
fn wire__crate__api__catalog__load_impl() {
  FLUTTER_RUST_BRIDGE_HANDLER.wrap_normal::<Codec>(TaskInfo { mode: FfiCallMode::Normal }, || {});
}
fn wire__next_impl() {
  FLUTTER_RUST_BRIDGE_HANDLER.wrap_normal::<Codec>(TaskInfo { mode: FfiCallMode::Normal }, || {});
}
'@
}

function Assert-Fixture {
    param([hashtable]$Sources)
    Assert-AmeAsyncBridgeContract $contract `
        (ConvertTo-AmeBridgeCode $Sources.RustApi Rust) `
        (ConvertTo-AmeBridgeCode $Sources.DartApi Dart) `
        (ConvertTo-AmeBridgeCode $Sources.DartGenerated Dart) `
        (ConvertTo-AmeBridgeCode $Sources.RustGenerated Rust)
}

function Assert-Rejected {
    param([string]$Name, [string]$Field, [string]$Value)
    $changed = $fixture.Clone()
    $changed[$Field] = $Value
    try { Assert-Fixture $changed } catch { return }
    throw "Bridge guardrail accepted $Name"
}

Assert-Fixture $fixture
Assert-Rejected "target executeSync borrowing next executeNormal" DartGenerated (
    $fixture.DartGenerated.Replace('executeNormal(NormalTask(()', 'executeSync(SyncTask(()')
)
Assert-Rejected "target normal marker only in a string" DartGenerated (
    $fixture.DartGenerated.Replace('return handler.executeNormal(NormalTask(() { return "} handler.executeSync() {"; }));', 'return "handler.executeNormal(NormalTask())";')
)
Assert-Rejected "target normal marker only in a comment" DartGenerated (
    $fixture.DartGenerated.Replace('return handler.executeNormal(NormalTask(() { return "} handler.executeSync() {"; }));', '/* return handler.executeNormal(NormalTask()); */ return missing();')
)
Assert-Rejected "missing generated method" DartGenerated (
    $fixture.DartGenerated.Replace("crateLoad()", "differentName()")
)
Assert-Rejected "duplicated generated method" DartGenerated ($fixture.DartGenerated + "`n" + $fixture.DartGenerated)
Assert-Rejected "missing source method" RustApi 'pub fn other() {}'
Assert-Rejected "duplicated source method" RustApi ($fixture.RustApi + "`n" + $fixture.RustApi)
Assert-Rejected "synchronous Rust annotation" RustApi ("#[flutter_rust_bridge::frb(sync)]`n" + $fixture.RustApi)
Assert-Rejected "non-Future Dart wrapper" DartApi ($fixture.DartApi.Replace("Future<String>", "String"))
Assert-Rejected "Dart wrapper calls another wire" DartApi ($fixture.DartApi.Replace("api.crateLoad()", "api.nextMethod()"))
Assert-Rejected "duplicated Dart wrapper" DartApi ($fixture.DartApi + "`n" + $fixture.DartApi)
Assert-Rejected "missing Rust wire" RustGenerated ($fixture.RustGenerated.Replace("catalog__load_impl", "catalog__other_impl"))
Assert-Rejected "duplicated Rust wire" RustGenerated ($fixture.RustGenerated + "`n" + $fixture.RustGenerated)
Assert-Rejected "Rust sync wire borrowing following normal wire" RustGenerated (
    [regex]::new([regex]::Escape(
        "FLUTTER_RUST_BRIDGE_HANDLER.wrap_normal::<Codec>(TaskInfo { mode: FfiCallMode::Normal }, || {});"
    )).Replace($fixture.RustGenerated,
        "FLUTTER_RUST_BRIDGE_HANDLER.wrap_sync::<Codec>(TaskInfo { mode: FfiCallMode::Sync }, || {});", 1)
)
Assert-Rejected "malformed generated body" DartGenerated ($fixture.DartGenerated -replace '}\r?\n@override', '@override')

# Opaque methods are resolved inside their exact owner, not an unrelated impl/class.
$contract = [AmeAsyncBridgeContract]::new("viewer_source", "close", "close", "crateClose", "void", "ViewerSourceReadLease")
$opaque = @{
    RustApi = 'impl ViewerSourceReadLease { pub fn close(&self) -> Result<(), Error> { Ok(()) } }'
    DartApi = 'abstract class ViewerSourceReadLease implements RustOpaqueInterface { Future<void> close(); }'
    DartGenerated = @'
@override Future<void> crateClose() { return handler.executeNormal(NormalTask()); }
class ViewerSourceReadLeaseImpl extends RustOpaque implements ViewerSourceReadLease {
  Future<void> close() => RustLib.instance.api.crateClose(that: this);
}
'@
    RustGenerated = 'fn wire__crate__api__viewer_source__ViewerSourceReadLease_close_impl() { FLUTTER_RUST_BRIDGE_HANDLER.wrap_normal::<Codec>(TaskInfo { mode: FfiCallMode::Normal }, || {}); }'
}
Assert-Fixture $opaque
$fixture = $opaque
Assert-Rejected "wrong opaque owner" RustApi ($opaque.RustApi.Replace("impl ViewerSourceReadLease", "impl DifferentLease"))
Assert-Rejected "duplicate opaque owner" RustApi ($opaque.RustApi + "`n" + $opaque.RustApi)
Assert-Rejected "opaque sync annotation" RustApi ($opaque.RustApi.Replace("pub fn", "#[flutter_rust_bridge::frb(sync)] pub fn"))
Assert-Rejected "opaque implementation delegates another method" DartGenerated ($opaque.DartGenerated.Replace("api.crateClose(that:", "api.anotherClose(that:"))
Assert-Rejected "opaque implementation is a no-op" DartGenerated ($opaque.DartGenerated.Replace("=> RustLib.instance.api.crateClose(that: this);", "{ return Future<void>.value(); }"))
Assert-Rejected "opaque implementation executes sync despite normal wire" DartGenerated ($opaque.DartGenerated.Replace("=> RustLib.instance.api.crateClose(that: this);", "async { handler.executeSync(SyncTask()); }"))
Assert-Rejected "opaque implementation method is missing" DartGenerated ($opaque.DartGenerated.Replace("Future<void> close()", "Future<void> other()"))
Assert-Rejected "opaque implementation method is duplicated" DartGenerated ($opaque.DartGenerated.Replace("Future<void> close() => RustLib.instance.api.crateClose(that: this);", "Future<void> close() => RustLib.instance.api.crateClose(that: this);`nFuture<void> close() => RustLib.instance.api.crateClose(that: this);"))
Assert-Rejected "opaque implementation forwards another lease" DartGenerated ($opaque.DartGenerated.Replace("that: this", "that: other"))
Assert-Rejected "opaque implementation is not Future" DartGenerated ($opaque.DartGenerated.Replace("Future<void> close()", "void close()"))

$rustLiteral = @'
pub fn test<'a>(value: &'a str) { let brace = '}'; let text = r##" } /* literal */ "##; /* outer /* nested */ comment */ }
'@
$masked = ConvertTo-AmeBridgeCode $rustLiteral Rust
if ($masked.Length -ne $rustLiteral.Length -or $masked -match 'literal|comment') {
    throw "Bridge lexer lost source offsets or admitted literal/comment evidence"
}
Get-AmeBridgeClosingDelimiter $masked ($masked.IndexOf('{')) '{' '}' | Out-Null
$contracts = @(Get-AmeAsyncBridgeContracts)
if ($contracts.Count -ne 15 -or @($contracts | Where-Object Module -eq "viewer_source").Count -ne 3) {
    throw "The complete asynchronous bridge contract roster changed unexpectedly"
}
$queryContracts = @($contracts | Where-Object {
    $_.Module -eq "catalog" -and $_.RustName -eq "load_library_query_snapshot" -and
    $_.ReturnType -eq "GalleryQuerySnapshot"
})
if ($queryContracts.Count -ne 1) {
    throw "The atomic gallery query must retain its asynchronous bridge contract"
}
Write-Output "Bridge method-boundary guardrails passed."
