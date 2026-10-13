use lsd_assets::{PsxExeHeader, is_canonical_reference, sha256_hex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: lsd-asset-check <path-to-SLPS_015.56>".into());
    }
    let bytes = std::fs::read(&args[0])?;
    let header = PsxExeHeader::parse(&bytes)?;
    println!(
        "bytes: {}\nentry: {:#010x}\nload: {:#010x}\nsha256: {}\ncanonical: {}",
        bytes.len(),
        header.entry_point,
        header.load_address,
        sha256_hex(&bytes),
        is_canonical_reference(&bytes)
    );
    if !is_canonical_reference(&bytes) {
        return Err("Reference hash does not match SLPS_015.56".into());
    }
    Ok(())
}
