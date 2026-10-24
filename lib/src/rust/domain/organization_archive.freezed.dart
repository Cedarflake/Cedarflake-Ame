// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'organization_archive.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$OrganizationArchiveEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationArchiveEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'OrganizationArchiveEvent()';
}


}

/// @nodoc
class $OrganizationArchiveEventCopyWith<$Res>  {
$OrganizationArchiveEventCopyWith(OrganizationArchiveEvent _, $Res Function(OrganizationArchiveEvent) __);
}


/// Adds pattern-matching-related methods to [OrganizationArchiveEvent].
extension OrganizationArchiveEventPatterns on OrganizationArchiveEvent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( OrganizationArchiveEvent_Progress value)?  progress,TResult Function( OrganizationArchiveEvent_Completed value)?  completed,TResult Function( OrganizationArchiveEvent_Cancelled value)?  cancelled,TResult Function( OrganizationArchiveEvent_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case OrganizationArchiveEvent_Progress() when progress != null:
return progress(_that);case OrganizationArchiveEvent_Completed() when completed != null:
return completed(_that);case OrganizationArchiveEvent_Cancelled() when cancelled != null:
return cancelled(_that);case OrganizationArchiveEvent_Failed() when failed != null:
return failed(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( OrganizationArchiveEvent_Progress value)  progress,required TResult Function( OrganizationArchiveEvent_Completed value)  completed,required TResult Function( OrganizationArchiveEvent_Cancelled value)  cancelled,required TResult Function( OrganizationArchiveEvent_Failed value)  failed,}){
final _that = this;
switch (_that) {
case OrganizationArchiveEvent_Progress():
return progress(_that);case OrganizationArchiveEvent_Completed():
return completed(_that);case OrganizationArchiveEvent_Cancelled():
return cancelled(_that);case OrganizationArchiveEvent_Failed():
return failed(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( OrganizationArchiveEvent_Progress value)?  progress,TResult? Function( OrganizationArchiveEvent_Completed value)?  completed,TResult? Function( OrganizationArchiveEvent_Cancelled value)?  cancelled,TResult? Function( OrganizationArchiveEvent_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case OrganizationArchiveEvent_Progress() when progress != null:
return progress(_that);case OrganizationArchiveEvent_Completed() when completed != null:
return completed(_that);case OrganizationArchiveEvent_Cancelled() when cancelled != null:
return cancelled(_that);case OrganizationArchiveEvent_Failed() when failed != null:
return failed(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( OrganizationArchiveProgress progress)?  progress,TResult Function( OrganizationArchiveReceipt receipt)?  completed,TResult Function()?  cancelled,TResult Function( String code,  String message)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case OrganizationArchiveEvent_Progress() when progress != null:
return progress(_that.progress);case OrganizationArchiveEvent_Completed() when completed != null:
return completed(_that.receipt);case OrganizationArchiveEvent_Cancelled() when cancelled != null:
return cancelled();case OrganizationArchiveEvent_Failed() when failed != null:
return failed(_that.code,_that.message);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( OrganizationArchiveProgress progress)  progress,required TResult Function( OrganizationArchiveReceipt receipt)  completed,required TResult Function()  cancelled,required TResult Function( String code,  String message)  failed,}) {final _that = this;
switch (_that) {
case OrganizationArchiveEvent_Progress():
return progress(_that.progress);case OrganizationArchiveEvent_Completed():
return completed(_that.receipt);case OrganizationArchiveEvent_Cancelled():
return cancelled();case OrganizationArchiveEvent_Failed():
return failed(_that.code,_that.message);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( OrganizationArchiveProgress progress)?  progress,TResult? Function( OrganizationArchiveReceipt receipt)?  completed,TResult? Function()?  cancelled,TResult? Function( String code,  String message)?  failed,}) {final _that = this;
switch (_that) {
case OrganizationArchiveEvent_Progress() when progress != null:
return progress(_that.progress);case OrganizationArchiveEvent_Completed() when completed != null:
return completed(_that.receipt);case OrganizationArchiveEvent_Cancelled() when cancelled != null:
return cancelled();case OrganizationArchiveEvent_Failed() when failed != null:
return failed(_that.code,_that.message);case _:
  return null;

}
}

}

/// @nodoc


class OrganizationArchiveEvent_Progress extends OrganizationArchiveEvent {
  const OrganizationArchiveEvent_Progress({required this.progress}): super._();


 final  OrganizationArchiveProgress progress;

/// Create a copy of OrganizationArchiveEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationArchiveEvent_ProgressCopyWith<OrganizationArchiveEvent_Progress> get copyWith => _$OrganizationArchiveEvent_ProgressCopyWithImpl<OrganizationArchiveEvent_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationArchiveEvent_Progress&&(identical(other.progress, progress) || other.progress == progress));
}


@override
int get hashCode => Object.hash(runtimeType,progress);

@override
String toString() {
  return 'OrganizationArchiveEvent.progress(progress: $progress)';
}


}

/// @nodoc
abstract mixin class $OrganizationArchiveEvent_ProgressCopyWith<$Res> implements $OrganizationArchiveEventCopyWith<$Res> {
  factory $OrganizationArchiveEvent_ProgressCopyWith(OrganizationArchiveEvent_Progress value, $Res Function(OrganizationArchiveEvent_Progress) _then) = _$OrganizationArchiveEvent_ProgressCopyWithImpl;
@useResult
$Res call({
 OrganizationArchiveProgress progress
});




}
/// @nodoc
class _$OrganizationArchiveEvent_ProgressCopyWithImpl<$Res>
    implements $OrganizationArchiveEvent_ProgressCopyWith<$Res> {
  _$OrganizationArchiveEvent_ProgressCopyWithImpl(this._self, this._then);

  final OrganizationArchiveEvent_Progress _self;
  final $Res Function(OrganizationArchiveEvent_Progress) _then;

/// Create a copy of OrganizationArchiveEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? progress = null,}) {
  return _then(OrganizationArchiveEvent_Progress(
progress: null == progress ? _self.progress : progress // ignore: cast_nullable_to_non_nullable
as OrganizationArchiveProgress,
  ));
}


}

