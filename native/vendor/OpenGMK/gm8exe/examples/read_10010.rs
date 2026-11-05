use std::fs;

fn main() {
    let mut data_bytes = fs::read("/tmp/reloaded_game.exe").expect("failed to read");
    let logger: Option<fn(&str)> = None;
    match gm8exe::reader::from_exe(&mut data_bytes, logger, false, false) {
        Ok(data) => {
            println!("--- SCRIPT 10010 ---");
            if let Some(s) = &data.scripts[10010] {
                println!("Name: {}\nSource:\n{}", String::from_utf8_lossy(&s.name.0), String::from_utf8_lossy(&s.source.0));
            }
        }
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
