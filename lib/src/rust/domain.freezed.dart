// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'domain.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ScanEvent {

 String get scanId;
/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEventCopyWith<ScanEvent> get copyWith => _$ScanEventCopyWithImpl<ScanEvent>(this as ScanEvent, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent&&(identical(other.scanId, scanId) || other.scanId == scanId));
}


@override
int get hashCode => Object.hash(runtimeType,scanId);

@override
String toString() {
  return 'ScanEvent(scanId: $scanId)';
}


}

/// @nodoc
abstract mixin class $ScanEventCopyWith<$Res>  {
  factory $ScanEventCopyWith(ScanEvent value, $Res Function(ScanEvent) _then) = _$ScanEventCopyWithImpl;
@useResult
$Res call({
 String scanId
});




}
/// @nodoc
class _$ScanEventCopyWithImpl<$Res>
    implements $ScanEventCopyWith<$Res> {
  _$ScanEventCopyWithImpl(this._self, this._then);

  final ScanEvent _self;
  final $Res Function(ScanEvent) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? scanId = null,}) {
  return _then(_self.copyWith(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [ScanEvent].
extension ScanEventPatterns on ScanEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ScanEvent_Started value)?  started,TResult Function( ScanEvent_Progress value)?  progress,TResult Function( ScanEvent_AssetDiscovered value)?  assetDiscovered,TResult Function( ScanEvent_Issue value)?  issue,TResult Function( ScanEvent_Completed value)?  completed,TResult Function( ScanEvent_Cancelled value)?  cancelled,TResult Function( ScanEvent_Paused value)?  paused,TResult Function( ScanEvent_Stale value)?  stale,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ScanEvent_Started() when started != null:
return started(_that);case ScanEvent_Progress() when progress != null:
return progress(_that);case ScanEvent_AssetDiscovered() when assetDiscovered != null:
return assetDiscovered(_that);case ScanEvent_Issue() when issue != null:
return issue(_that);case ScanEvent_Completed() when completed != null:
return completed(_that);case ScanEvent_Cancelled() when cancelled != null:
return cancelled(_that);case ScanEvent_Paused() when paused != null:
return paused(_that);case ScanEvent_Stale() when stale != null:
return stale(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ScanEvent_Started value)  started,required TResult Function( ScanEvent_Progress value)  progress,required TResult Function( ScanEvent_AssetDiscovered value)  assetDiscovered,required TResult Function( ScanEvent_Issue value)  issue,required TResult Function( ScanEvent_Completed value)  completed,required TResult Function( ScanEvent_Cancelled value)  cancelled,required TResult Function( ScanEvent_Paused value)  paused,required TResult Function( ScanEvent_Stale value)  stale,}){
final _that = this;
switch (_that) {
case ScanEvent_Started():
return started(_that);case ScanEvent_Progress():
return progress(_that);case ScanEvent_AssetDiscovered():
return assetDiscovered(_that);case ScanEvent_Issue():
return issue(_that);case ScanEvent_Completed():
return completed(_that);case ScanEvent_Cancelled():
return cancelled(_that);case ScanEvent_Paused():
return paused(_that);case ScanEvent_Stale():
return stale(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ScanEvent_Started value)?  started,TResult? Function( ScanEvent_Progress value)?  progress,TResult? Function( ScanEvent_AssetDiscovered value)?  assetDiscovered,TResult? Function( ScanEvent_Issue value)?  issue,TResult? Function( ScanEvent_Completed value)?  completed,TResult? Function( ScanEvent_Cancelled value)?  cancelled,TResult? Function( ScanEvent_Paused value)?  paused,TResult? Function( ScanEvent_Stale value)?  stale,}){
final _that = this;
switch (_that) {
case ScanEvent_Started() when started != null:
return started(_that);case ScanEvent_Progress() when progress != null:
return progress(_that);case ScanEvent_AssetDiscovered() when assetDiscovered != null:
return assetDiscovered(_that);case ScanEvent_Issue() when issue != null:
return issue(_that);case ScanEvent_Completed() when completed != null:
return completed(_that);case ScanEvent_Cancelled() when cancelled != null:
return cancelled(_that);case ScanEvent_Paused() when paused != null:
return paused(_that);case ScanEvent_Stale() when stale != null:
return stale(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String scanId,  String rootPath,  int? itemLimit,  int? entryLimit)?  started,TResult Function( String scanId,  BigInt visitedEntries,  BigInt acceptedItems,  BigInt issueCount)?  progress,TResult Function( String scanId,  AssetLocationView asset)?  assetDiscovered,TResult Function( String scanId,  ScanIssue issue)?  issue,TResult Function( String scanId,  String rootId,  BigInt assetCount,  BigInt issueCount,  String catalogPath,  bool wasLimited)?  completed,TResult Function( String scanId,  BigInt acceptedItems,  BigInt issueCount)?  cancelled,TResult Function( String scanId,  BigInt visitedEntries,  BigInt acceptedItems,  BigInt issueCount)?  paused,TResult Function( String scanId,  BigInt acceptedItems,  BigInt issueCount)?  stale,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ScanEvent_Started() when started != null:
return started(_that.scanId,_that.rootPath,_that.itemLimit,_that.entryLimit);case ScanEvent_Progress() when progress != null:
return progress(_that.scanId,_that.visitedEntries,_that.acceptedItems,_that.issueCount);case ScanEvent_AssetDiscovered() when assetDiscovered != null:
return assetDiscovered(_that.scanId,_that.asset);case ScanEvent_Issue() when issue != null:
return issue(_that.scanId,_that.issue);case ScanEvent_Completed() when completed != null:
return completed(_that.scanId,_that.rootId,_that.assetCount,_that.issueCount,_that.catalogPath,_that.wasLimited);case ScanEvent_Cancelled() when cancelled != null:
return cancelled(_that.scanId,_that.acceptedItems,_that.issueCount);case ScanEvent_Paused() when paused != null:
return paused(_that.scanId,_that.visitedEntries,_that.acceptedItems,_that.issueCount);case ScanEvent_Stale() when stale != null:
return stale(_that.scanId,_that.acceptedItems,_that.issueCount);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String scanId,  String rootPath,  int? itemLimit,  int? entryLimit)  started,required TResult Function( String scanId,  BigInt visitedEntries,  BigInt acceptedItems,  BigInt issueCount)  progress,required TResult Function( String scanId,  AssetLocationView asset)  assetDiscovered,required TResult Function( String scanId,  ScanIssue issue)  issue,required TResult Function( String scanId,  String rootId,  BigInt assetCount,  BigInt issueCount,  String catalogPath,  bool wasLimited)  completed,required TResult Function( String scanId,  BigInt acceptedItems,  BigInt issueCount)  cancelled,required TResult Function( String scanId,  BigInt visitedEntries,  BigInt acceptedItems,  BigInt issueCount)  paused,required TResult Function( String scanId,  BigInt acceptedItems,  BigInt issueCount)  stale,}) {final _that = this;
switch (_that) {
case ScanEvent_Started():
return started(_that.scanId,_that.rootPath,_that.itemLimit,_that.entryLimit);case ScanEvent_Progress():
return progress(_that.scanId,_that.visitedEntries,_that.acceptedItems,_that.issueCount);case ScanEvent_AssetDiscovered():
return assetDiscovered(_that.scanId,_that.asset);case ScanEvent_Issue():
return issue(_that.scanId,_that.issue);case ScanEvent_Completed():
return completed(_that.scanId,_that.rootId,_that.assetCount,_that.issueCount,_that.catalogPath,_that.wasLimited);case ScanEvent_Cancelled():
return cancelled(_that.scanId,_that.acceptedItems,_that.issueCount);case ScanEvent_Paused():
return paused(_that.scanId,_that.visitedEntries,_that.acceptedItems,_that.issueCount);case ScanEvent_Stale():
return stale(_that.scanId,_that.acceptedItems,_that.issueCount);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String scanId,  String rootPath,  int? itemLimit,  int? entryLimit)?  started,TResult? Function( String scanId,  BigInt visitedEntries,  BigInt acceptedItems,  BigInt issueCount)?  progress,TResult? Function( String scanId,  AssetLocationView asset)?  assetDiscovered,TResult? Function( String scanId,  ScanIssue issue)?  issue,TResult? Function( String scanId,  String rootId,  BigInt assetCount,  BigInt issueCount,  String catalogPath,  bool wasLimited)?  completed,TResult? Function( String scanId,  BigInt acceptedItems,  BigInt issueCount)?  cancelled,TResult? Function( String scanId,  BigInt visitedEntries,  BigInt acceptedItems,  BigInt issueCount)?  paused,TResult? Function( String scanId,  BigInt acceptedItems,  BigInt issueCount)?  stale,}) {final _that = this;
switch (_that) {
case ScanEvent_Started() when started != null:
return started(_that.scanId,_that.rootPath,_that.itemLimit,_that.entryLimit);case ScanEvent_Progress() when progress != null:
return progress(_that.scanId,_that.visitedEntries,_that.acceptedItems,_that.issueCount);case ScanEvent_AssetDiscovered() when assetDiscovered != null:
return assetDiscovered(_that.scanId,_that.asset);case ScanEvent_Issue() when issue != null:
return issue(_that.scanId,_that.issue);case ScanEvent_Completed() when completed != null:
return completed(_that.scanId,_that.rootId,_that.assetCount,_that.issueCount,_that.catalogPath,_that.wasLimited);case ScanEvent_Cancelled() when cancelled != null:
return cancelled(_that.scanId,_that.acceptedItems,_that.issueCount);case ScanEvent_Paused() when paused != null:
return paused(_that.scanId,_that.visitedEntries,_that.acceptedItems,_that.issueCount);case ScanEvent_Stale() when stale != null:
return stale(_that.scanId,_that.acceptedItems,_that.issueCount);case _:
  return null;

}
}

}

