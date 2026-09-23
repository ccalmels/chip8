use chip_8::machine::Chip8;
use chip_8::windowing::App;
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg()]
    rom_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let rom = std::fs::read(args.rom_path)?;
    let mut app = App::new(Chip8::from_rom(&rom)?);

    app.run();

    if let Some(error) = app.engine_error {
        Err(Box::new(error))
    } else {
        Ok(())
    }
}
