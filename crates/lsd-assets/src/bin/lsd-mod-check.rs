//! Validate a data-mod profile without opening a window or writing game files.
use lsd_assets::{AssetSource, is_canonical_reference, mods::split_mod_arguments};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (args, packs) = split_mod_arguments(std::env::args().skip(1))?;
    if args.len() != 1 {
        return Err(
            "Usage: lsd-mod-check <extracted-files-root> [--mod <pack-directory>]... [--original]"
                .into(),
        );
    }
    let root = Path::new(&args[0]);
    if !is_canonical_reference(&std::fs::read(root.join("SLPS_015.56"))?) {
        return Err("Extraction has a different reference executable".into());
    }
    let source = AssetSource::with_mods(root, &packs)?;
    println!("{}", source.profile().to_json());
    Ok(())
}