/// @nodoc


class ScanEvent_Started extends ScanEvent {
  const ScanEvent_Started({required this.scanId, required this.rootPath, this.itemLimit, this.entryLimit}): super._();


@override final  String scanId;
 final  String rootPath;
 final  int? itemLimit;
 final  int? entryLimit;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEvent_StartedCopyWith<ScanEvent_Started> get copyWith => _$ScanEvent_StartedCopyWithImpl<ScanEvent_Started>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent_Started&&(identical(other.scanId, scanId) || other.scanId == scanId)&&(identical(other.rootPath, rootPath) || other.rootPath == rootPath)&&(identical(other.itemLimit, itemLimit) || other.itemLimit == itemLimit)&&(identical(other.entryLimit, entryLimit) || other.entryLimit == entryLimit));
}


@override
int get hashCode => Object.hash(runtimeType,scanId,rootPath,itemLimit,entryLimit);

@override
String toString() {
  return 'ScanEvent.started(scanId: $scanId, rootPath: $rootPath, itemLimit: $itemLimit, entryLimit: $entryLimit)';
}


}

/// @nodoc
abstract mixin class $ScanEvent_StartedCopyWith<$Res> implements $ScanEventCopyWith<$Res> {
  factory $ScanEvent_StartedCopyWith(ScanEvent_Started value, $Res Function(ScanEvent_Started) _then) = _$ScanEvent_StartedCopyWithImpl;
@override @useResult
$Res call({
 String scanId, String rootPath, int? itemLimit, int? entryLimit
});




}
/// @nodoc
class _$ScanEvent_StartedCopyWithImpl<$Res>
    implements $ScanEvent_StartedCopyWith<$Res> {
  _$ScanEvent_StartedCopyWithImpl(this._self, this._then);

  final ScanEvent_Started _self;
  final $Res Function(ScanEvent_Started) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? scanId = null,Object? rootPath = null,Object? itemLimit = freezed,Object? entryLimit = freezed,}) {
  return _then(ScanEvent_Started(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,rootPath: null == rootPath ? _self.rootPath : rootPath // ignore: cast_nullable_to_non_nullable
as String,itemLimit: freezed == itemLimit ? _self.itemLimit : itemLimit // ignore: cast_nullable_to_non_nullable
as int?,entryLimit: freezed == entryLimit ? _self.entryLimit : entryLimit // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class ScanEvent_Progress extends ScanEvent {
  const ScanEvent_Progress({required this.scanId, required this.visitedEntries, required this.acceptedItems, required this.issueCount}): super._();


@override final  String scanId;
 final  BigInt visitedEntries;
 final  BigInt acceptedItems;
 final  BigInt issueCount;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEvent_ProgressCopyWith<ScanEvent_Progress> get copyWith => _$ScanEvent_ProgressCopyWithImpl<ScanEvent_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent_Progress&&(identical(other.scanId, scanId) || other.scanId == scanId)&&(identical(other.visitedEntries, visitedEntries) || other.visitedEntries == visitedEntries)&&(identical(other.acceptedItems, acceptedItems) || other.acceptedItems == acceptedItems)&&(identical(other.issueCount, issueCount) || other.issueCount == issueCount));
}


@override
int get hashCode => Object.hash(runtimeType,scanId,visitedEntries,acceptedItems,issueCount);

@override
String toString() {
  return 'ScanEvent.progress(scanId: $scanId, visitedEntries: $visitedEntries, acceptedItems: $acceptedItems, issueCount: $issueCount)';
}


}

