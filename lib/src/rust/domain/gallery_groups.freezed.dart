// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'gallery_groups.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$GalleryGroupKey {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GalleryGroupKey);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'GalleryGroupKey()';
}


}

/// @nodoc
class $GalleryGroupKeyCopyWith<$Res>  {
$GalleryGroupKeyCopyWith(GalleryGroupKey _, $Res Function(GalleryGroupKey) __);
}


/// Adds pattern-matching-related methods to [GalleryGroupKey].
extension GalleryGroupKeyPatterns on GalleryGroupKey {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( GalleryGroupKey_Exact value)?  exact,TResult Function( GalleryGroupKey_Similar value)?  similar,required TResult orElse(),}){
final _that = this;
switch (_that) {
case GalleryGroupKey_Exact() when exact != null:
return exact(_that);case GalleryGroupKey_Similar() when similar != null:
return similar(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( GalleryGroupKey_Exact value)  exact,required TResult Function( GalleryGroupKey_Similar value)  similar,}){
final _that = this;
switch (_that) {
case GalleryGroupKey_Exact():
return exact(_that);case GalleryGroupKey_Similar():
return similar(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( GalleryGroupKey_Exact value)?  exact,TResult? Function( GalleryGroupKey_Similar value)?  similar,}){
final _that = this;
switch (_that) {
case GalleryGroupKey_Exact() when exact != null:
return exact(_that);case GalleryGroupKey_Similar() when similar != null:
return similar(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String digest,  BigInt byteLength)?  exact,TResult Function( String groupId)?  similar,required TResult orElse(),}) {final _that = this;
switch (_that) {
case GalleryGroupKey_Exact() when exact != null:
return exact(_that.digest,_that.byteLength);case GalleryGroupKey_Similar() when similar != null:
return similar(_that.groupId);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String digest,  BigInt byteLength)  exact,required TResult Function( String groupId)  similar,}) {final _that = this;
switch (_that) {
case GalleryGroupKey_Exact():
return exact(_that.digest,_that.byteLength);case GalleryGroupKey_Similar():
return similar(_that.groupId);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String digest,  BigInt byteLength)?  exact,TResult? Function( String groupId)?  similar,}) {final _that = this;
switch (_that) {
case GalleryGroupKey_Exact() when exact != null:
return exact(_that.digest,_that.byteLength);case GalleryGroupKey_Similar() when similar != null:
return similar(_that.groupId);case _:
  return null;

}
}

}

/// @nodoc


class GalleryGroupKey_Exact extends GalleryGroupKey {
  const GalleryGroupKey_Exact({required this.digest, required this.byteLength}): super._();


 final  String digest;
 final  BigInt byteLength;

/// Create a copy of GalleryGroupKey
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GalleryGroupKey_ExactCopyWith<GalleryGroupKey_Exact> get copyWith => _$GalleryGroupKey_ExactCopyWithImpl<GalleryGroupKey_Exact>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GalleryGroupKey_Exact&&(identical(other.digest, digest) || other.digest == digest)&&(identical(other.byteLength, byteLength) || other.byteLength == byteLength));
}


@override
int get hashCode => Object.hash(runtimeType,digest,byteLength);

@override
String toString() {
  return 'GalleryGroupKey.exact(digest: $digest, byteLength: $byteLength)';
}


}

/// @nodoc
abstract mixin class $GalleryGroupKey_ExactCopyWith<$Res> implements $GalleryGroupKeyCopyWith<$Res> {
  factory $GalleryGroupKey_ExactCopyWith(GalleryGroupKey_Exact value, $Res Function(GalleryGroupKey_Exact) _then) = _$GalleryGroupKey_ExactCopyWithImpl;
@useResult
$Res call({
 String digest, BigInt byteLength
});




}
/// @nodoc
class _$GalleryGroupKey_ExactCopyWithImpl<$Res>
    implements $GalleryGroupKey_ExactCopyWith<$Res> {
  _$GalleryGroupKey_ExactCopyWithImpl(this._self, this._then);

  final GalleryGroupKey_Exact _self;
  final $Res Function(GalleryGroupKey_Exact) _then;

/// Create a copy of GalleryGroupKey
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? digest = null,Object? byteLength = null,}) {
  return _then(GalleryGroupKey_Exact(
digest: null == digest ? _self.digest : digest // ignore: cast_nullable_to_non_nullable
as String,byteLength: null == byteLength ? _self.byteLength : byteLength // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class GalleryGroupKey_Similar extends GalleryGroupKey {
  const GalleryGroupKey_Similar({required this.groupId}): super._();


 final  String groupId;

/// Create a copy of GalleryGroupKey
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GalleryGroupKey_SimilarCopyWith<GalleryGroupKey_Similar> get copyWith => _$GalleryGroupKey_SimilarCopyWithImpl<GalleryGroupKey_Similar>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GalleryGroupKey_Similar&&(identical(other.groupId, groupId) || other.groupId == groupId));
}


@override
int get hashCode => Object.hash(runtimeType,groupId);

@override
String toString() {
  return 'GalleryGroupKey.similar(groupId: $groupId)';
}


}

/// @nodoc
abstract mixin class $GalleryGroupKey_SimilarCopyWith<$Res> implements $GalleryGroupKeyCopyWith<$Res> {
  factory $GalleryGroupKey_SimilarCopyWith(GalleryGroupKey_Similar value, $Res Function(GalleryGroupKey_Similar) _then) = _$GalleryGroupKey_SimilarCopyWithImpl;
@useResult
$Res call({
 String groupId
});




}
/// @nodoc
class _$GalleryGroupKey_SimilarCopyWithImpl<$Res>
    implements $GalleryGroupKey_SimilarCopyWith<$Res> {
  _$GalleryGroupKey_SimilarCopyWithImpl(this._self, this._then);

  final GalleryGroupKey_Similar _self;
  final $Res Function(GalleryGroupKey_Similar) _then;

/// Create a copy of GalleryGroupKey
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? groupId = null,}) {
  return _then(GalleryGroupKey_Similar(
groupId: null == groupId ? _self.groupId : groupId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
