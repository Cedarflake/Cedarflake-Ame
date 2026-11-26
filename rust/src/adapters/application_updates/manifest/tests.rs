use super::*;

fn asset(name: &str) -> PackageAsset {
    PackageAsset {
        file_name: name.to_owned(),
        sha256: "a".repeat(64),
        size: 4096,
    }
}

#[test]
fn release_paths_and_hashes_are_admitted_before_engine_use() {
    for name in [
        "../update.msi",
        "C:\\update.msi",
        "x:stream.msi",
        "update/child.msi",
        " update.msi",
        "update%2fchild.msi",
        "a..msi",
    ] {
        assert!(asset(name).admit().is_err(), "{name}");
        assert!(asset_url("v0.1.0-beta.2", name).is_err(), "{name}");
    }
    assert!(
        asset("Cedarflake-Ame-v0.1.0-beta.2-windows-x64.msi")
            .admit()
            .is_ok()
    );
    let mut package = asset("update.msi");
    package.sha256 = "g".repeat(64);
    assert!(package.admit().is_err());
    package.sha256 = "a".repeat(64);
    package.size = MAX_PACKAGE_BYTES + 1;
    assert!(package.admit().is_err());
}

#[test]
fn manifest_binds_channel_architecture_and_published_bytes() {
    let installed = asset("update.msi");
    let manifest = UpdateManifest {
        schema: 1,
        version: "0.1.0-beta.2".into(),
        build: 5,
        architecture: "x64".into(),
        installed,
        portable: asset("update-full.nupkg"),
    };
    let mut release = GithubRelease {
        tag_name: "v0.1.0-beta.2".into(),
        draft: false,
        prerelease: true,
        body: Some("Release notes".into()),
        assets: vec![GithubAsset {
            name: "update.msi".into(),
            size: 4096,
            state: "uploaded".into(),
            digest: Some(format!("sha256:{}", "a".repeat(64))),
            browser_download_url: asset_url("v0.1.0-beta.2", "update.msi").unwrap(),
        }],
    };
    let (admitted, _) = manifest.admit(&release, "x64", false).unwrap();
    assert_eq!(admitted.version, "0.1.0-beta.2");
    assert_eq!(admitted.notes, "Release notes");
    assert!(manifest.admit(&release, "arm64", false).is_err());
    release.assets[0].size = 4097;
    assert!(manifest.admit(&release, "x64", false).is_err());
    release.assets[0].size = 4096;
    release.prerelease = false;
    assert!(manifest.admit(&release, "x64", false).is_err());
    release.prerelease = true;
    release.assets[0].browser_download_url = "https://example.com/update.msi".into();
    assert!(manifest.admit(&release, "x64", false).is_err());
}