/// @nodoc
abstract mixin class $ScanEvent_ProgressCopyWith<$Res> implements $ScanEventCopyWith<$Res> {
  factory $ScanEvent_ProgressCopyWith(ScanEvent_Progress value, $Res Function(ScanEvent_Progress) _then) = _$ScanEvent_ProgressCopyWithImpl;
@override @useResult
$Res call({
 String scanId, BigInt visitedEntries, BigInt acceptedItems, BigInt issueCount
});




}
/// @nodoc
class _$ScanEvent_ProgressCopyWithImpl<$Res>
    implements $ScanEvent_ProgressCopyWith<$Res> {
  _$ScanEvent_ProgressCopyWithImpl(this._self, this._then);

  final ScanEvent_Progress _self;
  final $Res Function(ScanEvent_Progress) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? scanId = null,Object? visitedEntries = null,Object? acceptedItems = null,Object? issueCount = null,}) {
  return _then(ScanEvent_Progress(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,visitedEntries: null == visitedEntries ? _self.visitedEntries : visitedEntries // ignore: cast_nullable_to_non_nullable
as BigInt,acceptedItems: null == acceptedItems ? _self.acceptedItems : acceptedItems // ignore: cast_nullable_to_non_nullable
as BigInt,issueCount: null == issueCount ? _self.issueCount : issueCount // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class ScanEvent_AssetDiscovered extends ScanEvent {
  const ScanEvent_AssetDiscovered({required this.scanId, required this.asset}): super._();


@override final  String scanId;
 final  AssetLocationView asset;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEvent_AssetDiscoveredCopyWith<ScanEvent_AssetDiscovered> get copyWith => _$ScanEvent_AssetDiscoveredCopyWithImpl<ScanEvent_AssetDiscovered>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent_AssetDiscovered&&(identical(other.scanId, scanId) || other.scanId == scanId)&&(identical(other.asset, asset) || other.asset == asset));
}


@override
int get hashCode => Object.hash(runtimeType,scanId,asset);

@override
String toString() {
  return 'ScanEvent.assetDiscovered(scanId: $scanId, asset: $asset)';
}


}

