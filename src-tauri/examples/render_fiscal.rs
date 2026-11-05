//! Local PDF verification: cargo run --example render_fiscal -- XML OUTPUT.pdf
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Uso: render_fiscal XML OUTPUT.pdf".into());
    }
    let xml = std::fs::read_to_string(&args[1])?;
    let pdf = danfe::render_default(&xml)?;
    std::fs::write(&args[2], pdf)?;
    Ok(())
}
