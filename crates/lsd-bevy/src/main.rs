fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut steps = 16usize;
    let mut seed = 1u32;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--steps" => steps = args.next().ok_or("--steps needs a number")?.parse()?,
            "--seed" => seed = args.next().ok_or("--seed needs a number")?.parse()?,
            "--help" | "-h" => {
                println!(
                    "lsd-bevy [--steps 0..1000000] [--seed u32]\nDeterministic compatibility probe; gameplay is not implemented yet."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    if steps > 1_000_000 {
        return Err("--steps must not exceed 1000000".into());
    }
    let state = lsd_bevy::run_probe(seed, steps);
    println!(
        "{{\"probe_steps\":{},\"rng_state\":{},\"last_random\":{},\"gameplay_parity_accepted\":false}}",
        state.steps,
        state.rng_state(),
        state.last_random
    );
    Ok(())
}
