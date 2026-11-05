use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let query = args.get(1).map(|s| s.to_lowercase()).unwrap_or_else(|| "encaja".to_string());

    let exe_path = "/run/media/feer/Proyectos/Programacion/PokemonReloadedAndroid/game.exe";
    let mut data_bytes = fs::read(exe_path).expect("failed to read game.exe");
    let logger: Option<fn(&str)> = None;
    match gm8exe::reader::from_exe(&mut data_bytes, logger, false, false) {
        Ok(data) => {
            println!("Searching for '{}' in {} scripts...", query, data.scripts.len());
            for (id, s) in data.scripts.iter().enumerate() {
                if let Some(script) = s {
                    let code = String::from_utf8_lossy(&script.source.0).to_lowercase();
                    let name = String::from_utf8_lossy(&script.name.0).to_lowercase();
                    if name.contains(&query) || code.contains(&query) {
                        println!("Found in Script {} ({}):", id, script.name);
                        let snippet: String = code.lines().filter(|l| l.contains(&query)).take(2).collect::<Vec<_>>().join(" | ");
                        println!("   Snippet: {}", snippet);
                    }
                }
            }
        }
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
