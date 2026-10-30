pub use crate::adapters::SqliteClassificationFeatureCache;
#[cfg(all(windows, feature = "classification-worker"))]
pub use crate::application::classification_analysis::analyze_classification_content;
pub use crate::application::classification_decisions::plan_classification_change;
pub use crate::application::classification_dispatch::{
    ClassificationDispatchClock, ClassificationDispatchSignals, ClassificationDispatchStep,
    ClassificationDispatcher, ClassificationForegroundState, MonotonicDispatchClock,
};
pub use crate::application::classification_features::prepare_classification_matcher;
pub use crate::application::classification_matching::{
    CategoryMatcher, CategoryScore, RankedPrediction,
};
pub use crate::application::classification_residency::{
    ClassificationEncoderResidency, MonotonicResidencyClock, ResidencyClock,
};
#[cfg(all(windows, feature = "classification-worker"))]
pub use crate::application::classification_runtime::{
    ClassificationRuntimeControl, ClassificationRuntimePaths, NativeClassificationRuntime,
};
pub use crate::application::classification_screening::{
    ReviewRecommendation, ScreeningClassifier, ScreeningResult,
};
pub use crate::application::classification_verification::verify_classification_source;
pub use crate::domain::classification::work::{
    ClassificationAnalysisResult, ClassificationBacklogPage, ClassificationDispatchCommand,
    ClassificationDispatchMode, ClassificationDispatchState, ClassificationRecoveryCursor,
    ClassificationRecoveryPage, ClassificationRecoveryRoot, ClassificationVerificationResult,
    ClassificationWorkFailure, ClassificationWorkIssue, ClassificationWorkLease,
    ClassificationWorkPriority, ClassificationWorkTarget,
};
pub use crate::domain::classification::{
    CategoryDefinition, CategoryId, CategoryScreeningThresholds, ClassificationAction,
    ClassificationAnalysisSetup, ClassificationChange, ClassificationContent, ClassificationError,
    ClassificationFieldAuthority, ClassificationObservation, ClassificationProposal,
    ClassificationRecordId, ClassificationReview, ClassificationRevision, ClassificationRun,
    ClassificationSetupActivation, ClassificationSubject, ClassificationTarget,
    EffectiveClassification, Embedding, FeatureIdentity, HumanCategoryChoice, HumanClassification,
    HumanClassificationConsensus, HumanDecisionOrigin, ImageFeature, ModelCompatibility,
    PredictionEvidence, PredictionReference, ProvisionalClassificationContent, RefinementIdentity,
    ScreeningIdentity, ScreeningPolicy, ScreeningProfile, ScreeningThresholds, Taxonomy,
    TextFeature,
};
pub use crate::domain::classification_catalog::{
    ClassificationCatalogState, ClassificationWriteReceipt, PreparedClassificationSelection,
};
pub use crate::domain::classification_review::{
    ClassificationReviewBookmark, ClassificationReviewFilter, ClassificationReviewPosition,
    ClassificationReviewReceipt, ClassificationReviewScope, ClassificationReviewSession,
};
pub use crate::ports::classification::ClassificationRepository;
pub use crate::ports::classification_availability::ClassificationAvailabilityRepository;
pub use crate::ports::classification_encoder::{
    ClassificationEncoder, ClassificationEncoderFactory, ClassificationEncoderKind,
};
pub use crate::ports::classification_execution::{
    ClassificationExecutionError, ClassificationExecutionOutcome, ClassificationWorkExecutor,
};
pub use crate::ports::classification_features::{
    ClassificationFeatureCacheStats, ClassificationFeatureRepository,
};
pub use crate::ports::classification_review::ClassificationReviewRepository;
pub use crate::ports::classification_setup::ClassificationSetupRepository;
pub use crate::ports::classification_work::ClassificationWorkRepository;
