use chip_8::machine::Chip8;
use chip_8::windowing::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rom = std::fs::read("./tests/fixtures/6-keypad.ch8")?;
    let mut app = App::new(Chip8::from_rom(&rom)?);

    app.run();

    if let Some(error) = app.engine_error {
        Err(Box::new(error))
    } else {
        Ok(())
    }
}
