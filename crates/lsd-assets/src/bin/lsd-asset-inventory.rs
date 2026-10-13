use lsd_assets::{
    formats::{Lbd, TimImage, Tix, Tmd, Vram},
    is_canonical_reference, sha256_hex,
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Serialize, Default)]
struct Counts {
    files: usize,
    tim_images: usize,
    tix_banks: usize,
    lbd_files: usize,
    tmd_objects: usize,
    vertices: usize,
    normals: usize,
    polygons: usize,
    present_tiles: usize,
    chained_tiles: usize,
    model_libraries_pending: usize,
    polygon_variants: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct Record {
    path: String,
    bytes: usize,
    sha256: String,
    counts: Counts,
    vram_sha256: Option<String>,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    reference_sha256: String,
    files: Vec<Record>,
    totals: Counts,
    scope: &'static str,
    original_game_groups_accepted: usize,
}

fn collect(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(&entry.path(), paths)?;
        } else if kind.is_file() {
            paths.push(entry.path());
        }
    }
    Ok(())
}

fn count_tmd(model: &Tmd, counts: &mut Counts) {
    counts.tmd_objects += model.objects.len();
    for object in &model.objects {
        counts.vertices += object.vertices.len();
        counts.normals += object.normals.len();
        counts.polygons += object.polygons.len();
        for polygon in &object.polygons {
            let key = format!(
                "{:02x}:{:02x}:{}:{}",
                polygon.mode, polygon.flag, polygon.input_words, polygon.output_words
            );
            *counts.polygon_variants.entry(key).or_default() += 1;
        }
    }
}

fn merge(total: &mut Counts, count: &Counts) {
    total.files += count.files;
    total.tim_images += count.tim_images;
    total.tix_banks += count.tix_banks;
    total.lbd_files += count.lbd_files;
    total.tmd_objects += count.tmd_objects;
    total.vertices += count.vertices;
    total.normals += count.normals;
    total.polygons += count.polygons;
    total.present_tiles += count.present_tiles;
    total.chained_tiles += count.chained_tiles;
    total.model_libraries_pending += count.model_libraries_pending;
    for (key, value) in &count.polygon_variants {
        *total.polygon_variants.entry(key.clone()).or_default() += value;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: lsd-asset-inventory <extracted-files-root>".into());
    }
    let root = PathBuf::from(&args[0]).canonicalize()?;
    let reference = std::fs::read(root.join("SLPS_015.56"))?;
    if !is_canonical_reference(&reference) {
        return Err("Extraction has a different reference executable".into());
    }
    let mut paths = Vec::new();
    collect(&root, &mut paths)?;
    paths.sort();
    let mut files = Vec::new();
    let mut totals = Counts::default();
    for path in paths {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_uppercase();
        if !matches!(extension.as_str(), "TIM" | "TIX" | "TMD" | "LBD") {
            continue;
        }
        let bytes = std::fs::read(&path)?;
        let mut counts = Counts {
            files: 1,
            ..Counts::default()
        };
        let mut vram_sha256 = None;
        let result: Result<(), Box<dyn std::error::Error>> = (|| {
            match extension.as_str() {
                "TIM" => {
                    TimImage::parse(&bytes)?;
                    counts.tim_images = 1;
                }
                "TIX" => {
                    let tix = Tix::parse(&bytes)?;
                    counts.tix_banks = tix.banks.len();
                    let mut vram = Vram::default();
                    for bank in &tix.banks {
                        for image in &bank.images {
                            vram.upload(image)?;
                            counts.tim_images += 1;
                        }
                    }
                    let words: Vec<u8> = vram
                        .words()
                        .iter()
                        .flat_map(|word| word.to_le_bytes())
                        .collect();
                    vram_sha256 = Some(sha256_hex(&words));
                }
                "TMD" => {
                    count_tmd(&Tmd::parse(&bytes)?, &mut counts);
                }
                "LBD" => {
                    let map = Lbd::parse(&bytes)?;
                    counts.lbd_files = 1;
                    counts.present_tiles = map.tiles.iter().filter(|tile| tile.present()).count();
                    counts.chained_tiles = map.tiles.len() - 400;
                    counts.model_libraries_pending = usize::from(map.model_library.is_some());
                    count_tmd(&map.tmd, &mut counts);
                }
                _ => unreachable!(),
            }
            Ok(())
        })();
        if let Err(error) = result {
            return Err(format!("{}: {error}", path.display()).into());
        }
        merge(&mut totals, &counts);
        files.push(Record {
            path: path
                .strip_prefix(&root)?
                .to_string_lossy()
                .replace('\\', "/"),
            bytes: bytes.len(),
            sha256: sha256_hex(&bytes),
            counts,
            vram_sha256,
        });
    }
    let report = Report {
        schema_version: 1,
        reference_sha256: sha256_hex(&reference),
        files,
        totals,
        scope: "Bounded texture/map/polygon decoding and raw TIM transfer order. Runtime rendering, palette shading, MML animation, audio and gameplay acceptance remain open.",
        original_game_groups_accepted: 0,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
