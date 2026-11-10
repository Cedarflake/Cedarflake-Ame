// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'file_operation_selection.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$FileOperationSelectionScope {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileOperationSelectionScope);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'FileOperationSelectionScope()';
}


}

/// @nodoc
class $FileOperationSelectionScopeCopyWith<$Res>  {
$FileOperationSelectionScopeCopyWith(FileOperationSelectionScope _, $Res Function(FileOperationSelectionScope) __);
}


/// Adds pattern-matching-related methods to [FileOperationSelectionScope].
extension FileOperationSelectionScopePatterns on FileOperationSelectionScope {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( FileOperationSelectionScope_Gallery value)?  gallery,TResult Function( FileOperationSelectionScope_MergedGalleryGroups value)?  mergedGalleryGroups,TResult Function( FileOperationSelectionScope_Locations value)?  locations,required TResult orElse(),}){
final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery() when gallery != null:
return gallery(_that);case FileOperationSelectionScope_MergedGalleryGroups() when mergedGalleryGroups != null:
return mergedGalleryGroups(_that);case FileOperationSelectionScope_Locations() when locations != null:
return locations(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( FileOperationSelectionScope_Gallery value)  gallery,required TResult Function( FileOperationSelectionScope_MergedGalleryGroups value)  mergedGalleryGroups,required TResult Function( FileOperationSelectionScope_Locations value)  locations,}){
final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery():
return gallery(_that);case FileOperationSelectionScope_MergedGalleryGroups():
return mergedGalleryGroups(_that);case FileOperationSelectionScope_Locations():
return locations(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( FileOperationSelectionScope_Gallery value)?  gallery,TResult? Function( FileOperationSelectionScope_MergedGalleryGroups value)?  mergedGalleryGroups,TResult? Function( FileOperationSelectionScope_Locations value)?  locations,}){
final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery() when gallery != null:
return gallery(_that);case FileOperationSelectionScope_MergedGalleryGroups() when mergedGalleryGroups != null:
return mergedGalleryGroups(_that);case FileOperationSelectionScope_Locations() when locations != null:
return locations(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  gallery,TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  mergedGalleryGroups,TResult Function()?  locations,required TResult orElse(),}) {final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery() when gallery != null:
return gallery(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_MergedGalleryGroups() when mergedGalleryGroups != null:
return mergedGalleryGroups(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_Locations() when locations != null:
return locations();case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)  gallery,required TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)  mergedGalleryGroups,required TResult Function()  locations,}) {final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery():
return gallery(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_MergedGalleryGroups():
return mergedGalleryGroups(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_Locations():
return locations();}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  gallery,TResult? Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  mergedGalleryGroups,TResult? Function()?  locations,}) {final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery() when gallery != null:
return gallery(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_MergedGalleryGroups() when mergedGalleryGroups != null:
return mergedGalleryGroups(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_Locations() when locations != null:
return locations();case _:
  return null;

}
}

}

/// @nodoc


class FileOperationSelectionScope_Gallery extends FileOperationSelectionScope {
  const FileOperationSelectionScope_Gallery({required this.query, required this.catalogRevision, required this.allMatching}): super._();


 final  GalleryQuery query;
 final  BigInt catalogRevision;
 final  bool allMatching;

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FileOperationSelectionScope_GalleryCopyWith<FileOperationSelectionScope_Gallery> get copyWith => _$FileOperationSelectionScope_GalleryCopyWithImpl<FileOperationSelectionScope_Gallery>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileOperationSelectionScope_Gallery&&(identical(other.query, query) || other.query == query)&&(identical(other.catalogRevision, catalogRevision) || other.catalogRevision == catalogRevision)&&(identical(other.allMatching, allMatching) || other.allMatching == allMatching));
}


@override
int get hashCode => Object.hash(runtimeType,query,catalogRevision,allMatching);

@override
String toString() {
  return 'FileOperationSelectionScope.gallery(query: $query, catalogRevision: $catalogRevision, allMatching: $allMatching)';
}


}

