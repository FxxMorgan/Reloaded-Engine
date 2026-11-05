use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let target_id: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(8017);

    let exe_path = "/run/media/feer/Proyectos/Programacion/PokemonReloadedAndroid/game.exe";
    let mut data_bytes = fs::read(exe_path).expect("failed to read game.exe");
    let logger: Option<fn(&str)> = None;
    match gm8exe::reader::from_exe(&mut data_bytes, logger, false, false) {
        Ok(data) => {
            if target_id < data.scripts.len() {
                if let Some(ref script) = data.scripts[target_id] {
                    println!("=== SCRIPT {} ({}) ===", target_id, script.name);
                    println!("{}", String::from_utf8_lossy(&script.source.0));
                } else {
                    println!("Script {} is None", target_id);
                }
            } else {
                println!("target_id {} exceeds scripts len {}", target_id, data.scripts.len());
            }
        }
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
