// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'organization.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$OrganizationGroupKey {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationGroupKey);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'OrganizationGroupKey()';
}


}

/// @nodoc
class $OrganizationGroupKeyCopyWith<$Res>  {
$OrganizationGroupKeyCopyWith(OrganizationGroupKey _, $Res Function(OrganizationGroupKey) __);
}


/// Adds pattern-matching-related methods to [OrganizationGroupKey].
extension OrganizationGroupKeyPatterns on OrganizationGroupKey {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( OrganizationGroupKey_Confirmed value)?  confirmed,TResult Function( OrganizationGroupKey_Provisional value)?  provisional,required TResult orElse(),}){
final _that = this;
switch (_that) {
case OrganizationGroupKey_Confirmed() when confirmed != null:
return confirmed(_that);case OrganizationGroupKey_Provisional() when provisional != null:
return provisional(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( OrganizationGroupKey_Confirmed value)  confirmed,required TResult Function( OrganizationGroupKey_Provisional value)  provisional,}){
final _that = this;
switch (_that) {
case OrganizationGroupKey_Confirmed():
return confirmed(_that);case OrganizationGroupKey_Provisional():
return provisional(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( OrganizationGroupKey_Confirmed value)?  confirmed,TResult? Function( OrganizationGroupKey_Provisional value)?  provisional,}){
final _that = this;
switch (_that) {
case OrganizationGroupKey_Confirmed() when confirmed != null:
return confirmed(_that);case OrganizationGroupKey_Provisional() when provisional != null:
return provisional(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String digest,  BigInt byteLength)?  confirmed,TResult Function( String assetId,  BigInt sourceGeneration)?  provisional,required TResult orElse(),}) {final _that = this;
switch (_that) {
case OrganizationGroupKey_Confirmed() when confirmed != null:
return confirmed(_that.digest,_that.byteLength);case OrganizationGroupKey_Provisional() when provisional != null:
return provisional(_that.assetId,_that.sourceGeneration);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String digest,  BigInt byteLength)  confirmed,required TResult Function( String assetId,  BigInt sourceGeneration)  provisional,}) {final _that = this;
switch (_that) {
case OrganizationGroupKey_Confirmed():
return confirmed(_that.digest,_that.byteLength);case OrganizationGroupKey_Provisional():
return provisional(_that.assetId,_that.sourceGeneration);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String digest,  BigInt byteLength)?  confirmed,TResult? Function( String assetId,  BigInt sourceGeneration)?  provisional,}) {final _that = this;
switch (_that) {
case OrganizationGroupKey_Confirmed() when confirmed != null:
return confirmed(_that.digest,_that.byteLength);case OrganizationGroupKey_Provisional() when provisional != null:
return provisional(_that.assetId,_that.sourceGeneration);case _:
  return null;

}
}

}

/// @nodoc


class OrganizationGroupKey_Confirmed extends OrganizationGroupKey {
  const OrganizationGroupKey_Confirmed({required this.digest, required this.byteLength}): super._();
  

 final  String digest;
 final  BigInt byteLength;

/// Create a copy of OrganizationGroupKey
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationGroupKey_ConfirmedCopyWith<OrganizationGroupKey_Confirmed> get copyWith => _$OrganizationGroupKey_ConfirmedCopyWithImpl<OrganizationGroupKey_Confirmed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationGroupKey_Confirmed&&(identical(other.digest, digest) || other.digest == digest)&&(identical(other.byteLength, byteLength) || other.byteLength == byteLength));
}


@override
int get hashCode => Object.hash(runtimeType,digest,byteLength);

@override
String toString() {
  return 'OrganizationGroupKey.confirmed(digest: $digest, byteLength: $byteLength)';
}


}

/// @nodoc
abstract mixin class $OrganizationGroupKey_ConfirmedCopyWith<$Res> implements $OrganizationGroupKeyCopyWith<$Res> {
  factory $OrganizationGroupKey_ConfirmedCopyWith(OrganizationGroupKey_Confirmed value, $Res Function(OrganizationGroupKey_Confirmed) _then) = _$OrganizationGroupKey_ConfirmedCopyWithImpl;
@useResult
$Res call({
 String digest, BigInt byteLength
});




}
/// @nodoc
class _$OrganizationGroupKey_ConfirmedCopyWithImpl<$Res>
    implements $OrganizationGroupKey_ConfirmedCopyWith<$Res> {
  _$OrganizationGroupKey_ConfirmedCopyWithImpl(this._self, this._then);

  final OrganizationGroupKey_Confirmed _self;
  final $Res Function(OrganizationGroupKey_Confirmed) _then;

/// Create a copy of OrganizationGroupKey
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? digest = null,Object? byteLength = null,}) {
  return _then(OrganizationGroupKey_Confirmed(
digest: null == digest ? _self.digest : digest // ignore: cast_nullable_to_non_nullable
as String,byteLength: null == byteLength ? _self.byteLength : byteLength // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class OrganizationGroupKey_Provisional extends OrganizationGroupKey {
  const OrganizationGroupKey_Provisional({required this.assetId, required this.sourceGeneration}): super._();
  

 final  String assetId;
 final  BigInt sourceGeneration;

/// Create a copy of OrganizationGroupKey
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationGroupKey_ProvisionalCopyWith<OrganizationGroupKey_Provisional> get copyWith => _$OrganizationGroupKey_ProvisionalCopyWithImpl<OrganizationGroupKey_Provisional>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationGroupKey_Provisional&&(identical(other.assetId, assetId) || other.assetId == assetId)&&(identical(other.sourceGeneration, sourceGeneration) || other.sourceGeneration == sourceGeneration));
}


@override
int get hashCode => Object.hash(runtimeType,assetId,sourceGeneration);

@override
String toString() {
  return 'OrganizationGroupKey.provisional(assetId: $assetId, sourceGeneration: $sourceGeneration)';
}


}

/// @nodoc
abstract mixin class $OrganizationGroupKey_ProvisionalCopyWith<$Res> implements $OrganizationGroupKeyCopyWith<$Res> {
  factory $OrganizationGroupKey_ProvisionalCopyWith(OrganizationGroupKey_Provisional value, $Res Function(OrganizationGroupKey_Provisional) _then) = _$OrganizationGroupKey_ProvisionalCopyWithImpl;
@useResult
$Res call({
 String assetId, BigInt sourceGeneration
});




}
/// @nodoc
class _$OrganizationGroupKey_ProvisionalCopyWithImpl<$Res>
    implements $OrganizationGroupKey_ProvisionalCopyWith<$Res> {
  _$OrganizationGroupKey_ProvisionalCopyWithImpl(this._self, this._then);

  final OrganizationGroupKey_Provisional _self;
  final $Res Function(OrganizationGroupKey_Provisional) _then;

/// Create a copy of OrganizationGroupKey
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? assetId = null,Object? sourceGeneration = null,}) {
  return _then(OrganizationGroupKey_Provisional(
assetId: null == assetId ? _self.assetId : assetId // ignore: cast_nullable_to_non_nullable
as String,sourceGeneration: null == sourceGeneration ? _self.sourceGeneration : sourceGeneration // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

// dart format on
