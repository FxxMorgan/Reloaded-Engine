use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = Path::new("/run/media/feer/Proyectos/Programacion/PokemonReloadedAndroid/game.exe");
    let mut exe_bytes = std::fs::read(exe_path)?;
    let assets = gm8exe::reader::from_exe(&mut exe_bytes, None::<fn(&str)>, false, false)?;
    if let Some(script) = &assets.scripts[8768] {
        println!("=== Script 8768 ===");
        println!("{}", String::from_utf8_lossy(&script.source.0));
    }
    if let Some(script) = &assets.scripts[7814] {
        println!("=== Script 7814 ===");
        println!("{}", String::from_utf8_lossy(&script.source.0));
    }
    if let Some(script) = &assets.scripts[9341] {
        println!("=== Script 9341 ===");
        println!("{}", String::from_utf8_lossy(&script.source.0));
    }
    Ok(())
}
