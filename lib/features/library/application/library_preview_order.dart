enum LibraryPreviewPriority { idle, guard, nearDirection, visible, viewer }

bool isPreviewRequestPreferred({
  required LibraryPreviewPriority priority,
  required int? demandRank,
  required int sequence,
  required LibraryPreviewPriority currentPriority,
  required int? currentDemandRank,
  required int currentSequence,
}) {
  if (priority != currentPriority) {
    return priority.index > currentPriority.index;
  }
  if (demandRank != currentDemandRank) {
    if (demandRank == null) {
      return false;
    }
    if (currentDemandRank == null) {
      return true;
    }
    return demandRank < currentDemandRank;
  }
  return sequence < currentSequence;
}
