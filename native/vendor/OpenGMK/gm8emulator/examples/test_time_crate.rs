fn main() {
    println!("Testing time crate...");
    let dt = gm8emulator::gml::datetime::DateTime::now();
    println!("Year: {}, Month: {}, Day: {}", dt.year(), dt.month(), dt.day());
}