/// @nodoc
abstract mixin class $ScanEvent_AssetDiscoveredCopyWith<$Res> implements $ScanEventCopyWith<$Res> {
  factory $ScanEvent_AssetDiscoveredCopyWith(ScanEvent_AssetDiscovered value, $Res Function(ScanEvent_AssetDiscovered) _then) = _$ScanEvent_AssetDiscoveredCopyWithImpl;
@override @useResult
$Res call({
 String scanId, AssetLocationView asset
});




}
/// @nodoc
class _$ScanEvent_AssetDiscoveredCopyWithImpl<$Res>
    implements $ScanEvent_AssetDiscoveredCopyWith<$Res> {
  _$ScanEvent_AssetDiscoveredCopyWithImpl(this._self, this._then);

  final ScanEvent_AssetDiscovered _self;
  final $Res Function(ScanEvent_AssetDiscovered) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? scanId = null,Object? asset = null,}) {
  return _then(ScanEvent_AssetDiscovered(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,asset: null == asset ? _self.asset : asset // ignore: cast_nullable_to_non_nullable
as AssetLocationView,
  ));
}


}

/// @nodoc


class ScanEvent_Issue extends ScanEvent {
  const ScanEvent_Issue({required this.scanId, required this.issue}): super._();


@override final  String scanId;
 final  ScanIssue issue;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEvent_IssueCopyWith<ScanEvent_Issue> get copyWith => _$ScanEvent_IssueCopyWithImpl<ScanEvent_Issue>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent_Issue&&(identical(other.scanId, scanId) || other.scanId == scanId)&&(identical(other.issue, issue) || other.issue == issue));
}


