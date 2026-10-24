// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'organization_batch.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$OrganizationBatchAction {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationBatchAction);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'OrganizationBatchAction()';
}


}

/// @nodoc
class $OrganizationBatchActionCopyWith<$Res>  {
$OrganizationBatchActionCopyWith(OrganizationBatchAction _, $Res Function(OrganizationBatchAction) __);
}


/// Adds pattern-matching-related methods to [OrganizationBatchAction].
extension OrganizationBatchActionPatterns on OrganizationBatchAction {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( OrganizationBatchAction_Favorite value)?  favorite,TResult Function( OrganizationBatchAction_Organize value)?  organize,required TResult orElse(),}){
final _that = this;
switch (_that) {
case OrganizationBatchAction_Favorite() when favorite != null:
return favorite(_that);case OrganizationBatchAction_Organize() when organize != null:
return organize(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( OrganizationBatchAction_Favorite value)  favorite,required TResult Function( OrganizationBatchAction_Organize value)  organize,}){
final _that = this;
switch (_that) {
case OrganizationBatchAction_Favorite():
return favorite(_that);case OrganizationBatchAction_Organize():
return organize(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( OrganizationBatchAction_Favorite value)?  favorite,TResult? Function( OrganizationBatchAction_Organize value)?  organize,}){
final _that = this;
switch (_that) {
case OrganizationBatchAction_Favorite() when favorite != null:
return favorite(_that);case OrganizationBatchAction_Organize() when organize != null:
return organize(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( bool favorite)?  favorite,TResult Function( OrganizationAction action)?  organize,required TResult orElse(),}) {final _that = this;
switch (_that) {
case OrganizationBatchAction_Favorite() when favorite != null:
return favorite(_that.favorite);case OrganizationBatchAction_Organize() when organize != null:
return organize(_that.action);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( bool favorite)  favorite,required TResult Function( OrganizationAction action)  organize,}) {final _that = this;
switch (_that) {
case OrganizationBatchAction_Favorite():
return favorite(_that.favorite);case OrganizationBatchAction_Organize():
return organize(_that.action);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( bool favorite)?  favorite,TResult? Function( OrganizationAction action)?  organize,}) {final _that = this;
switch (_that) {
case OrganizationBatchAction_Favorite() when favorite != null:
return favorite(_that.favorite);case OrganizationBatchAction_Organize() when organize != null:
return organize(_that.action);case _:
  return null;

}
}

}

/// @nodoc


class OrganizationBatchAction_Favorite extends OrganizationBatchAction {
  const OrganizationBatchAction_Favorite({required this.favorite}): super._();


 final  bool favorite;

/// Create a copy of OrganizationBatchAction
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationBatchAction_FavoriteCopyWith<OrganizationBatchAction_Favorite> get copyWith => _$OrganizationBatchAction_FavoriteCopyWithImpl<OrganizationBatchAction_Favorite>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationBatchAction_Favorite&&(identical(other.favorite, favorite) || other.favorite == favorite));
}


@override
int get hashCode => Object.hash(runtimeType,favorite);

@override
String toString() {
  return 'OrganizationBatchAction.favorite(favorite: $favorite)';
}


}

/// @nodoc
abstract mixin class $OrganizationBatchAction_FavoriteCopyWith<$Res> implements $OrganizationBatchActionCopyWith<$Res> {
  factory $OrganizationBatchAction_FavoriteCopyWith(OrganizationBatchAction_Favorite value, $Res Function(OrganizationBatchAction_Favorite) _then) = _$OrganizationBatchAction_FavoriteCopyWithImpl;
@useResult
$Res call({
 bool favorite
});




}
/// @nodoc
class _$OrganizationBatchAction_FavoriteCopyWithImpl<$Res>
    implements $OrganizationBatchAction_FavoriteCopyWith<$Res> {
  _$OrganizationBatchAction_FavoriteCopyWithImpl(this._self, this._then);

  final OrganizationBatchAction_Favorite _self;
  final $Res Function(OrganizationBatchAction_Favorite) _then;

/// Create a copy of OrganizationBatchAction
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? favorite = null,}) {
  return _then(OrganizationBatchAction_Favorite(
favorite: null == favorite ? _self.favorite : favorite // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class OrganizationBatchAction_Organize extends OrganizationBatchAction {
  const OrganizationBatchAction_Organize({required this.action}): super._();


 final  OrganizationAction action;

/// Create a copy of OrganizationBatchAction
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationBatchAction_OrganizeCopyWith<OrganizationBatchAction_Organize> get copyWith => _$OrganizationBatchAction_OrganizeCopyWithImpl<OrganizationBatchAction_Organize>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationBatchAction_Organize&&(identical(other.action, action) || other.action == action));
}


@override
int get hashCode => Object.hash(runtimeType,action);

@override
String toString() {
  return 'OrganizationBatchAction.organize(action: $action)';
}


}

/// @nodoc
abstract mixin class $OrganizationBatchAction_OrganizeCopyWith<$Res> implements $OrganizationBatchActionCopyWith<$Res> {
  factory $OrganizationBatchAction_OrganizeCopyWith(OrganizationBatchAction_Organize value, $Res Function(OrganizationBatchAction_Organize) _then) = _$OrganizationBatchAction_OrganizeCopyWithImpl;
@useResult
$Res call({
 OrganizationAction action
});


$OrganizationActionCopyWith<$Res> get action;

}
/// @nodoc
class _$OrganizationBatchAction_OrganizeCopyWithImpl<$Res>
    implements $OrganizationBatchAction_OrganizeCopyWith<$Res> {
  _$OrganizationBatchAction_OrganizeCopyWithImpl(this._self, this._then);

  final OrganizationBatchAction_Organize _self;
  final $Res Function(OrganizationBatchAction_Organize) _then;

/// Create a copy of OrganizationBatchAction
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? action = null,}) {
  return _then(OrganizationBatchAction_Organize(
action: null == action ? _self.action : action // ignore: cast_nullable_to_non_nullable
as OrganizationAction,
  ));
}

/// Create a copy of OrganizationBatchAction
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$OrganizationActionCopyWith<$Res> get action {

  return $OrganizationActionCopyWith<$Res>(_self.action, (value) {
    return _then(_self.copyWith(action: value));
  });
}
}

// dart format on
