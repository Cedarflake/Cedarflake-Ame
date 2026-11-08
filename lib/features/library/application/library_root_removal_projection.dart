import "../domain/library_models.dart";
import "../domain/library_state.dart";

LibraryState projectCommittedRootRemoval(
  LibraryState state,
  LibraryRoot removedRoot,
) {
  final roots = List<LibraryRoot>.unmodifiable(
    state.roots.where((root) => root.id != removedRoot.id),
  );
  final retainsScope =
      state.query.rootId != null && state.query.rootId != removedRoot.id;
  final assets = retainsScope
      ? state.assets
      : List<LibraryAsset>.unmodifiable(
          state.assets.where((asset) => asset.rootId != removedRoot.id),
        );
  final query = state.query.rootId == removedRoot.id
      ? LibraryGalleryQuery(
          collection: state.query.collection,
          organizationQueue: state.query.organizationQueue,
        )
      : state.query;
  return state.copyWith(
    rootPath: state.rootPath == removedRoot.path ? null : state.rootPath,
    displayRootPath: state.displayRootPath == removedRoot.displayPath
        ? null
        : state.displayRootPath,
    isRemovalCommitted: true,
    roots: roots,
    assets: assets,
    query: query,
    queryId: retainsScope ? state.queryId : "",
    windowStartItemOffset: retainsScope ? state.windowStartItemOffset : 0,
    previousCursor: null,
    nextCursor: null,
    timeline: retainsScope ? state.timeline : null,
    layoutManifest: retainsScope ? state.layoutManifest : null,
    activeTimeAnchor: retainsScope ? state.activeTimeAnchor : null,
    queryAnchorResolution: retainsScope ? state.queryAnchorResolution : null,
    isLoadingPage: false,
    isLoadingPreviousPage: false,
    isLoadingTimeline: !retainsScope && roots.isNotEmpty,
    isLoadingTimeAnchor: false,
    isLoadingVisibleRange: false,
    pageErrorMessage: null,
    previousPageErrorMessage: null,
    timeNavigationErrorMessage: null,
    errorMessage: null,
  );
}