@override
int get hashCode => Object.hash(runtimeType,scanId,issue);

@override
String toString() {
  return 'ScanEvent.issue(scanId: $scanId, issue: $issue)';
}


}

/// @nodoc
abstract mixin class $ScanEvent_IssueCopyWith<$Res> implements $ScanEventCopyWith<$Res> {
  factory $ScanEvent_IssueCopyWith(ScanEvent_Issue value, $Res Function(ScanEvent_Issue) _then) = _$ScanEvent_IssueCopyWithImpl;
@override @useResult
$Res call({
 String scanId, ScanIssue issue
});




}
/// @nodoc
class _$ScanEvent_IssueCopyWithImpl<$Res>
    implements $ScanEvent_IssueCopyWith<$Res> {
  _$ScanEvent_IssueCopyWithImpl(this._self, this._then);

  final ScanEvent_Issue _self;
  final $Res Function(ScanEvent_Issue) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? scanId = null,Object? issue = null,}) {
  return _then(ScanEvent_Issue(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,issue: null == issue ? _self.issue : issue // ignore: cast_nullable_to_non_nullable
as ScanIssue,
  ));
}


}

/// @nodoc


class ScanEvent_Completed extends ScanEvent {
  const ScanEvent_Completed({required this.scanId, required this.rootId, required this.assetCount, required this.issueCount, required this.catalogPath, required this.wasLimited}): super._();


@override final  String scanId;
 final  String rootId;
 final  BigInt assetCount;
 final  BigInt issueCount;
 final  String catalogPath;
 final  bool wasLimited;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEvent_CompletedCopyWith<ScanEvent_Completed> get copyWith => _$ScanEvent_CompletedCopyWithImpl<ScanEvent_Completed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent_Completed&&(identical(other.scanId, scanId) || other.scanId == scanId)&&(identical(other.rootId, rootId) || other.rootId == rootId)&&(identical(other.assetCount, assetCount) || other.assetCount == assetCount)&&(identical(other.issueCount, issueCount) || other.issueCount == issueCount)&&(identical(other.catalogPath, catalogPath) || other.catalogPath == catalogPath)&&(identical(other.wasLimited, wasLimited) || other.wasLimited == wasLimited));
}


@override
int get hashCode => Object.hash(runtimeType,scanId,rootId,assetCount,issueCount,catalogPath,wasLimited);

@override
String toString() {
  return 'ScanEvent.completed(scanId: $scanId, rootId: $rootId, assetCount: $assetCount, issueCount: $issueCount, catalogPath: $catalogPath, wasLimited: $wasLimited)';
}


}

