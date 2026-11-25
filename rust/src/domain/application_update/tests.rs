use super::*;

fn release(version: &str, build: u64) -> ApplicationRelease {
    ApplicationRelease {
        version: version.to_owned(),
        build,
        download_bytes: 1024,
        notes: String::new(),
    }
}

#[test]
fn channel_changes_do_not_downgrade_or_offer_previews_to_stable() {
    for (current, target, channel, expected) in [
        (
            "0.1.0-beta.2",
            "0.1.0-beta.10",
            ApplicationUpdateChannel::Preview,
            true,
        ),
        (
            "0.1.0-beta.2",
            "0.1.0-beta.10",
            ApplicationUpdateChannel::Stable,
            false,
        ),
        (
            "0.1.0-beta.2",
            "0.1.0",
            ApplicationUpdateChannel::Stable,
            true,
        ),
        (
            "0.1.0",
            "0.1.0-beta.10",
            ApplicationUpdateChannel::Preview,
            false,
        ),
        (
            "0.2.0-beta.1",
            "0.1.9",
            ApplicationUpdateChannel::Stable,
            false,
        ),
    ] {
        assert_eq!(
            is_newer_release(current, 4, &release(target, 5), channel).unwrap(),
            expected,
            "{current} -> {target} on {channel:?}"
        );
    }
}

#[test]
fn builds_follow_the_installer_upgrade_order() {
    for (current, target, build, expected) in [
        ("0.1.0-beta.1", "0.1.0-beta.1", 4, false),
        ("0.1.0-beta.1", "0.1.0-beta.1", 5, false),
        ("0.1.0-beta.1", "0.1.0", 3, false),
        ("0.1.0", "0.1.1", 4, false),
        ("0.1.0", "0.1.1", 5, true),
        ("0.1.0", "0.2.0", 1, true),
    ] {
        assert_eq!(
            is_newer_release(
                current,
                4,
                &release(target, build),
                ApplicationUpdateChannel::Preview
            )
            .unwrap(),
            expected,
            "{current}+4 -> {target}+{build}"
        );
    }
}

#[test]
fn malformed_release_metadata_never_becomes_an_update() {
    for (version, build) in [("0.1.0-beta.01", 5), ("0.1.0+5", 5), ("0.1.0", 0)] {
        let failure = is_newer_release(
            "0.1.0-beta.1",
            4,
            &release(version, build),
            ApplicationUpdateChannel::Preview,
        )
        .unwrap_err();
        assert_eq!(failure.code, ApplicationUpdateFailureCode::InvalidRelease);
    }
}
