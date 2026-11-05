use std::fs;

fn main() {
    let mut data_bytes = fs::read("/tmp/reloaded_game.exe").expect("failed to read");
    let logger: Option<fn(&str)> = None;
    match gm8exe::reader::from_exe(&mut data_bytes, logger, false, false) {
        Ok(data) => {
            for (idx, script) in data.scripts.iter().enumerate() {
                if let Some(s) = script {
                    let text = String::from_utf8_lossy(&s.source.0);
                    if text.contains("date_get_year") || text.contains("current_year") || text.contains("date_current_datetime") {
                        println!("SCRIPT #{}: {}", idx, s.name);
                        for line in text.lines() {
                            if line.contains("date_") || line.contains("year") || line.contains("show_") {
                                println!("   {}", line);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
