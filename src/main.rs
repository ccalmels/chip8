use chip_8::machine::{Chip8Builder, Chip8Flavor};
use chip_8::windowing::App;
use clap::{Args, Parser};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    flavor: Flavor,

    /// ROM path
    #[arg()]
    rom_path: String,

    /// Frequency
    #[arg(short, long)]
    frequency: Option<u64>,
}

#[derive(Args)]
#[group(multiple = false)]
struct Flavor {
    /// schip-modern mode
    #[arg(short, long)]
    superchip: bool,

    /// xo mode
    #[arg(short, long)]
    xo: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let rom = std::fs::read(args.rom_path)?;
    let flavor = if args.flavor.superchip {
        Chip8Flavor::SuperChip
    } else if args.flavor.xo {
        Chip8Flavor::Xo
    } else {
        Chip8Flavor::Classic
    };
    let builder = args
        .frequency
        .into_iter()
        .fold(flavor.builder(), Chip8Builder::frequency);

    let mut app = App::new(builder.load(&rom)?);

    app.run();

    if let Some(error) = app.engine_error {
        Err(Box::new(error))
    } else {
        Ok(())
    }
}
