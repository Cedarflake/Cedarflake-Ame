// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'organization_cleanup.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$OrganizationCleanupEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationCleanupEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'OrganizationCleanupEvent()';
}


}

/// @nodoc
class $OrganizationCleanupEventCopyWith<$Res>  {
$OrganizationCleanupEventCopyWith(OrganizationCleanupEvent _, $Res Function(OrganizationCleanupEvent) __);
}


/// Adds pattern-matching-related methods to [OrganizationCleanupEvent].
extension OrganizationCleanupEventPatterns on OrganizationCleanupEvent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( OrganizationCleanupEvent_Progress value)?  progress,TResult Function( OrganizationCleanupEvent_Inspected value)?  inspected,TResult Function( OrganizationCleanupEvent_Completed value)?  completed,TResult Function( OrganizationCleanupEvent_Cancelled value)?  cancelled,TResult Function( OrganizationCleanupEvent_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case OrganizationCleanupEvent_Progress() when progress != null:
return progress(_that);case OrganizationCleanupEvent_Inspected() when inspected != null:
return inspected(_that);case OrganizationCleanupEvent_Completed() when completed != null:
return completed(_that);case OrganizationCleanupEvent_Cancelled() when cancelled != null:
return cancelled(_that);case OrganizationCleanupEvent_Failed() when failed != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( OrganizationCleanupEvent_Progress value)  progress,required TResult Function( OrganizationCleanupEvent_Inspected value)  inspected,required TResult Function( OrganizationCleanupEvent_Completed value)  completed,required TResult Function( OrganizationCleanupEvent_Cancelled value)  cancelled,required TResult Function( OrganizationCleanupEvent_Failed value)  failed,}){
final _that = this;
switch (_that) {
case OrganizationCleanupEvent_Progress():
return progress(_that);case OrganizationCleanupEvent_Inspected():
return inspected(_that);case OrganizationCleanupEvent_Completed():
return completed(_that);case OrganizationCleanupEvent_Cancelled():
return cancelled(_that);case OrganizationCleanupEvent_Failed():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( OrganizationCleanupEvent_Progress value)?  progress,TResult? Function( OrganizationCleanupEvent_Inspected value)?  inspected,TResult? Function( OrganizationCleanupEvent_Completed value)?  completed,TResult? Function( OrganizationCleanupEvent_Cancelled value)?  cancelled,TResult? Function( OrganizationCleanupEvent_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case OrganizationCleanupEvent_Progress() when progress != null:
return progress(_that);case OrganizationCleanupEvent_Inspected() when inspected != null:
return inspected(_that);case OrganizationCleanupEvent_Completed() when completed != null:
return completed(_that);case OrganizationCleanupEvent_Cancelled() when cancelled != null:
return cancelled(_that);case OrganizationCleanupEvent_Failed() when failed != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( OrganizationCleanupProgress progress)?  progress,TResult Function( OrganizationCleanupInspection inspection)?  inspected,TResult Function( OrganizationCleanupReceipt receipt)?  completed,TResult Function()?  cancelled,TResult Function( String code,  String message)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case OrganizationCleanupEvent_Progress() when progress != null:
return progress(_that.progress);case OrganizationCleanupEvent_Inspected() when inspected != null:
return inspected(_that.inspection);case OrganizationCleanupEvent_Completed() when completed != null:
return completed(_that.receipt);case OrganizationCleanupEvent_Cancelled() when cancelled != null:
return cancelled();case OrganizationCleanupEvent_Failed() when failed != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( OrganizationCleanupProgress progress)  progress,required TResult Function( OrganizationCleanupInspection inspection)  inspected,required TResult Function( OrganizationCleanupReceipt receipt)  completed,required TResult Function()  cancelled,required TResult Function( String code,  String message)  failed,}) {final _that = this;
switch (_that) {
case OrganizationCleanupEvent_Progress():
return progress(_that.progress);case OrganizationCleanupEvent_Inspected():
return inspected(_that.inspection);case OrganizationCleanupEvent_Completed():
return completed(_that.receipt);case OrganizationCleanupEvent_Cancelled():
return cancelled();case OrganizationCleanupEvent_Failed():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( OrganizationCleanupProgress progress)?  progress,TResult? Function( OrganizationCleanupInspection inspection)?  inspected,TResult? Function( OrganizationCleanupReceipt receipt)?  completed,TResult? Function()?  cancelled,TResult? Function( String code,  String message)?  failed,}) {final _that = this;
switch (_that) {
case OrganizationCleanupEvent_Progress() when progress != null:
return progress(_that.progress);case OrganizationCleanupEvent_Inspected() when inspected != null:
return inspected(_that.inspection);case OrganizationCleanupEvent_Completed() when completed != null:
return completed(_that.receipt);case OrganizationCleanupEvent_Cancelled() when cancelled != null:
return cancelled();case OrganizationCleanupEvent_Failed() when failed != null:
return failed(_that.code,_that.message);case _:
  return null;

}
}

}

