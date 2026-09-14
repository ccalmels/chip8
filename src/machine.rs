use crate::Error;
use crate::cpu::Cpu;
use crate::peripheral::{HEIGHT, MEMORY_LENGTH, Peripheral, WIDTH};
use crate::windowing::{Renderable, Updatable};

use pixels::Pixels;

pub struct Chip8 {
    cpu: Cpu,
    peripheral: Peripheral,
}

impl Chip8 {
    pub fn from_rom(rom: &[u8]) -> Result<Self, crate::Error> {
        let start = Cpu::START_PC.value() as usize;
        let capacity = MEMORY_LENGTH - start;

        if rom.len() < capacity {
            let mut memory = vec![0; start];

            memory.extend_from_slice(rom);
            memory.resize(MEMORY_LENGTH, 0);

            Ok(Self {
                cpu: Cpu::new(),
                peripheral: Peripheral::new(memory.try_into().unwrap()),
            })
        } else {
            Err(Error::RomTooLarge {
                size: rom.len(),
                capacity,
            })
        }
    }

    pub fn framebuffer(&self) -> &[u8] {
        self.peripheral.framebuffer()
    }
}

const ORANGE: [u8; 4] = [0xff, 0x95, 0x0, 0xff];
const WHITE: [u8; 4] = [0xff; 4];

fn pixel_color(framebuffer: &[u8], i: usize) -> [u8; 4] {
    let bit = 7 - i % 8;
    let index = i / 8;
    let line = framebuffer[index];

    if (line & (1 << bit)) != 0 {
        ORANGE
    } else {
        WHITE
    }
}

impl Renderable for Chip8 {
    const WIDTH: u32 = WIDTH as u32;
    const HEIGHT: u32 = HEIGHT as u32;

    fn render(&self, pixels: &mut Pixels<'_>) {
        for (i, pixel) in pixels.frame_mut().chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&pixel_color(
                self.framebuffer(),
                i,
            ));
        }
    }
}

impl Updatable for Chip8 {
    fn update(&mut self) {
        self.cpu.step(&mut self.peripheral).unwrap();
    }
}

#[cfg(test)]
mod pixel_color_tests {
    use super::*;

    #[test]
    fn pixel_color_is_fill() {
        let mut fb = [0; 256];

        fb[0] = 0b10000000;
        fb[1] = 0b00001000;

        assert_eq!(pixel_color(&fb, 0), ORANGE);
        assert_eq!(pixel_color(&fb, 12), ORANGE);

        for i in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15] {
            assert_eq!(pixel_color(&fb, i), WHITE, "pixel {i} should be white");
        }
    }
}
