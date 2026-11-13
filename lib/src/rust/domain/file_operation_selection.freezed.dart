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
mixin _$DuplicateCleanupGroup {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DuplicateCleanupGroup);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DuplicateCleanupGroup()';
}


}

/// @nodoc
class $DuplicateCleanupGroupCopyWith<$Res>  {
$DuplicateCleanupGroupCopyWith(DuplicateCleanupGroup _, $Res Function(DuplicateCleanupGroup) __);
}


/// Adds pattern-matching-related methods to [DuplicateCleanupGroup].
extension DuplicateCleanupGroupPatterns on DuplicateCleanupGroup {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( DuplicateCleanupGroup_Exact value)?  exact,TResult Function( DuplicateCleanupGroup_Similar value)?  similar,required TResult orElse(),}){
final _that = this;
switch (_that) {
case DuplicateCleanupGroup_Exact() when exact != null:
return exact(_that);case DuplicateCleanupGroup_Similar() when similar != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( DuplicateCleanupGroup_Exact value)  exact,required TResult Function( DuplicateCleanupGroup_Similar value)  similar,}){
final _that = this;
switch (_that) {
case DuplicateCleanupGroup_Exact():
return exact(_that);case DuplicateCleanupGroup_Similar():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( DuplicateCleanupGroup_Exact value)?  exact,TResult? Function( DuplicateCleanupGroup_Similar value)?  similar,}){
final _that = this;
switch (_that) {
case DuplicateCleanupGroup_Exact() when exact != null:
return exact(_that);case DuplicateCleanupGroup_Similar() when similar != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( ExactDuplicateReviewKey key)?  exact,TResult Function( String groupId)?  similar,required TResult orElse(),}) {final _that = this;
switch (_that) {
case DuplicateCleanupGroup_Exact() when exact != null:
return exact(_that.key);case DuplicateCleanupGroup_Similar() when similar != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( ExactDuplicateReviewKey key)  exact,required TResult Function( String groupId)  similar,}) {final _that = this;
switch (_that) {
case DuplicateCleanupGroup_Exact():
return exact(_that.key);case DuplicateCleanupGroup_Similar():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( ExactDuplicateReviewKey key)?  exact,TResult? Function( String groupId)?  similar,}) {final _that = this;
switch (_that) {
case DuplicateCleanupGroup_Exact() when exact != null:
return exact(_that.key);case DuplicateCleanupGroup_Similar() when similar != null:
return similar(_that.groupId);case _:
  return null;

}
}

}

/// @nodoc


class DuplicateCleanupGroup_Exact extends DuplicateCleanupGroup {
  const DuplicateCleanupGroup_Exact({required this.key}): super._();


 final  ExactDuplicateReviewKey key;

/// Create a copy of DuplicateCleanupGroup
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DuplicateCleanupGroup_ExactCopyWith<DuplicateCleanupGroup_Exact> get copyWith => _$DuplicateCleanupGroup_ExactCopyWithImpl<DuplicateCleanupGroup_Exact>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DuplicateCleanupGroup_Exact&&(identical(other.key, key) || other.key == key));
}


@override
int get hashCode => Object.hash(runtimeType,key);

@override
String toString() {
  return 'DuplicateCleanupGroup.exact(key: $key)';
}


}

/// @nodoc
abstract mixin class $DuplicateCleanupGroup_ExactCopyWith<$Res> implements $DuplicateCleanupGroupCopyWith<$Res> {
  factory $DuplicateCleanupGroup_ExactCopyWith(DuplicateCleanupGroup_Exact value, $Res Function(DuplicateCleanupGroup_Exact) _then) = _$DuplicateCleanupGroup_ExactCopyWithImpl;
@useResult
$Res call({
 ExactDuplicateReviewKey key
});




}
/// @nodoc
class _$DuplicateCleanupGroup_ExactCopyWithImpl<$Res>
    implements $DuplicateCleanupGroup_ExactCopyWith<$Res> {
  _$DuplicateCleanupGroup_ExactCopyWithImpl(this._self, this._then);

  final DuplicateCleanupGroup_Exact _self;
  final $Res Function(DuplicateCleanupGroup_Exact) _then;

/// Create a copy of DuplicateCleanupGroup
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? key = null,}) {
  return _then(DuplicateCleanupGroup_Exact(
key: null == key ? _self.key : key // ignore: cast_nullable_to_non_nullable
as ExactDuplicateReviewKey,
  ));
}


}

