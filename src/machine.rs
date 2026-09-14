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

impl Renderable for Chip8 {
    const WIDTH: u32 = WIDTH as u32;
    const HEIGHT: u32 = HEIGHT as u32;

    fn render(&self, pixels: &mut Pixels<'_>) {
        for (i, pixel) in pixels.frame_mut().chunks_exact_mut(4).enumerate() {
            let bit = 7 - i % 8;
            let index = i / 8;
            let line = self.framebuffer()[index];

            if (line & (1 << bit)) != 0 {
                pixel[0] = 0xff;
                pixel[1] = 0x95;
                pixel[2] = 0x0;
                pixel[3] = 0xff;
            } else {
                pixel[0] = 0xff;
                pixel[1] = 0xff;
                pixel[2] = 0xff;
                pixel[3] = 0xff;
            }
        }
    }
}

impl Updatable for Chip8 {
    fn update(&mut self) {
        self.cpu.step(&mut self.peripheral).unwrap();
    }
}
