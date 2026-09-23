use chip_8::machine::Chip8;
use chip_8::windowing::Updatable;

use std::time::Duration;

fn assert_rom_matches_expected(rom: &[u8], expected: &[u8], steps: usize) {
    let mut chip8 = Chip8::from_rom(rom).unwrap();

    for _ in 0..steps {
        chip8.update(Duration::from_millis(2)).unwrap();
    }

    assert_eq!(chip8.framebuffer(), expected);
}

fn assert_rom_matches_expected_duration(rom: &[u8], expected: &[u8], duration: Duration) {
    let mut chip8 = Chip8::from_rom(rom).unwrap();

    chip8.update(duration).unwrap();

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
    assert_rom_matches_expected_duration(
        include_bytes!("fixtures/3-corax+.ch8"),
        include_bytes!("fixtures/3-corax+.expected"),
        Duration::from_secs(1),
    );
}

#[test]
fn flags() {
    assert_rom_matches_expected_duration(
        include_bytes!("fixtures/4-flags.ch8"),
        include_bytes!("fixtures/4-flags.expected"),
        Duration::from_secs(2),
    );
}

#[test]
fn quirks() {
    let expected = include_bytes!("fixtures/5-quirks.expected");
    let mut chip8 = Chip8::from_rom_with_quirk(include_bytes!("fixtures/5-quirks.ch8"), 1).unwrap();

    chip8.update(Duration::from_secs(5)).unwrap();

    assert_eq!(chip8.framebuffer(), expected);
}