/// @nodoc


class OrganizationCleanupEvent_Progress extends OrganizationCleanupEvent {
  const OrganizationCleanupEvent_Progress({required this.progress}): super._();


 final  OrganizationCleanupProgress progress;

/// Create a copy of OrganizationCleanupEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationCleanupEvent_ProgressCopyWith<OrganizationCleanupEvent_Progress> get copyWith => _$OrganizationCleanupEvent_ProgressCopyWithImpl<OrganizationCleanupEvent_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationCleanupEvent_Progress&&(identical(other.progress, progress) || other.progress == progress));
}


@override
int get hashCode => Object.hash(runtimeType,progress);

@override
String toString() {
  return 'OrganizationCleanupEvent.progress(progress: $progress)';
}


}

/// @nodoc
abstract mixin class $OrganizationCleanupEvent_ProgressCopyWith<$Res> implements $OrganizationCleanupEventCopyWith<$Res> {
  factory $OrganizationCleanupEvent_ProgressCopyWith(OrganizationCleanupEvent_Progress value, $Res Function(OrganizationCleanupEvent_Progress) _then) = _$OrganizationCleanupEvent_ProgressCopyWithImpl;
@useResult
$Res call({
 OrganizationCleanupProgress progress
});




}
/// @nodoc
class _$OrganizationCleanupEvent_ProgressCopyWithImpl<$Res>
    implements $OrganizationCleanupEvent_ProgressCopyWith<$Res> {
  _$OrganizationCleanupEvent_ProgressCopyWithImpl(this._self, this._then);

  final OrganizationCleanupEvent_Progress _self;
  final $Res Function(OrganizationCleanupEvent_Progress) _then;

/// Create a copy of OrganizationCleanupEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? progress = null,}) {
  return _then(OrganizationCleanupEvent_Progress(
progress: null == progress ? _self.progress : progress // ignore: cast_nullable_to_non_nullable
as OrganizationCleanupProgress,
  ));
}


}

/// @nodoc


class OrganizationCleanupEvent_Inspected extends OrganizationCleanupEvent {
  const OrganizationCleanupEvent_Inspected({required this.inspection}): super._();


 final  OrganizationCleanupInspection inspection;

/// Create a copy of OrganizationCleanupEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationCleanupEvent_InspectedCopyWith<OrganizationCleanupEvent_Inspected> get copyWith => _$OrganizationCleanupEvent_InspectedCopyWithImpl<OrganizationCleanupEvent_Inspected>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationCleanupEvent_Inspected&&(identical(other.inspection, inspection) || other.inspection == inspection));
}


@override
int get hashCode => Object.hash(runtimeType,inspection);

@override
String toString() {
  return 'OrganizationCleanupEvent.inspected(inspection: $inspection)';
}


}

/// @nodoc
abstract mixin class $OrganizationCleanupEvent_InspectedCopyWith<$Res> implements $OrganizationCleanupEventCopyWith<$Res> {
  factory $OrganizationCleanupEvent_InspectedCopyWith(OrganizationCleanupEvent_Inspected value, $Res Function(OrganizationCleanupEvent_Inspected) _then) = _$OrganizationCleanupEvent_InspectedCopyWithImpl;
@useResult
$Res call({
 OrganizationCleanupInspection inspection
});




}
/// @nodoc
class _$OrganizationCleanupEvent_InspectedCopyWithImpl<$Res>
    implements $OrganizationCleanupEvent_InspectedCopyWith<$Res> {
  _$OrganizationCleanupEvent_InspectedCopyWithImpl(this._self, this._then);

  final OrganizationCleanupEvent_Inspected _self;
  final $Res Function(OrganizationCleanupEvent_Inspected) _then;

/// Create a copy of OrganizationCleanupEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? inspection = null,}) {
  return _then(OrganizationCleanupEvent_Inspected(
inspection: null == inspection ? _self.inspection : inspection // ignore: cast_nullable_to_non_nullable
as OrganizationCleanupInspection,
  ));
}


}

