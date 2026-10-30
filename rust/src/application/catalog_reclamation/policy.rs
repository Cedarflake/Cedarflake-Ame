use crate::domain::{CatalogAutoVacuumMode, CatalogSpaceUsage};

const INCREMENTAL_START_BYTES: u64 = 16 * 1024 * 1024;
const INCREMENTAL_START_PERCENT: u64 = 20;
const CONVERSION_START_BYTES: u64 = 64 * 1024 * 1024;
const CONVERSION_START_PERCENT: u64 = 25;
const TARGET_BYTES: u64 = 8 * 1024 * 1024;
const TARGET_PERCENT: u64 = 5;

pub(super) fn should_start_reclamation(usage: CatalogSpaceUsage) -> bool {
    let (minimum_bytes, minimum_percent) = match usage.auto_vacuum {
        CatalogAutoVacuumMode::Incremental => (INCREMENTAL_START_BYTES, INCREMENTAL_START_PERCENT),
        CatalogAutoVacuumMode::None | CatalogAutoVacuumMode::Full => {
            (CONVERSION_START_BYTES, CONVERSION_START_PERCENT)
        }
    };
    usage.reclaimable_bytes() >= minimum_bytes
        && usage.page_count != 0
        && u128::from(usage.freelist_count) * 100
            >= u128::from(usage.page_count) * u128::from(minimum_percent)
}

pub(super) fn reclamation_target_reached(usage: CatalogSpaceUsage) -> bool {
    usage.freelist_count == 0
        || usage.reclaimable_bytes() <= TARGET_BYTES
        || (usage.page_count != 0
            && u128::from(usage.freelist_count) * 100
                <= u128::from(usage.page_count) * u128::from(TARGET_PERCENT))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(page_count: u64, free_pages: u64, mode: CatalogAutoVacuumMode) -> CatalogSpaceUsage {
        CatalogSpaceUsage {
            page_size: 4096,
            page_count,
            freelist_count: free_pages,
            auto_vacuum: mode,
        }
    }

    #[test]
    fn incremental_mode_requires_both_free_bytes_and_fraction_before_reclaiming() {
        let incremental = CatalogAutoVacuumMode::Incremental;
        assert!(should_start_reclamation(usage(20_480, 4096, incremental)));
        assert!(!should_start_reclamation(usage(20_481, 4096, incremental)));
        assert!(!should_start_reclamation(usage(10_000, 4095, incremental)));
        assert!(!should_start_reclamation(usage(0, 0, incremental)));
        assert!(should_start_reclamation(usage(16_964, 4143, incremental)));
    }

    #[test]
    fn legacy_rewrite_keeps_the_higher_conversion_threshold() {
        for mode in [CatalogAutoVacuumMode::None, CatalogAutoVacuumMode::Full] {
            assert!(!should_start_reclamation(usage(16_964, 4143, mode)));
            assert!(!should_start_reclamation(usage(65_536, 16_383, mode)));
            assert!(!should_start_reclamation(usage(65_537, 16_384, mode)));
            assert!(should_start_reclamation(usage(65_536, 16_384, mode)));
        }
    }

    #[test]
    fn low_watermark_leaves_reusable_pages_without_restarting() {
        let current = usage(15_000, 2048, CatalogAutoVacuumMode::Incremental);
        assert!(reclamation_target_reached(current));
        assert!(!should_start_reclamation(current));
        assert!(!reclamation_target_reached(usage(
            15_000,
            2049,
            CatalogAutoVacuumMode::Incremental,
        )));
    }
}