/// @nodoc


class DuplicateCleanupGroup_Similar extends DuplicateCleanupGroup {
  const DuplicateCleanupGroup_Similar({required this.groupId}): super._();


 final  String groupId;

/// Create a copy of DuplicateCleanupGroup
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DuplicateCleanupGroup_SimilarCopyWith<DuplicateCleanupGroup_Similar> get copyWith => _$DuplicateCleanupGroup_SimilarCopyWithImpl<DuplicateCleanupGroup_Similar>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DuplicateCleanupGroup_Similar&&(identical(other.groupId, groupId) || other.groupId == groupId));
}


@override
int get hashCode => Object.hash(runtimeType,groupId);

@override
String toString() {
  return 'DuplicateCleanupGroup.similar(groupId: $groupId)';
}


}

/// @nodoc
abstract mixin class $DuplicateCleanupGroup_SimilarCopyWith<$Res> implements $DuplicateCleanupGroupCopyWith<$Res> {
  factory $DuplicateCleanupGroup_SimilarCopyWith(DuplicateCleanupGroup_Similar value, $Res Function(DuplicateCleanupGroup_Similar) _then) = _$DuplicateCleanupGroup_SimilarCopyWithImpl;
@useResult
$Res call({
 String groupId
});




}
/// @nodoc
class _$DuplicateCleanupGroup_SimilarCopyWithImpl<$Res>
    implements $DuplicateCleanupGroup_SimilarCopyWith<$Res> {
  _$DuplicateCleanupGroup_SimilarCopyWithImpl(this._self, this._then);

  final DuplicateCleanupGroup_Similar _self;
  final $Res Function(DuplicateCleanupGroup_Similar) _then;

/// Create a copy of DuplicateCleanupGroup
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? groupId = null,}) {
  return _then(DuplicateCleanupGroup_Similar(
groupId: null == groupId ? _self.groupId : groupId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$DuplicateCleanupKind {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DuplicateCleanupKind);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DuplicateCleanupKind()';
}


}

/// @nodoc
class $DuplicateCleanupKindCopyWith<$Res>  {
$DuplicateCleanupKindCopyWith(DuplicateCleanupKind _, $Res Function(DuplicateCleanupKind) __);
}


/// Adds pattern-matching-related methods to [DuplicateCleanupKind].
extension DuplicateCleanupKindPatterns on DuplicateCleanupKind {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( DuplicateCleanupKind_Exact value)?  exact,TResult Function( DuplicateCleanupKind_Similar value)?  similar,required TResult orElse(),}){
final _that = this;
switch (_that) {
case DuplicateCleanupKind_Exact() when exact != null:
return exact(_that);case DuplicateCleanupKind_Similar() when similar != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( DuplicateCleanupKind_Exact value)  exact,required TResult Function( DuplicateCleanupKind_Similar value)  similar,}){
final _that = this;
switch (_that) {
case DuplicateCleanupKind_Exact():
return exact(_that);case DuplicateCleanupKind_Similar():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( DuplicateCleanupKind_Exact value)?  exact,TResult? Function( DuplicateCleanupKind_Similar value)?  similar,}){
final _that = this;
switch (_that) {
case DuplicateCleanupKind_Exact() when exact != null:
return exact(_that);case DuplicateCleanupKind_Similar() when similar != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  exact,TResult Function( SimilarityDegree degree)?  similar,required TResult orElse(),}) {final _that = this;
switch (_that) {
case DuplicateCleanupKind_Exact() when exact != null:
return exact();case DuplicateCleanupKind_Similar() when similar != null:
return similar(_that.degree);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  exact,required TResult Function( SimilarityDegree degree)  similar,}) {final _that = this;
switch (_that) {
case DuplicateCleanupKind_Exact():
return exact();case DuplicateCleanupKind_Similar():
return similar(_that.degree);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  exact,TResult? Function( SimilarityDegree degree)?  similar,}) {final _that = this;
switch (_that) {
case DuplicateCleanupKind_Exact() when exact != null:
return exact();case DuplicateCleanupKind_Similar() when similar != null:
return similar(_that.degree);case _:
  return null;

}
}

}

