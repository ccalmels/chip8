use crate::Error;
use crate::cpu::Cpu;
use crate::peripheral::{HEIGHT, MEMORY_LENGTH, Peripheral, WIDTH};
use crate::ticker::{Tickable, Ticker};
use crate::windowing::{KeyListener, Renderable, Updatable};

use pixels::Pixels;
use std::time::Duration;

struct Component {
    cpu: Cpu,
    peripheral: Peripheral,
}

impl Tickable for Component {
    type Error = crate::Error;

    // CPU at 500 Hz
    const DT_TICK: Duration = Duration::from_micros(1_000_000 / 500);
    // Timers at 60 Hz
    const DT_TICK_TIMER: Duration = Duration::from_micros(1_000_000 / 60);

    fn tick(&mut self) -> Result<(), Self::Error> {
        self.cpu.step(&mut self.peripheral)
    }

    fn tick_timer(&mut self) {
        self.cpu.tick_timer(&self.peripheral.beeper);
    }
}

pub struct Chip8 {
    component: Component,
    ticker: Ticker,
}

impl Chip8 {
    pub fn from_rom_with_quirk(rom: &[u8], quirk: u8) -> Result<Self, crate::Error> {
        let start = Cpu::START_PC.value() as usize;
        let capacity = MEMORY_LENGTH - start;

        if rom.len() < capacity {
            let mut memory = vec![0; start - 1];

            memory.push(quirk);

            memory.extend_from_slice(rom);
            memory.resize(MEMORY_LENGTH, 0);

            let component = Component {
                cpu: Cpu::new(),
                peripheral: Peripheral::new(memory.try_into().unwrap()),
            };
            let ticker = Ticker::new();

            Ok(Self { component, ticker })
        } else {
            Err(Error::RomTooLarge {
                size: rom.len(),
                capacity,
            })
        }
    }

    pub fn from_rom(rom: &[u8]) -> Result<Self, crate::Error> {
        Self::from_rom_with_quirk(rom, 0)
    }

    pub fn framebuffer(&self) -> &[u8] {
        self.component.peripheral.framebuffer()
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
            pixel.copy_from_slice(&pixel_color(self.framebuffer(), i));
        }
    }
}

impl Updatable for Chip8 {
    type Error = crate::Error;

    fn update(&mut self, delta: Duration) -> Result<(), crate::Error> {
        self.ticker.tick(delta, &mut self.component)
    }
}

impl KeyListener for Chip8 {
    fn event(&mut self, key: winit::keyboard::KeyCode, is_pressed: bool) {
        let key = match key {
            winit::keyboard::KeyCode::Digit1 => 1,
            winit::keyboard::KeyCode::Digit2 => 2,
            winit::keyboard::KeyCode::Digit3 => 3,
            winit::keyboard::KeyCode::Digit4 => 0xc,
            winit::keyboard::KeyCode::KeyQ => 4,
            winit::keyboard::KeyCode::KeyW => 5,
            winit::keyboard::KeyCode::KeyE => 6,
            winit::keyboard::KeyCode::KeyR => 0xd,
            winit::keyboard::KeyCode::KeyA => 7,
            winit::keyboard::KeyCode::KeyS => 8,
            winit::keyboard::KeyCode::KeyD => 9,
            winit::keyboard::KeyCode::KeyF => 0xe,
            winit::keyboard::KeyCode::KeyZ => 0xa,
            winit::keyboard::KeyCode::KeyX => 0,
            winit::keyboard::KeyCode::KeyC => 0xb,
            winit::keyboard::KeyCode::KeyV => 0xf,
            _ => return,
        };

        self.component.peripheral.keypad.key_event(key, is_pressed);
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