/// @nodoc


class OrganizationCleanupEvent_Completed extends OrganizationCleanupEvent {
  const OrganizationCleanupEvent_Completed({required this.receipt}): super._();


 final  OrganizationCleanupReceipt receipt;

/// Create a copy of OrganizationCleanupEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationCleanupEvent_CompletedCopyWith<OrganizationCleanupEvent_Completed> get copyWith => _$OrganizationCleanupEvent_CompletedCopyWithImpl<OrganizationCleanupEvent_Completed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationCleanupEvent_Completed&&(identical(other.receipt, receipt) || other.receipt == receipt));
}


@override
int get hashCode => Object.hash(runtimeType,receipt);

@override
String toString() {
  return 'OrganizationCleanupEvent.completed(receipt: $receipt)';
}


}

/// @nodoc
abstract mixin class $OrganizationCleanupEvent_CompletedCopyWith<$Res> implements $OrganizationCleanupEventCopyWith<$Res> {
  factory $OrganizationCleanupEvent_CompletedCopyWith(OrganizationCleanupEvent_Completed value, $Res Function(OrganizationCleanupEvent_Completed) _then) = _$OrganizationCleanupEvent_CompletedCopyWithImpl;
@useResult
$Res call({
 OrganizationCleanupReceipt receipt
});




}
/// @nodoc
class _$OrganizationCleanupEvent_CompletedCopyWithImpl<$Res>
    implements $OrganizationCleanupEvent_CompletedCopyWith<$Res> {
  _$OrganizationCleanupEvent_CompletedCopyWithImpl(this._self, this._then);

  final OrganizationCleanupEvent_Completed _self;
  final $Res Function(OrganizationCleanupEvent_Completed) _then;

/// Create a copy of OrganizationCleanupEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? receipt = null,}) {
  return _then(OrganizationCleanupEvent_Completed(
receipt: null == receipt ? _self.receipt : receipt // ignore: cast_nullable_to_non_nullable
as OrganizationCleanupReceipt,
  ));
}


}

/// @nodoc


class OrganizationCleanupEvent_Cancelled extends OrganizationCleanupEvent {
  const OrganizationCleanupEvent_Cancelled(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationCleanupEvent_Cancelled);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'OrganizationCleanupEvent.cancelled()';
}


}




/// @nodoc


class OrganizationCleanupEvent_Failed extends OrganizationCleanupEvent {
  const OrganizationCleanupEvent_Failed({required this.code, required this.message}): super._();


 final  String code;
 final  String message;

/// Create a copy of OrganizationCleanupEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OrganizationCleanupEvent_FailedCopyWith<OrganizationCleanupEvent_Failed> get copyWith => _$OrganizationCleanupEvent_FailedCopyWithImpl<OrganizationCleanupEvent_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OrganizationCleanupEvent_Failed&&(identical(other.code, code) || other.code == code)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,code,message);

@override
String toString() {
  return 'OrganizationCleanupEvent.failed(code: $code, message: $message)';
}


}

/// @nodoc
abstract mixin class $OrganizationCleanupEvent_FailedCopyWith<$Res> implements $OrganizationCleanupEventCopyWith<$Res> {
  factory $OrganizationCleanupEvent_FailedCopyWith(OrganizationCleanupEvent_Failed value, $Res Function(OrganizationCleanupEvent_Failed) _then) = _$OrganizationCleanupEvent_FailedCopyWithImpl;
@useResult
$Res call({
 String code, String message
});




}
/// @nodoc
class _$OrganizationCleanupEvent_FailedCopyWithImpl<$Res>
    implements $OrganizationCleanupEvent_FailedCopyWith<$Res> {
  _$OrganizationCleanupEvent_FailedCopyWithImpl(this._self, this._then);

  final OrganizationCleanupEvent_Failed _self;
  final $Res Function(OrganizationCleanupEvent_Failed) _then;

/// Create a copy of OrganizationCleanupEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? code = null,Object? message = null,}) {
  return _then(OrganizationCleanupEvent_Failed(
code: null == code ? _self.code : code // ignore: cast_nullable_to_non_nullable
as String,message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
