#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use lsd_assets::load_link_tables;
use serde_json::Value;
use std::path::PathBuf;

#[test]
#[ignore = "Requires local original executable and private native link observations"]
fn local_original_link_catalog_and_spawn_results_match_native_cpu() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let observations = PathBuf::from(
        std::env::var_os("LSD_NATIVE_LINK_CASES").expect("Set LSD_NATIVE_LINK_CASES"),
    );
    let catalogue = PathBuf::from(
        std::env::var_os("LSD_NATIVE_LINK_CATALOG").expect("Set LSD_NATIVE_LINK_CATALOG"),
    );
    let tables = load_link_tables(&std::fs::read(files.join("SLPS_015.56")).unwrap()).unwrap();
    let expected: Value = serde_json::from_slice(&std::fs::read(catalogue).unwrap()).unwrap();
    assert_eq!(tables.catalog, support::catalog(&expected));
    let cases: Value = serde_json::from_slice(&std::fs::read(observations).unwrap()).unwrap();
    support::static_cases(&tables.catalog, cases["static"].as_array().unwrap());
    support::spawn_cases(&tables.catalog, cases["random"].as_array().unwrap(), false);
    assert_eq!(
        tables
            .catalog
            .random_spawns
            .iter()
            .map(Vec::len)
            .sum::<usize>(),
        239
    );
    assert_eq!(
        tables
            .catalog
            .wall
            .triggers
            .iter()
            .map(Vec::len)
            .sum::<usize>(),
        17
    );
    assert_eq!(
        tables
            .catalog
            .tunnel
            .triggers
            .iter()
            .map(Vec::len)
            .sum::<usize>(),
        32
    );
    assert_eq!(
        tables
            .catalog
            .teleport
            .triggers
            .iter()
            .map(Vec::len)
            .sum::<usize>(),
        17
    );
    assert_eq!(
        tables
            .catalog
            .staircase
            .triggers
            .iter()
            .map(Vec::len)
            .sum::<usize>(),
        8
    );
}

#[test]
fn link_import_rejects_unidentified_inputs_before_reading_native_pointers() {
    assert!(load_link_tables(&[]).is_err());
    assert!(load_link_tables(&[0; 2048]).is_err());
}