/// @nodoc
abstract mixin class $ScanEvent_CompletedCopyWith<$Res> implements $ScanEventCopyWith<$Res> {
  factory $ScanEvent_CompletedCopyWith(ScanEvent_Completed value, $Res Function(ScanEvent_Completed) _then) = _$ScanEvent_CompletedCopyWithImpl;
@override @useResult
$Res call({
 String scanId, String rootId, BigInt assetCount, BigInt issueCount, String catalogPath, bool wasLimited
});




}
/// @nodoc
class _$ScanEvent_CompletedCopyWithImpl<$Res>
    implements $ScanEvent_CompletedCopyWith<$Res> {
  _$ScanEvent_CompletedCopyWithImpl(this._self, this._then);

  final ScanEvent_Completed _self;
  final $Res Function(ScanEvent_Completed) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? scanId = null,Object? rootId = null,Object? assetCount = null,Object? issueCount = null,Object? catalogPath = null,Object? wasLimited = null,}) {
  return _then(ScanEvent_Completed(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,rootId: null == rootId ? _self.rootId : rootId // ignore: cast_nullable_to_non_nullable
as String,assetCount: null == assetCount ? _self.assetCount : assetCount // ignore: cast_nullable_to_non_nullable
as BigInt,issueCount: null == issueCount ? _self.issueCount : issueCount // ignore: cast_nullable_to_non_nullable
as BigInt,catalogPath: null == catalogPath ? _self.catalogPath : catalogPath // ignore: cast_nullable_to_non_nullable
as String,wasLimited: null == wasLimited ? _self.wasLimited : wasLimited // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class ScanEvent_Cancelled extends ScanEvent {
  const ScanEvent_Cancelled({required this.scanId, required this.acceptedItems, required this.issueCount}): super._();


@override final  String scanId;
 final  BigInt acceptedItems;
 final  BigInt issueCount;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEvent_CancelledCopyWith<ScanEvent_Cancelled> get copyWith => _$ScanEvent_CancelledCopyWithImpl<ScanEvent_Cancelled>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent_Cancelled&&(identical(other.scanId, scanId) || other.scanId == scanId)&&(identical(other.acceptedItems, acceptedItems) || other.acceptedItems == acceptedItems)&&(identical(other.issueCount, issueCount) || other.issueCount == issueCount));
}


@override
int get hashCode => Object.hash(runtimeType,scanId,acceptedItems,issueCount);

@override
String toString() {
  return 'ScanEvent.cancelled(scanId: $scanId, acceptedItems: $acceptedItems, issueCount: $issueCount)';
}


}

/// @nodoc
abstract mixin class $ScanEvent_CancelledCopyWith<$Res> implements $ScanEventCopyWith<$Res> {
  factory $ScanEvent_CancelledCopyWith(ScanEvent_Cancelled value, $Res Function(ScanEvent_Cancelled) _then) = _$ScanEvent_CancelledCopyWithImpl;
@override @useResult
$Res call({
 String scanId, BigInt acceptedItems, BigInt issueCount
});




}
/// @nodoc
class _$ScanEvent_CancelledCopyWithImpl<$Res>
    implements $ScanEvent_CancelledCopyWith<$Res> {
  _$ScanEvent_CancelledCopyWithImpl(this._self, this._then);

  final ScanEvent_Cancelled _self;
  final $Res Function(ScanEvent_Cancelled) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? scanId = null,Object? acceptedItems = null,Object? issueCount = null,}) {
  return _then(ScanEvent_Cancelled(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,acceptedItems: null == acceptedItems ? _self.acceptedItems : acceptedItems // ignore: cast_nullable_to_non_nullable
as BigInt,issueCount: null == issueCount ? _self.issueCount : issueCount // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class ScanEvent_Paused extends ScanEvent {
  const ScanEvent_Paused({required this.scanId, required this.visitedEntries, required this.acceptedItems, required this.issueCount}): super._();


@override final  String scanId;
 final  BigInt visitedEntries;
 final  BigInt acceptedItems;
 final  BigInt issueCount;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEvent_PausedCopyWith<ScanEvent_Paused> get copyWith => _$ScanEvent_PausedCopyWithImpl<ScanEvent_Paused>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent_Paused&&(identical(other.scanId, scanId) || other.scanId == scanId)&&(identical(other.visitedEntries, visitedEntries) || other.visitedEntries == visitedEntries)&&(identical(other.acceptedItems, acceptedItems) || other.acceptedItems == acceptedItems)&&(identical(other.issueCount, issueCount) || other.issueCount == issueCount));
}


@override
int get hashCode => Object.hash(runtimeType,scanId,visitedEntries,acceptedItems,issueCount);

@override
String toString() {
  return 'ScanEvent.paused(scanId: $scanId, visitedEntries: $visitedEntries, acceptedItems: $acceptedItems, issueCount: $issueCount)';
}


}