/// @nodoc


class DuplicateCleanupKind_Exact extends DuplicateCleanupKind {
  const DuplicateCleanupKind_Exact(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DuplicateCleanupKind_Exact);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DuplicateCleanupKind.exact()';
}


}




/// @nodoc


class DuplicateCleanupKind_Similar extends DuplicateCleanupKind {
  const DuplicateCleanupKind_Similar({required this.degree}): super._();


 final  SimilarityDegree degree;

/// Create a copy of DuplicateCleanupKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DuplicateCleanupKind_SimilarCopyWith<DuplicateCleanupKind_Similar> get copyWith => _$DuplicateCleanupKind_SimilarCopyWithImpl<DuplicateCleanupKind_Similar>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DuplicateCleanupKind_Similar&&(identical(other.degree, degree) || other.degree == degree));
}


@override
int get hashCode => Object.hash(runtimeType,degree);

@override
String toString() {
  return 'DuplicateCleanupKind.similar(degree: $degree)';
}


}

/// @nodoc
abstract mixin class $DuplicateCleanupKind_SimilarCopyWith<$Res> implements $DuplicateCleanupKindCopyWith<$Res> {
  factory $DuplicateCleanupKind_SimilarCopyWith(DuplicateCleanupKind_Similar value, $Res Function(DuplicateCleanupKind_Similar) _then) = _$DuplicateCleanupKind_SimilarCopyWithImpl;
@useResult
$Res call({
 SimilarityDegree degree
});




}
/// @nodoc
class _$DuplicateCleanupKind_SimilarCopyWithImpl<$Res>
    implements $DuplicateCleanupKind_SimilarCopyWith<$Res> {
  _$DuplicateCleanupKind_SimilarCopyWithImpl(this._self, this._then);

  final DuplicateCleanupKind_Similar _self;
  final $Res Function(DuplicateCleanupKind_Similar) _then;

/// Create a copy of DuplicateCleanupKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? degree = null,}) {
  return _then(DuplicateCleanupKind_Similar(
degree: null == degree ? _self.degree : degree // ignore: cast_nullable_to_non_nullable
as SimilarityDegree,
  ));
}


}

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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( FileOperationSelectionScope_Gallery value)?  gallery,TResult Function( FileOperationSelectionScope_MergedGalleryGroups value)?  mergedGalleryGroups,TResult Function( FileOperationSelectionScope_Locations value)?  locations,TResult Function( FileOperationSelectionScope_DuplicateCleanup value)?  duplicateCleanup,TResult Function( FileOperationSelectionScope_GalleryCleanup value)?  galleryCleanup,required TResult orElse(),}){
final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery() when gallery != null:
return gallery(_that);case FileOperationSelectionScope_MergedGalleryGroups() when mergedGalleryGroups != null:
return mergedGalleryGroups(_that);case FileOperationSelectionScope_Locations() when locations != null:
return locations(_that);case FileOperationSelectionScope_DuplicateCleanup() when duplicateCleanup != null:
return duplicateCleanup(_that);case FileOperationSelectionScope_GalleryCleanup() when galleryCleanup != null:
return galleryCleanup(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( FileOperationSelectionScope_Gallery value)  gallery,required TResult Function( FileOperationSelectionScope_MergedGalleryGroups value)  mergedGalleryGroups,required TResult Function( FileOperationSelectionScope_Locations value)  locations,required TResult Function( FileOperationSelectionScope_DuplicateCleanup value)  duplicateCleanup,required TResult Function( FileOperationSelectionScope_GalleryCleanup value)  galleryCleanup,}){
final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery():
return gallery(_that);case FileOperationSelectionScope_MergedGalleryGroups():
return mergedGalleryGroups(_that);case FileOperationSelectionScope_Locations():
return locations(_that);case FileOperationSelectionScope_DuplicateCleanup():
return duplicateCleanup(_that);case FileOperationSelectionScope_GalleryCleanup():
return galleryCleanup(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( FileOperationSelectionScope_Gallery value)?  gallery,TResult? Function( FileOperationSelectionScope_MergedGalleryGroups value)?  mergedGalleryGroups,TResult? Function( FileOperationSelectionScope_Locations value)?  locations,TResult? Function( FileOperationSelectionScope_DuplicateCleanup value)?  duplicateCleanup,TResult? Function( FileOperationSelectionScope_GalleryCleanup value)?  galleryCleanup,}){
final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery() when gallery != null:
return gallery(_that);case FileOperationSelectionScope_MergedGalleryGroups() when mergedGalleryGroups != null:
return mergedGalleryGroups(_that);case FileOperationSelectionScope_Locations() when locations != null:
return locations(_that);case FileOperationSelectionScope_DuplicateCleanup() when duplicateCleanup != null:
return duplicateCleanup(_that);case FileOperationSelectionScope_GalleryCleanup() when galleryCleanup != null:
return galleryCleanup(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  gallery,TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  mergedGalleryGroups,TResult Function()?  locations,TResult Function( DuplicateCleanupKind kind,  bool allGroups)?  duplicateCleanup,TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  galleryCleanup,required TResult orElse(),}) {final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery() when gallery != null:
return gallery(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_MergedGalleryGroups() when mergedGalleryGroups != null:
return mergedGalleryGroups(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_Locations() when locations != null:
return locations();case FileOperationSelectionScope_DuplicateCleanup() when duplicateCleanup != null:
return duplicateCleanup(_that.kind,_that.allGroups);case FileOperationSelectionScope_GalleryCleanup() when galleryCleanup != null:
return galleryCleanup(_that.query,_that.catalogRevision,_that.allMatching);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)  gallery,required TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)  mergedGalleryGroups,required TResult Function()  locations,required TResult Function( DuplicateCleanupKind kind,  bool allGroups)  duplicateCleanup,required TResult Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)  galleryCleanup,}) {final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery():
return gallery(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_MergedGalleryGroups():
return mergedGalleryGroups(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_Locations():
return locations();case FileOperationSelectionScope_DuplicateCleanup():
return duplicateCleanup(_that.kind,_that.allGroups);case FileOperationSelectionScope_GalleryCleanup():
return galleryCleanup(_that.query,_that.catalogRevision,_that.allMatching);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  gallery,TResult? Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  mergedGalleryGroups,TResult? Function()?  locations,TResult? Function( DuplicateCleanupKind kind,  bool allGroups)?  duplicateCleanup,TResult? Function( GalleryQuery query,  BigInt catalogRevision,  bool allMatching)?  galleryCleanup,}) {final _that = this;
switch (_that) {
case FileOperationSelectionScope_Gallery() when gallery != null:
return gallery(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_MergedGalleryGroups() when mergedGalleryGroups != null:
return mergedGalleryGroups(_that.query,_that.catalogRevision,_that.allMatching);case FileOperationSelectionScope_Locations() when locations != null:
return locations();case FileOperationSelectionScope_DuplicateCleanup() when duplicateCleanup != null:
return duplicateCleanup(_that.kind,_that.allGroups);case FileOperationSelectionScope_GalleryCleanup() when galleryCleanup != null:
return galleryCleanup(_that.query,_that.catalogRevision,_that.allMatching);case _:
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




/// @nodoc


class FileOperationSelectionScope_DuplicateCleanup extends FileOperationSelectionScope {
  const FileOperationSelectionScope_DuplicateCleanup({required this.kind, required this.allGroups}): super._();


 final  DuplicateCleanupKind kind;
 final  bool allGroups;

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FileOperationSelectionScope_DuplicateCleanupCopyWith<FileOperationSelectionScope_DuplicateCleanup> get copyWith => _$FileOperationSelectionScope_DuplicateCleanupCopyWithImpl<FileOperationSelectionScope_DuplicateCleanup>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileOperationSelectionScope_DuplicateCleanup&&(identical(other.kind, kind) || other.kind == kind)&&(identical(other.allGroups, allGroups) || other.allGroups == allGroups));
}


@override
int get hashCode => Object.hash(runtimeType,kind,allGroups);

@override
String toString() {
  return 'FileOperationSelectionScope.duplicateCleanup(kind: $kind, allGroups: $allGroups)';
}


}

/// @nodoc
abstract mixin class $FileOperationSelectionScope_DuplicateCleanupCopyWith<$Res> implements $FileOperationSelectionScopeCopyWith<$Res> {
  factory $FileOperationSelectionScope_DuplicateCleanupCopyWith(FileOperationSelectionScope_DuplicateCleanup value, $Res Function(FileOperationSelectionScope_DuplicateCleanup) _then) = _$FileOperationSelectionScope_DuplicateCleanupCopyWithImpl;
@useResult
$Res call({
 DuplicateCleanupKind kind, bool allGroups
});


$DuplicateCleanupKindCopyWith<$Res> get kind;

}
/// @nodoc
class _$FileOperationSelectionScope_DuplicateCleanupCopyWithImpl<$Res>
    implements $FileOperationSelectionScope_DuplicateCleanupCopyWith<$Res> {
  _$FileOperationSelectionScope_DuplicateCleanupCopyWithImpl(this._self, this._then);

  final FileOperationSelectionScope_DuplicateCleanup _self;
  final $Res Function(FileOperationSelectionScope_DuplicateCleanup) _then;

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? kind = null,Object? allGroups = null,}) {
  return _then(FileOperationSelectionScope_DuplicateCleanup(
kind: null == kind ? _self.kind : kind // ignore: cast_nullable_to_non_nullable
as DuplicateCleanupKind,allGroups: null == allGroups ? _self.allGroups : allGroups // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$DuplicateCleanupKindCopyWith<$Res> get kind {

  return $DuplicateCleanupKindCopyWith<$Res>(_self.kind, (value) {
    return _then(_self.copyWith(kind: value));
  });
}
}

/// @nodoc


class FileOperationSelectionScope_GalleryCleanup extends FileOperationSelectionScope {
  const FileOperationSelectionScope_GalleryCleanup({required this.query, required this.catalogRevision, required this.allMatching}): super._();


 final  GalleryQuery query;
 final  BigInt catalogRevision;
 final  bool allMatching;

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FileOperationSelectionScope_GalleryCleanupCopyWith<FileOperationSelectionScope_GalleryCleanup> get copyWith => _$FileOperationSelectionScope_GalleryCleanupCopyWithImpl<FileOperationSelectionScope_GalleryCleanup>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileOperationSelectionScope_GalleryCleanup&&(identical(other.query, query) || other.query == query)&&(identical(other.catalogRevision, catalogRevision) || other.catalogRevision == catalogRevision)&&(identical(other.allMatching, allMatching) || other.allMatching == allMatching));
}


@override
int get hashCode => Object.hash(runtimeType,query,catalogRevision,allMatching);

@override
String toString() {
  return 'FileOperationSelectionScope.galleryCleanup(query: $query, catalogRevision: $catalogRevision, allMatching: $allMatching)';
}


}

