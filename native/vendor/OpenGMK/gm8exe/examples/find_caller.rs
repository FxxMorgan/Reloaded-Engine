use std::fs;

fn main() {
    let mut data_bytes = fs::read("/tmp/reloaded_game.exe").expect("failed to read");
    let logger: Option<fn(&str)> = None;
    match gm8exe::reader::from_exe(&mut data_bytes, logger, false, false) {
        Ok(data) => {
            for (idx, script) in data.scripts.iter().enumerate() {
                if let Some(s) = script {
                    let text = String::from_utf8_lossy(&s.source.0);
                    if text.contains("ValidateCV") || text.contains("ex(10002") {
                        println!("CALLER SCRIPT #{}: {}", idx, String::from_utf8_lossy(&s.name.0));
                        println!("{}", text);
                    }
                }
            }
            for (idx, obj) in data.objects.iter().enumerate() {
                if let Some(o) = obj {
                    for (_sub, actions) in o.events.iter().flat_map(|list| list.iter()) {
                        for act in actions {
                            for p in &act.param_strings {
                                let code = String::from_utf8_lossy(&p.0);
                                if code.contains("ValidateCV") || code.contains("ex(10002") {
                                    println!("CALLER OBJECT #{}: {}", idx, String::from_utf8_lossy(&o.name.0));
                                    println!("{}", code);
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