/// @nodoc


class OrganizationArchiveEvent_Completed extends OrganizationArchiveEvent {
  const OrganizationArchiveEvent_Completed({required this.receipt}): super._();


 final  OrganizationArchiveReceipt receipt;

/// Create a copy of OrganizationArchiveEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationArchiveEvent_CompletedCopyWith<OrganizationArchiveEvent_Completed> get copyWith => _$OrganizationArchiveEvent_CompletedCopyWithImpl<OrganizationArchiveEvent_Completed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationArchiveEvent_Completed&&(identical(other.receipt, receipt) || other.receipt == receipt));
}


@override
int get hashCode => Object.hash(runtimeType,receipt);

@override
String toString() {
  return 'OrganizationArchiveEvent.completed(receipt: $receipt)';
}


}

/// @nodoc
abstract mixin class $OrganizationArchiveEvent_CompletedCopyWith<$Res> implements $OrganizationArchiveEventCopyWith<$Res> {
  factory $OrganizationArchiveEvent_CompletedCopyWith(OrganizationArchiveEvent_Completed value, $Res Function(OrganizationArchiveEvent_Completed) _then) = _$OrganizationArchiveEvent_CompletedCopyWithImpl;
@useResult
$Res call({
 OrganizationArchiveReceipt receipt
});




}
/// @nodoc
class _$OrganizationArchiveEvent_CompletedCopyWithImpl<$Res>
    implements $OrganizationArchiveEvent_CompletedCopyWith<$Res> {
  _$OrganizationArchiveEvent_CompletedCopyWithImpl(this._self, this._then);

  final OrganizationArchiveEvent_Completed _self;
  final $Res Function(OrganizationArchiveEvent_Completed) _then;

/// Create a copy of OrganizationArchiveEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? receipt = null,}) {
  return _then(OrganizationArchiveEvent_Completed(
receipt: null == receipt ? _self.receipt : receipt // ignore: cast_nullable_to_non_nullable
as OrganizationArchiveReceipt,
  ));
}


}

/// @nodoc


class OrganizationArchiveEvent_Cancelled extends OrganizationArchiveEvent {
  const OrganizationArchiveEvent_Cancelled(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationArchiveEvent_Cancelled);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'OrganizationArchiveEvent.cancelled()';
}


}




/// @nodoc


class OrganizationArchiveEvent_Failed extends OrganizationArchiveEvent {
  const OrganizationArchiveEvent_Failed({required this.code, required this.message}): super._();


 final  String code;
 final  String message;

/// Create a copy of OrganizationArchiveEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationArchiveEvent_FailedCopyWith<OrganizationArchiveEvent_Failed> get copyWith => _$OrganizationArchiveEvent_FailedCopyWithImpl<OrganizationArchiveEvent_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationArchiveEvent_Failed&&(identical(other.code, code) || other.code == code)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,code,message);

@override
String toString() {
  return 'OrganizationArchiveEvent.failed(code: $code, message: $message)';
}


}

/// @nodoc
abstract mixin class $OrganizationArchiveEvent_FailedCopyWith<$Res> implements $OrganizationArchiveEventCopyWith<$Res> {
  factory $OrganizationArchiveEvent_FailedCopyWith(OrganizationArchiveEvent_Failed value, $Res Function(OrganizationArchiveEvent_Failed) _then) = _$OrganizationArchiveEvent_FailedCopyWithImpl;
@useResult
$Res call({
 String code, String message
});




}
/// @nodoc
class _$OrganizationArchiveEvent_FailedCopyWithImpl<$Res>
    implements $OrganizationArchiveEvent_FailedCopyWith<$Res> {
  _$OrganizationArchiveEvent_FailedCopyWithImpl(this._self, this._then);

  final OrganizationArchiveEvent_Failed _self;
  final $Res Function(OrganizationArchiveEvent_Failed) _then;

/// Create a copy of OrganizationArchiveEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? code = null,Object? message = null,}) {
  return _then(OrganizationArchiveEvent_Failed(
code: null == code ? _self.code : code // ignore: cast_nullable_to_non_nullable
as String,message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
