//! Owned synthetic content only; original-game parity is a separate campaign.
use lsd_assets::{AssetSource, CANONICAL_SHA256, mods::split_mod_arguments, sha256_hex};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "lsd-mod-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("base")).unwrap();
        Self(path)
    }
    fn base(&self) -> PathBuf {
        self.0.join("base")
    }
    fn pack(&self, id: &str, data: &[u8]) -> PathBuf {
        let pack = self.0.join(id);
        fs::create_dir(&pack).unwrap();
        fs::write(pack.join("texture.tim"), data).unwrap();
        self.manifest(&pack, json!({
            "schema_version": 1, "id": id, "name": id, "version": "1.0.0",
            "reference_sha256": CANONICAL_SHA256,
            "files": [{"target": "CDI/ETC/ETC.TIM", "source": "texture.tim", "sha256": sha256_hex(data)}]
        }));
        pack
    }
    fn manifest(&self, pack: &Path, value: Value) {
        fs::write(
            pack.join("mod.json"),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
    fn change(&self, pack: &Path, change: impl FnOnce(&mut Value)) {
        let mut value = serde_json::from_slice(&fs::read(pack.join("mod.json")).unwrap()).unwrap();
        change(&mut value);
        self.manifest(pack, value);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn original_mode_does_not_discover_nearby_mods_or_modify_base() {
    let f = Fixture::new();
    fs::create_dir_all(f.base().join("CDI/ETC")).unwrap();
    fs::write(f.base().join("CDI/ETC/ETC.TIM"), b"original").unwrap();
    f.pack("unselected", b"mod");
    let source = AssetSource::with_mods(&f.base(), &[]).unwrap();
    assert_eq!(source.read("CDI/ETC/ETC.TIM").unwrap(), b"original");
    assert_eq!(source.profile(), AssetSource::original(&f.base()).profile());
    assert_eq!(source.profile().mode, "original");
}

#[test]
fn later_packs_win_with_portable_reproducible_receipt_and_immutable_snapshot() {
    let f = Fixture::new();
    let a = f.pack("first", b"one");
    let b = f.pack("second", b"two");
    let source = AssetSource::with_mods(&f.base(), &[a.clone(), b.clone()]).unwrap();
    assert_eq!(source.read("cdi\\etc\\etc.tim").unwrap(), b"two");
    assert_eq!(source.profile().overrides[0].replaces, ["first"]);
    assert_eq!(
        source
            .profile()
            .mods
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    assert_eq!(
        source.profile(),
        AssetSource::with_mods(&f.base(), &[a.clone(), b.clone()])
            .unwrap()
            .profile()
    );
    assert_ne!(
        source.profile().profile_sha256,
        AssetSource::with_mods(&f.base(), &[b.clone(), a])
            .unwrap()
            .profile()
            .profile_sha256
    );
    fs::write(b.join("texture.tim"), b"changed during play").unwrap();
    assert_eq!(source.clone().read("CDI/ETC/ETC.TIM").unwrap(), b"two");
    assert!(
        !source
            .profile()
            .to_json()
            .contains(&f.0.to_string_lossy().to_string())
    );
}

#[test]
fn dependencies_require_explicit_earlier_activation() {
    let f = Fixture::new();
    let a = f.pack("base-pack", b"one");
    let b = f.pack("dependent", b"two");
    f.change(&b, |v| v["requires"] = json!(["base-pack"]));
    assert!(AssetSource::with_mods(&f.base(), std::slice::from_ref(&b)).is_err());
    assert!(AssetSource::with_mods(&f.base(), &[b.clone(), a.clone()]).is_err());
    assert!(AssetSource::with_mods(&f.base(), &[a, b]).is_ok());
}

#[test]
fn declared_conflicts_are_symmetric_and_duplicate_ids_fail() {
    let f = Fixture::new();
    let a = f.pack("first", b"one");
    let b = f.pack("second", b"two");
    f.change(&a, |v| v["conflicts"] = json!(["second"]));
    assert!(AssetSource::with_mods(&f.base(), &[a.clone(), b.clone()]).is_err());
    assert!(AssetSource::with_mods(&f.base(), &[b, a.clone()]).is_err());
    assert!(AssetSource::with_mods(&f.base(), &[a.clone(), a]).is_err());
}

#[test]
fn incompatible_api_reference_and_unknown_fields_fail() {
    for (key, value) in [
        ("schema_version", json!(2)),
        ("reference_sha256", json!("wrong")),
        ("typo", json!(1)),
    ] {
        let f = Fixture::new();
        let pack = f.pack("example", b"one");
        f.change(&pack, |v| v[key] = value);
        assert!(AssetSource::with_mods(&f.base(), &[pack]).is_err(), "{key}");
    }
}

#[test]
fn changed_missing_and_corrupt_payloads_fail_without_fallback() {
    let f = Fixture::new();
    let pack = f.pack("example", b"one");
    fs::write(pack.join("texture.tim"), b"corrupt").unwrap();
    assert!(
        AssetSource::with_mods(&f.base(), std::slice::from_ref(&pack))
            .unwrap_err()
            .to_string()
            .contains("SHA-256")
    );
    fs::remove_file(pack.join("texture.tim")).unwrap();
    assert!(AssetSource::with_mods(&f.base(), &[pack]).is_err());
}

#[test]
fn traversal_absolute_paths_device_names_and_executable_overrides_fail() {
    for target in [
        "../ETC.TIM",
        "/CDI/ETC.TIM",
        "C:/CDI/ETC.TIM",
        "CDI/../ETC.TIM",
        "CDI//ETC.TIM",
        "CDI/NUL.TIM",
        "CDI/A.TIM:stream",
        "SLPS_015.56",
        "CDI/PLUGIN.DLL",
        "CDI/ETC./ETC.TIM",
    ] {
        let f = Fixture::new();
        let pack = f.pack("example", b"one");
        f.change(&pack, |v| v["files"][0]["target"] = json!(target));
        assert!(
            AssetSource::with_mods(&f.base(), &[pack]).is_err(),
            "{target}"
        );
    }
}

#[test]
fn duplicate_targets_are_case_and_separator_insensitive() {
    let f = Fixture::new();
    let pack = f.pack("example", b"one");
    f.change(&pack, |v| {
        let mut alias = v["files"][0].clone();
        alias["target"] = json!("cdi\\etc\\etc.tim");
        v["files"].as_array_mut().unwrap().push(alias);
    });
    assert!(AssetSource::with_mods(&f.base(), &[pack]).is_err());
}

#[test]
fn source_traversal_and_invalid_base_read_paths_fail() {
    let f = Fixture::new();
    let pack = f.pack("example", b"one");
    f.change(&pack, |v| {
        v["files"][0]["source"] = json!("../example/texture.tim")
    });
    assert!(AssetSource::with_mods(&f.base(), &[pack]).is_err());
    let original = AssetSource::original(&f.base());
    assert!(original.read("../example/texture.tim").is_err());
    assert!(original.read("").is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_payload_cannot_escape_the_selected_pack() {
    let f = Fixture::new();
    let pack = f.pack("example", b"one");
    fs::write(f.0.join("outside.tim"), b"one").unwrap();
    fs::remove_file(pack.join("texture.tim")).unwrap();
    std::os::unix::fs::symlink(f.0.join("outside.tim"), pack.join("texture.tim")).unwrap();
    assert!(AssetSource::with_mods(&f.base(), &[pack]).is_err());
}

#[test]
fn bounded_manifest_and_payload_reads_reject_oversize_files() {
    let f = Fixture::new();
    let pack = f.pack("example", b"one");
    fs::File::options()
        .write(true)
        .open(pack.join("texture.tim"))
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    assert!(AssetSource::with_mods(&f.base(), std::slice::from_ref(&pack)).is_err());
    fs::write(pack.join("mod.json"), vec![b' '; 64 * 1024 + 1]).unwrap();
    assert!(AssetSource::with_mods(&f.base(), &[pack]).is_err());
}

#[test]
fn cli_preserves_host_options_and_requires_unambiguous_activation() {
    let parse = |args: &[&str]| split_mod_arguments(args.iter().map(|s| (*s).to_owned()));
    let (host, packs) = parse(&[
        "game", "0", "0", "A", "--dream", "--mod", "pack one", "--audio", "--mod", "pack two",
    ])
    .unwrap();
    assert_eq!(host, ["game", "0", "0", "A", "--dream", "--audio"]);
    assert_eq!(
        packs,
        [PathBuf::from("pack one"), PathBuf::from("pack two")]
    );
    for args in [
        &["--mod"][..],
        &["--mod", "--audio"],
        &["--mod", ""],
        &["--original", "--mod", "pack"],
    ] {
        assert!(parse(args).is_err());
    }
}

#[test]
fn owned_texture_replacement_reaches_the_existing_native_format_decoder() {
    let f = Fixture::new();
    // An owned two-pixel direct-color TIM, not original game data.
    let mut tim = Vec::new();
    for word in [0x10u32, 2, 16] {
        tim.extend(word.to_le_bytes());
    }
    for half in [0u16, 0, 2, 1, 0x001f, 0x03e0] {
        tim.extend(half.to_le_bytes());
    }
    let pack = f.pack("two-pixels", &tim);
    let source = AssetSource::with_mods(&f.base(), &[pack]).unwrap();
    let bytes = source.read("CDI/ETC/ETC.TIM").unwrap();
    let image = lsd_assets::formats::TimImage::parse(&bytes).unwrap();
    let mut vram = lsd_assets::formats::Vram::default();
    vram.upload(&image).unwrap();
    assert_eq!(source.profile().overrides[0].sha256, sha256_hex(&tim));
    assert_eq!((image.image.width_words, image.image.height), (2, 1));
}

#[test]
#[ignore = "Requires local original executable, stage maps and texture variants"]
fn native_stream_and_destination_rebuild_retain_the_selected_texture_snapshot() {
    let f = Fixture::new();
    let root = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = fs::read(root.join("SLPS_015.56")).unwrap();
    assert!(lsd_assets::is_canonical_reference(&executable));
    let original_a = fs::read(root.join("CDI/STG00/TEXA.TIX")).unwrap();
    let original_b = fs::read(root.join("CDI/STG00/TEXB.TIX")).unwrap();
    assert_ne!(original_a, original_b);
    let pack = f.pack("alternate-texture", &original_b);
    f.change(&pack, |v| {
        v["files"][0]["target"] = json!("CDI/STG00/TEXA.TIX")
    });
    let source = AssetSource::with_mods(&root, std::slice::from_ref(&pack)).unwrap();
    fs::remove_file(pack.join("texture.tim")).unwrap();
    let stage =
        lsd_assets::StreamedStage::load_from_source(&source, &executable, 0, 0, [0; 3]).unwrap();
    let expected = stage.load_texture_variant('B').unwrap();
    let replaced = stage.load_texture_variant('A').unwrap();
    assert_eq!(replaced.words(), expected.words());
    let spawn = lsd_core::SpawnLocation {
        chunk: 0,
        tile: 0,
        position: [0; 3],
    };
    let (destination, _) = stage.load_destination(0, spawn).unwrap();
    assert_eq!(
        destination.load_texture_variant('A').unwrap().words(),
        expected.words()
    );
    let original = lsd_assets::StreamedStage::load(&root, &executable, 0, 0, [0; 3]).unwrap();
    assert_ne!(
        original.load_texture_variant('A').unwrap().words(),
        expected.words()
    );
    assert_eq!(
        fs::read(root.join("CDI/STG00/TEXA.TIX")).unwrap(),
        original_a
    );
    assert_eq!(
        fs::read(root.join("CDI/STG00/TEXB.TIX")).unwrap(),
        original_b
    );
}
