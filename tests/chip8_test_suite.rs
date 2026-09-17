use chip_8::machine::Chip8;
use chip_8::windowing::Updatable;

fn assert_rom_matches_expected(rom: &[u8], expected: &[u8], steps: usize) {
    let mut chip8 = Chip8::from_rom(rom).unwrap();

    for _ in 0..steps {
        chip8.update();
    }

    assert_eq!(chip8.framebuffer(), expected);
}

#[test]
fn chip8_logo() {
    assert_rom_matches_expected(
        include_bytes!("fixtures/1-chip8-logo.ch8"),
        include_bytes!("fixtures/1-chip8-logo.expected"),
        39, // this number comes from Timendus CHIP-8 splash screen description
    );
}

#[test]
fn ibm_logo() {
    assert_rom_matches_expected(
        include_bytes!("fixtures/2-ibm-logo.ch8"),
        include_bytes!("fixtures/2-ibm-logo.expected"),
        20, // this number comes from Timendus IBM splash screen description
    );
}

#[test]
fn corax_plus() {
    assert_rom_matches_expected(
        include_bytes!("fixtures/3-corax+.ch8"),
        include_bytes!("fixtures/3-corax+.expected"),
        306, // this number was found by dichotomy
    );
}