/// @nodoc
abstract mixin class $FileOperationSelectionScope_GalleryCleanupCopyWith<$Res> implements $FileOperationSelectionScopeCopyWith<$Res> {
  factory $FileOperationSelectionScope_GalleryCleanupCopyWith(FileOperationSelectionScope_GalleryCleanup value, $Res Function(FileOperationSelectionScope_GalleryCleanup) _then) = _$FileOperationSelectionScope_GalleryCleanupCopyWithImpl;
@useResult
$Res call({
 GalleryQuery query, BigInt catalogRevision, bool allMatching
});




}
/// @nodoc
class _$FileOperationSelectionScope_GalleryCleanupCopyWithImpl<$Res>
    implements $FileOperationSelectionScope_GalleryCleanupCopyWith<$Res> {
  _$FileOperationSelectionScope_GalleryCleanupCopyWithImpl(this._self, this._then);

  final FileOperationSelectionScope_GalleryCleanup _self;
  final $Res Function(FileOperationSelectionScope_GalleryCleanup) _then;

/// Create a copy of FileOperationSelectionScope
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? query = null,Object? catalogRevision = null,Object? allMatching = null,}) {
  return _then(FileOperationSelectionScope_GalleryCleanup(
query: null == query ? _self.query : query // ignore: cast_nullable_to_non_nullable
as GalleryQuery,catalogRevision: null == catalogRevision ? _self.catalogRevision : catalogRevision // ignore: cast_nullable_to_non_nullable
as BigInt,allMatching: null == allMatching ? _self.allMatching : allMatching // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

// dart format on
