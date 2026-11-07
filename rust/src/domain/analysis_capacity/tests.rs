use super::*;

fn capacity(processors: usize, free_gib: u64) -> AnalysisCapacity {
    AnalysisCapacity {
        processors,
        physical_bytes: 16 * GIB,
        available_bytes: free_gib * GIB,
        commit_headroom_bytes: free_gib * GIB,
    }
}

#[test]
fn overlap_requires_cpu_capacity_and_both_memory_budgets() {
    for (capacity, gpu, expected) in [
        (Some(capacity(4, 4)), true, AnalysisConcurrency::Parallel),
        (Some(capacity(4, 4)), false, AnalysisConcurrency::Serial),
        (Some(capacity(8, 4)), false, AnalysisConcurrency::Parallel),
        (
            Some(AnalysisCapacity {
                physical_bytes: 8 * GIB,
                ..capacity(16, 4)
            }),
            true,
            AnalysisConcurrency::Serial,
        ),
        (
            Some(AnalysisCapacity {
                commit_headroom_bytes: GIB,
                ..capacity(16, 4)
            }),
            true,
            AnalysisConcurrency::Serial,
        ),
        (Some(capacity(16, 2)), true, AnalysisConcurrency::Serial),
        (None, true, AnalysisConcurrency::Serial),
    ] {
        assert_eq!(
            AnalysisCapacityPolicy::default().update(capacity, gpu, false, Duration::ZERO),
            expected
        );
    }
}

#[test]
fn pressure_downgrades_immediately_and_recovery_requires_continuous_headroom() {
    let mut policy = AnalysisCapacityPolicy::default();
    let mut update = |seconds, free, pressure| {
        policy.update(
            Some(capacity(16, free)),
            true,
            pressure,
            Duration::from_secs(seconds),
        )
    };
    assert_eq!(update(0, 4, false), AnalysisConcurrency::Parallel);
    assert_eq!(update(1, 2, false), AnalysisConcurrency::Parallel);
    assert_eq!(update(2, 4, true), AnalysisConcurrency::Serial);
    assert_eq!(update(3, 4, false), AnalysisConcurrency::Serial);
    assert_eq!(update(32, 2, false), AnalysisConcurrency::Serial);
    assert_eq!(update(33, 4, false), AnalysisConcurrency::Serial);
    assert_eq!(update(62, 4, false), AnalysisConcurrency::Serial);
    assert_eq!(update(63, 4, false), AnalysisConcurrency::Parallel);
    assert_eq!(update(64, 1, false), AnalysisConcurrency::Serial);
}
