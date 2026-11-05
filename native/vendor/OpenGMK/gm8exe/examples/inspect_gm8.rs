use std::fs;

fn main() {
    let mut data_bytes = fs::read("/tmp/reloaded_game.exe").expect("failed to read");
    println!("Read {} bytes", data_bytes.len());
    let logger: Option<fn(&str)> = None;
    match gm8exe::reader::from_exe(&mut data_bytes, logger, false, false) {
        Ok(data) => {
            println!("Rooms: {}, Objects: {}, Scripts: {}", data.rooms.len(), data.objects.len(), data.scripts.len());
            for (idx, script) in data.scripts.iter().enumerate() {
                if let Some(s) = script {
                    let text = String::from_utf8_lossy(&s.source.0);
                    let src_lower = text.to_lowercase();
                    if src_lower.contains("ajusta") || src_lower.contains("fecha") {
                        println!("FOUND SCRIPT #{}: {}", idx, s.name);
                        for line in text.lines() {
                            println!("   {}", line);
                        }
                    }
                }
            }
            for (idx, obj) in data.objects.iter().enumerate() {
                if let Some(o) = obj {
                    for (_sub, actions) in o.events.iter().flat_map(|list| list.iter()) {
                        for act in actions {
                            for p in &act.param_strings {
                                let code = String::from_utf8_lossy(&p.0);
                                if code.to_lowercase().contains("ajusta") || code.to_lowercase().contains("fecha") {
                                    println!("FOUND IN OBJECT #{}: {}", idx, o.name);
                                    for line in code.lines() {
                                        println!("   {}", line);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
