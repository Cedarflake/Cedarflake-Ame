// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'classification_acceleration.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ClassificationComputeDevice {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ClassificationComputeDevice);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ClassificationComputeDevice()';
}


}

/// @nodoc
class $ClassificationComputeDeviceCopyWith<$Res>  {
$ClassificationComputeDeviceCopyWith(ClassificationComputeDevice _, $Res Function(ClassificationComputeDevice) __);
}


/// Adds pattern-matching-related methods to [ClassificationComputeDevice].
extension ClassificationComputeDevicePatterns on ClassificationComputeDevice {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ClassificationComputeDevice_Cpu value)?  cpu,TResult Function( ClassificationComputeDevice_Gpu value)?  gpu,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ClassificationComputeDevice_Cpu() when cpu != null:
return cpu(_that);case ClassificationComputeDevice_Gpu() when gpu != null:
return gpu(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ClassificationComputeDevice_Cpu value)  cpu,required TResult Function( ClassificationComputeDevice_Gpu value)  gpu,}){
final _that = this;
switch (_that) {
case ClassificationComputeDevice_Cpu():
return cpu(_that);case ClassificationComputeDevice_Gpu():
return gpu(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ClassificationComputeDevice_Cpu value)?  cpu,TResult? Function( ClassificationComputeDevice_Gpu value)?  gpu,}){
final _that = this;
switch (_that) {
case ClassificationComputeDevice_Cpu() when cpu != null:
return cpu(_that);case ClassificationComputeDevice_Gpu() when gpu != null:
return gpu(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  cpu,TResult Function( String name)?  gpu,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ClassificationComputeDevice_Cpu() when cpu != null:
return cpu();case ClassificationComputeDevice_Gpu() when gpu != null:
return gpu(_that.name);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  cpu,required TResult Function( String name)  gpu,}) {final _that = this;
switch (_that) {
case ClassificationComputeDevice_Cpu():
return cpu();case ClassificationComputeDevice_Gpu():
return gpu(_that.name);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  cpu,TResult? Function( String name)?  gpu,}) {final _that = this;
switch (_that) {
case ClassificationComputeDevice_Cpu() when cpu != null:
return cpu();case ClassificationComputeDevice_Gpu() when gpu != null:
return gpu(_that.name);case _:
  return null;

}
}

}

/// @nodoc


class ClassificationComputeDevice_Cpu extends ClassificationComputeDevice {
  const ClassificationComputeDevice_Cpu(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ClassificationComputeDevice_Cpu);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ClassificationComputeDevice.cpu()';
}


}




/// @nodoc


class ClassificationComputeDevice_Gpu extends ClassificationComputeDevice {
  const ClassificationComputeDevice_Gpu({required this.name}): super._();


 final  String name;

/// Create a copy of ClassificationComputeDevice
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ClassificationComputeDevice_GpuCopyWith<ClassificationComputeDevice_Gpu> get copyWith => _$ClassificationComputeDevice_GpuCopyWithImpl<ClassificationComputeDevice_Gpu>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ClassificationComputeDevice_Gpu&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode => Object.hash(runtimeType,name);

@override
String toString() {
  return 'ClassificationComputeDevice.gpu(name: $name)';
}


}

/// @nodoc
abstract mixin class $ClassificationComputeDevice_GpuCopyWith<$Res> implements $ClassificationComputeDeviceCopyWith<$Res> {
  factory $ClassificationComputeDevice_GpuCopyWith(ClassificationComputeDevice_Gpu value, $Res Function(ClassificationComputeDevice_Gpu) _then) = _$ClassificationComputeDevice_GpuCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$ClassificationComputeDevice_GpuCopyWithImpl<$Res>
    implements $ClassificationComputeDevice_GpuCopyWith<$Res> {
  _$ClassificationComputeDevice_GpuCopyWithImpl(this._self, this._then);

  final ClassificationComputeDevice_Gpu _self;
  final $Res Function(ClassificationComputeDevice_Gpu) _then;

/// Create a copy of ClassificationComputeDevice
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(ClassificationComputeDevice_Gpu(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
