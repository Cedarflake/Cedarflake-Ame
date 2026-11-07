// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'semantic_results.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$SemanticSearchInput {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SemanticSearchInput);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SemanticSearchInput()';
}


}

/// @nodoc
class $SemanticSearchInputCopyWith<$Res>  {
$SemanticSearchInputCopyWith(SemanticSearchInput _, $Res Function(SemanticSearchInput) __);
}


/// Adds pattern-matching-related methods to [SemanticSearchInput].
extension SemanticSearchInputPatterns on SemanticSearchInput {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SemanticSearchInput_Text value)?  text,TResult Function( SemanticSearchInput_Image value)?  image,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SemanticSearchInput_Text() when text != null:
return text(_that);case SemanticSearchInput_Image() when image != null:
return image(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SemanticSearchInput_Text value)  text,required TResult Function( SemanticSearchInput_Image value)  image,}){
final _that = this;
switch (_that) {
case SemanticSearchInput_Text():
return text(_that);case SemanticSearchInput_Image():
return image(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SemanticSearchInput_Text value)?  text,TResult? Function( SemanticSearchInput_Image value)?  image,}){
final _that = this;
switch (_that) {
case SemanticSearchInput_Text() when text != null:
return text(_that);case SemanticSearchInput_Image() when image != null:
return image(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String description)?  text,TResult Function( OrganizationSelection source)?  image,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SemanticSearchInput_Text() when text != null:
return text(_that.description);case SemanticSearchInput_Image() when image != null:
return image(_that.source);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String description)  text,required TResult Function( OrganizationSelection source)  image,}) {final _that = this;
switch (_that) {
case SemanticSearchInput_Text():
return text(_that.description);case SemanticSearchInput_Image():
return image(_that.source);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String description)?  text,TResult? Function( OrganizationSelection source)?  image,}) {final _that = this;
switch (_that) {
case SemanticSearchInput_Text() when text != null:
return text(_that.description);case SemanticSearchInput_Image() when image != null:
return image(_that.source);case _:
  return null;

}
}

}

/// @nodoc


class SemanticSearchInput_Text extends SemanticSearchInput {
  const SemanticSearchInput_Text({required this.description}): super._();


 final  String description;

/// Create a copy of SemanticSearchInput
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SemanticSearchInput_TextCopyWith<SemanticSearchInput_Text> get copyWith => _$SemanticSearchInput_TextCopyWithImpl<SemanticSearchInput_Text>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SemanticSearchInput_Text&&(identical(other.description, description) || other.description == description));
}


@override
int get hashCode => Object.hash(runtimeType,description);

@override
String toString() {
  return 'SemanticSearchInput.text(description: $description)';
}


}

/// @nodoc
abstract mixin class $SemanticSearchInput_TextCopyWith<$Res> implements $SemanticSearchInputCopyWith<$Res> {
  factory $SemanticSearchInput_TextCopyWith(SemanticSearchInput_Text value, $Res Function(SemanticSearchInput_Text) _then) = _$SemanticSearchInput_TextCopyWithImpl;
@useResult
$Res call({
 String description
});




}
/// @nodoc
class _$SemanticSearchInput_TextCopyWithImpl<$Res>
    implements $SemanticSearchInput_TextCopyWith<$Res> {
  _$SemanticSearchInput_TextCopyWithImpl(this._self, this._then);

  final SemanticSearchInput_Text _self;
  final $Res Function(SemanticSearchInput_Text) _then;

/// Create a copy of SemanticSearchInput
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? description = null,}) {
  return _then(SemanticSearchInput_Text(
description: null == description ? _self.description : description // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SemanticSearchInput_Image extends SemanticSearchInput {
  const SemanticSearchInput_Image({required this.source}): super._();


 final  OrganizationSelection source;

/// Create a copy of SemanticSearchInput
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SemanticSearchInput_ImageCopyWith<SemanticSearchInput_Image> get copyWith => _$SemanticSearchInput_ImageCopyWithImpl<SemanticSearchInput_Image>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SemanticSearchInput_Image&&(identical(other.source, source) || other.source == source));
}


@override
int get hashCode => Object.hash(runtimeType,source);

@override
String toString() {
  return 'SemanticSearchInput.image(source: $source)';
}


}

/// @nodoc
abstract mixin class $SemanticSearchInput_ImageCopyWith<$Res> implements $SemanticSearchInputCopyWith<$Res> {
  factory $SemanticSearchInput_ImageCopyWith(SemanticSearchInput_Image value, $Res Function(SemanticSearchInput_Image) _then) = _$SemanticSearchInput_ImageCopyWithImpl;
@useResult
$Res call({
 OrganizationSelection source
});




}
/// @nodoc
class _$SemanticSearchInput_ImageCopyWithImpl<$Res>
    implements $SemanticSearchInput_ImageCopyWith<$Res> {
  _$SemanticSearchInput_ImageCopyWithImpl(this._self, this._then);

  final SemanticSearchInput_Image _self;
  final $Res Function(SemanticSearchInput_Image) _then;

/// Create a copy of SemanticSearchInput
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? source = null,}) {
  return _then(SemanticSearchInput_Image(
source: null == source ? _self.source : source // ignore: cast_nullable_to_non_nullable
as OrganizationSelection,
  ));
}


}

// dart format on
