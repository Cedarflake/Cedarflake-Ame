// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'file_operation_exports.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$FileExportReport {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileExportReport);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'FileExportReport()';
}


}

/// @nodoc
class $FileExportReportCopyWith<$Res>  {
$FileExportReportCopyWith(FileExportReport _, $Res Function(FileExportReport) __);
}


/// Adds pattern-matching-related methods to [FileExportReport].
extension FileExportReportPatterns on FileExportReport {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( FileExportReport_Pending value)?  pending,TResult Function( FileExportReport_NotNeeded value)?  notNeeded,TResult Function( FileExportReport_Written value)?  written,TResult Function( FileExportReport_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case FileExportReport_Pending() when pending != null:
return pending(_that);case FileExportReport_NotNeeded() when notNeeded != null:
return notNeeded(_that);case FileExportReport_Written() when written != null:
return written(_that);case FileExportReport_Failed() when failed != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( FileExportReport_Pending value)  pending,required TResult Function( FileExportReport_NotNeeded value)  notNeeded,required TResult Function( FileExportReport_Written value)  written,required TResult Function( FileExportReport_Failed value)  failed,}){
final _that = this;
switch (_that) {
case FileExportReport_Pending():
return pending(_that);case FileExportReport_NotNeeded():
return notNeeded(_that);case FileExportReport_Written():
return written(_that);case FileExportReport_Failed():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( FileExportReport_Pending value)?  pending,TResult? Function( FileExportReport_NotNeeded value)?  notNeeded,TResult? Function( FileExportReport_Written value)?  written,TResult? Function( FileExportReport_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case FileExportReport_Pending() when pending != null:
return pending(_that);case FileExportReport_NotNeeded() when notNeeded != null:
return notNeeded(_that);case FileExportReport_Written() when written != null:
return written(_that);case FileExportReport_Failed() when failed != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  pending,TResult Function()?  notNeeded,TResult Function( String field0)?  written,TResult Function( ScanError field0)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case FileExportReport_Pending() when pending != null:
return pending();case FileExportReport_NotNeeded() when notNeeded != null:
return notNeeded();case FileExportReport_Written() when written != null:
return written(_that.field0);case FileExportReport_Failed() when failed != null:
return failed(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  pending,required TResult Function()  notNeeded,required TResult Function( String field0)  written,required TResult Function( ScanError field0)  failed,}) {final _that = this;
switch (_that) {
case FileExportReport_Pending():
return pending();case FileExportReport_NotNeeded():
return notNeeded();case FileExportReport_Written():
return written(_that.field0);case FileExportReport_Failed():
return failed(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  pending,TResult? Function()?  notNeeded,TResult? Function( String field0)?  written,TResult? Function( ScanError field0)?  failed,}) {final _that = this;
switch (_that) {
case FileExportReport_Pending() when pending != null:
return pending();case FileExportReport_NotNeeded() when notNeeded != null:
return notNeeded();case FileExportReport_Written() when written != null:
return written(_that.field0);case FileExportReport_Failed() when failed != null:
return failed(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class FileExportReport_Pending extends FileExportReport {
  const FileExportReport_Pending(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileExportReport_Pending);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'FileExportReport.pending()';
}


}




/// @nodoc


class FileExportReport_NotNeeded extends FileExportReport {
  const FileExportReport_NotNeeded(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileExportReport_NotNeeded);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'FileExportReport.notNeeded()';
}


}




/// @nodoc


class FileExportReport_Written extends FileExportReport {
  const FileExportReport_Written(this.field0): super._();


 final  String field0;

/// Create a copy of FileExportReport
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FileExportReport_WrittenCopyWith<FileExportReport_Written> get copyWith => _$FileExportReport_WrittenCopyWithImpl<FileExportReport_Written>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileExportReport_Written&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'FileExportReport.written(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $FileExportReport_WrittenCopyWith<$Res> implements $FileExportReportCopyWith<$Res> {
  factory $FileExportReport_WrittenCopyWith(FileExportReport_Written value, $Res Function(FileExportReport_Written) _then) = _$FileExportReport_WrittenCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$FileExportReport_WrittenCopyWithImpl<$Res>
    implements $FileExportReport_WrittenCopyWith<$Res> {
  _$FileExportReport_WrittenCopyWithImpl(this._self, this._then);

  final FileExportReport_Written _self;
  final $Res Function(FileExportReport_Written) _then;

/// Create a copy of FileExportReport
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(FileExportReport_Written(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class FileExportReport_Failed extends FileExportReport {
  const FileExportReport_Failed(this.field0): super._();


 final  ScanError field0;

/// Create a copy of FileExportReport
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FileExportReport_FailedCopyWith<FileExportReport_Failed> get copyWith => _$FileExportReport_FailedCopyWithImpl<FileExportReport_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FileExportReport_Failed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'FileExportReport.failed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $FileExportReport_FailedCopyWith<$Res> implements $FileExportReportCopyWith<$Res> {
  factory $FileExportReport_FailedCopyWith(FileExportReport_Failed value, $Res Function(FileExportReport_Failed) _then) = _$FileExportReport_FailedCopyWithImpl;
@useResult
$Res call({
 ScanError field0
});




}
/// @nodoc
class _$FileExportReport_FailedCopyWithImpl<$Res>
    implements $FileExportReport_FailedCopyWith<$Res> {
  _$FileExportReport_FailedCopyWithImpl(this._self, this._then);

  final FileExportReport_Failed _self;
  final $Res Function(FileExportReport_Failed) _then;

/// Create a copy of FileExportReport
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(FileExportReport_Failed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ScanError,
  ));
}


}

// dart format on
