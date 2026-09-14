use chip_8::machine::Chip8;
use chip_8::windowing::App;

fn main() {
    let rom = std::fs::read("./tests/fixtures/1-chip8-logo.ch8").unwrap();
    let mut app = App::new(Chip8::from_rom(&rom).unwrap());

    app.run();
}