/// @nodoc
abstract mixin class $ScanEvent_PausedCopyWith<$Res> implements $ScanEventCopyWith<$Res> {
  factory $ScanEvent_PausedCopyWith(ScanEvent_Paused value, $Res Function(ScanEvent_Paused) _then) = _$ScanEvent_PausedCopyWithImpl;
@override @useResult
$Res call({
 String scanId, BigInt visitedEntries, BigInt acceptedItems, BigInt issueCount
});




}
/// @nodoc
class _$ScanEvent_PausedCopyWithImpl<$Res>
    implements $ScanEvent_PausedCopyWith<$Res> {
  _$ScanEvent_PausedCopyWithImpl(this._self, this._then);

  final ScanEvent_Paused _self;
  final $Res Function(ScanEvent_Paused) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? scanId = null,Object? visitedEntries = null,Object? acceptedItems = null,Object? issueCount = null,}) {
  return _then(ScanEvent_Paused(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,visitedEntries: null == visitedEntries ? _self.visitedEntries : visitedEntries // ignore: cast_nullable_to_non_nullable
as BigInt,acceptedItems: null == acceptedItems ? _self.acceptedItems : acceptedItems // ignore: cast_nullable_to_non_nullable
as BigInt,issueCount: null == issueCount ? _self.issueCount : issueCount // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class ScanEvent_Stale extends ScanEvent {
  const ScanEvent_Stale({required this.scanId, required this.acceptedItems, required this.issueCount}): super._();


@override final  String scanId;
 final  BigInt acceptedItems;
 final  BigInt issueCount;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScanEvent_StaleCopyWith<ScanEvent_Stale> get copyWith => _$ScanEvent_StaleCopyWithImpl<ScanEvent_Stale>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScanEvent_Stale&&(identical(other.scanId, scanId) || other.scanId == scanId)&&(identical(other.acceptedItems, acceptedItems) || other.acceptedItems == acceptedItems)&&(identical(other.issueCount, issueCount) || other.issueCount == issueCount));
}


@override
int get hashCode => Object.hash(runtimeType,scanId,acceptedItems,issueCount);

@override
String toString() {
  return 'ScanEvent.stale(scanId: $scanId, acceptedItems: $acceptedItems, issueCount: $issueCount)';
}


}

/// @nodoc
abstract mixin class $ScanEvent_StaleCopyWith<$Res> implements $ScanEventCopyWith<$Res> {
  factory $ScanEvent_StaleCopyWith(ScanEvent_Stale value, $Res Function(ScanEvent_Stale) _then) = _$ScanEvent_StaleCopyWithImpl;
@override @useResult
$Res call({
 String scanId, BigInt acceptedItems, BigInt issueCount
});




}
/// @nodoc
class _$ScanEvent_StaleCopyWithImpl<$Res>
    implements $ScanEvent_StaleCopyWith<$Res> {
  _$ScanEvent_StaleCopyWithImpl(this._self, this._then);

  final ScanEvent_Stale _self;
  final $Res Function(ScanEvent_Stale) _then;

/// Create a copy of ScanEvent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? scanId = null,Object? acceptedItems = null,Object? issueCount = null,}) {
  return _then(ScanEvent_Stale(
scanId: null == scanId ? _self.scanId : scanId // ignore: cast_nullable_to_non_nullable
as String,acceptedItems: null == acceptedItems ? _self.acceptedItems : acceptedItems // ignore: cast_nullable_to_non_nullable
as BigInt,issueCount: null == issueCount ? _self.issueCount : issueCount // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

// dart format on