/// @nodoc
abstract mixin class $FileOperationSelectionScope_GalleryCopyWith<$Res> implements $FileOperationSelectionScopeCopyWith<$Res> {
  factory $FileOperationSelectionScope_GalleryCopyWith(FileOperationSelectionScope_Gallery value, $Res Function(FileOperationSelectionScope_Gallery) _then) = _$FileOperationSelectionScope_GalleryCopyWithImpl;
@useResult
$Res call({
 GalleryQuery query, BigInt catalogRevision, bool allMatching
});




}
/// @nodoc
class _$FileOperationSelectionScope_GalleryCopyWithImpl<$Res>
    implements $FileOperationSelectionScope_GalleryCopyWith<$Res> {
  _$FileOperationSelectionScope_GalleryCopyWithImpl(this._self, this._then);

  final FileOperationSelectionScope_Gallery _self;
  final $Res Function(FileOperationSelectionScope_Gallery) _then;

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? query = null,Object? catalogRevision = null,Object? allMatching = null,}) {
  return _then(FileOperationSelectionScope_Gallery(
query: null == query ? _self.query : query // ignore: cast_nullable_to_non_nullable
as GalleryQuery,catalogRevision: null == catalogRevision ? _self.catalogRevision : catalogRevision // ignore: cast_nullable_to_non_nullable
as BigInt,allMatching: null == allMatching ? _self.allMatching : allMatching // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class FileOperationSelectionScope_MergedGalleryGroups extends FileOperationSelectionScope {
  const FileOperationSelectionScope_MergedGalleryGroups({required this.query, required this.catalogRevision, required this.allMatching}): super._();


 final  GalleryQuery query;
 final  BigInt catalogRevision;
 final  bool allMatching;

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FileOperationSelectionScope_MergedGalleryGroupsCopyWith<FileOperationSelectionScope_MergedGalleryGroups> get copyWith => _$FileOperationSelectionScope_MergedGalleryGroupsCopyWithImpl<FileOperationSelectionScope_MergedGalleryGroups>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileOperationSelectionScope_MergedGalleryGroups&&(identical(other.query, query) || other.query == query)&&(identical(other.catalogRevision, catalogRevision) || other.catalogRevision == catalogRevision)&&(identical(other.allMatching, allMatching) || other.allMatching == allMatching));
}


@override
int get hashCode => Object.hash(runtimeType,query,catalogRevision,allMatching);

@override
String toString() {
  return 'FileOperationSelectionScope.mergedGalleryGroups(query: $query, catalogRevision: $catalogRevision, allMatching: $allMatching)';
}


}

/// @nodoc
abstract mixin class $FileOperationSelectionScope_MergedGalleryGroupsCopyWith<$Res> implements $FileOperationSelectionScopeCopyWith<$Res> {
  factory $FileOperationSelectionScope_MergedGalleryGroupsCopyWith(FileOperationSelectionScope_MergedGalleryGroups value, $Res Function(FileOperationSelectionScope_MergedGalleryGroups) _then) = _$FileOperationSelectionScope_MergedGalleryGroupsCopyWithImpl;
@useResult
$Res call({
 GalleryQuery query, BigInt catalogRevision, bool allMatching
});




}
/// @nodoc
class _$FileOperationSelectionScope_MergedGalleryGroupsCopyWithImpl<$Res>
    implements $FileOperationSelectionScope_MergedGalleryGroupsCopyWith<$Res> {
  _$FileOperationSelectionScope_MergedGalleryGroupsCopyWithImpl(this._self, this._then);

  final FileOperationSelectionScope_MergedGalleryGroups _self;
  final $Res Function(FileOperationSelectionScope_MergedGalleryGroups) _then;

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? query = null,Object? catalogRevision = null,Object? allMatching = null,}) {
  return _then(FileOperationSelectionScope_MergedGalleryGroups(
query: null == query ? _self.query : query // ignore: cast_nullable_to_non_nullable
as GalleryQuery,catalogRevision: null == catalogRevision ? _self.catalogRevision : catalogRevision // ignore: cast_nullable_to_non_nullable
as BigInt,allMatching: null == allMatching ? _self.allMatching : allMatching // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class FileOperationSelectionScope_Locations extends FileOperationSelectionScope {
  const FileOperationSelectionScope_Locations(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileOperationSelectionScope_Locations);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'FileOperationSelectionScope.locations()';
}


}




// dart format on
