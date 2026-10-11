use crate::Error;
use crate::cpu::{Cpu, Quirks, Rng};
use crate::peripheral::{HEIGHT, MEMORY_SIZE, Peripheral, WIDTH, XO_MEMORY_SIZE};
use crate::ticker::{Tickable, Ticker};
use crate::windowing::{KeyListener, Renderable, Updatable};

use pixels::Pixels;
use rand::random;
use std::time::Duration;

struct Random();

impl Rng for Random {
    fn next(&self) -> u8 {
        random()
    }
}

pub struct Chip8Builder {
    stack_in_memory: bool,
    framebuffer_in_memory: bool,
    frequency: u64,
    timendus: u8,
    quirks: Quirks,
    step: fn(&mut Cpu, &mut Peripheral, &Random) -> Result<(), crate::Error>,
    clipping: bool,
    memory_size: usize,
    double_buffer: bool,
}

impl Chip8Builder {
    pub fn frequency(mut self, frequency: u64) -> Self {
        self.frequency = frequency;
        self
    }

    pub fn timendus(mut self, timendus: u8) -> Self {
        self.timendus = timendus;
        self
    }

    pub fn clipping(mut self, clipping: bool) -> Self {
        self.clipping = clipping;
        self
    }

    pub fn load(self, rom: &[u8]) -> Result<Chip8, crate::Error> {
        let start = Cpu::START_PC.value() as usize;
        let fonts = Cpu::FONTS.value() as usize;
        let capacity = self.memory_size - start;

        if rom.len() <= capacity {
            let mut memory = vec![0; fonts];

            memory.extend_from_slice(FONTS);
            memory.extend_from_slice(HIRES_FONTS);
            memory.resize(start - 1, 0);

            memory.push(self.timendus);

            memory.extend_from_slice(rom);
            memory.resize(self.memory_size, 0);

            let screen = if self.double_buffer {
                Some(Screen {
                    is_hi_res: false,
                    plane0: vec![],
                    plane1: vec![],
                })
            } else {
                None
            };

            let component = Component {
                cpu: Cpu::new(self.stack_in_memory, self.quirks),
                peripheral: Peripheral::new(self.framebuffer_in_memory, self.clipping, memory),
                rng: Random(),
                step: self.step,
                screen,
            };
            let ticker = Ticker::new(Duration::from_micros(1_000_000 / self.frequency));

            Ok(Chip8 { component, ticker })
        } else {
            Err(Error::RomTooLarge {
                size: rom.len(),
                capacity,
            })
        }
    }
}

pub enum Chip8Flavor {
    Classic,
    SuperChip,
    Xo,
}

impl Chip8Flavor {
    pub fn builder(self) -> Chip8Builder {
        match self {
            Chip8Flavor::Classic => Chip8Builder {
                stack_in_memory: true,
                framebuffer_in_memory: true,
                frequency: 500,
                timendus: 0,
                quirks: Quirks::classic(),
                step: Cpu::step,
                clipping: true,
                memory_size: MEMORY_SIZE,
                double_buffer: false,
            },
            Chip8Flavor::SuperChip => Chip8Builder {
                stack_in_memory: false,
                framebuffer_in_memory: false,
                frequency: 2_000,
                timendus: 0,
                quirks: Quirks::schip(),
                step: Cpu::super_step,
                clipping: true,
                memory_size: MEMORY_SIZE,
                double_buffer: false,
            },
            Chip8Flavor::Xo => Chip8Builder {
                stack_in_memory: false,
                framebuffer_in_memory: false,
                frequency: 50_000,
                timendus: 0,
                quirks: Quirks::xo(),
                step: Cpu::xo_step,
                clipping: false,
                memory_size: XO_MEMORY_SIZE,
                double_buffer: true,
            },
        }
    }
}

struct Screen {
    is_hi_res: bool,
    plane0: Vec<u8>,
    plane1: Vec<u8>,
}

struct Component {
    cpu: Cpu,
    peripheral: Peripheral,
    rng: Random,
    step: fn(&mut Cpu, &mut Peripheral, &Random) -> Result<(), crate::Error>,
    screen: Option<Screen>,
}

impl Tickable for Component {
    type Error = crate::Error;

    // Timers at 60 Hz
    const DT_TICK_TIMER: Duration = Duration::from_micros(1_000_000 / 60);

    fn tick(&mut self) -> Result<(), Self::Error> {
        (self.step)(&mut self.cpu, &mut self.peripheral, &self.rng)
    }

    fn tick_timer(&mut self) {
        self.cpu.delay_timer();
        self.peripheral.beeper.tick(Self::DT_TICK_TIMER);

        if let Some(screen) = &mut self.screen {
            let (plane0, plane1) = self.peripheral.framebuffer();

            screen.is_hi_res = self.peripheral.is_hi_res();

            screen.plane0.resize(plane0.len(), 0);
            screen.plane1.resize(plane1.len(), 0);

            screen.plane0.copy_from_slice(plane0);
            screen.plane1.copy_from_slice(plane1);
        }
    }
}

pub struct Chip8 {
    component: Component,
    ticker: Ticker,
}

const FONTS: &[u8] = &[
    0xf0, 0x90, 0x90, 0x90, 0xf0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xf0, 0x10, 0xf0, 0x80, 0xf0, // 2
    0xf0, 0x10, 0xf0, 0x10, 0xf0, // 3
    0x90, 0x90, 0xf0, 0x10, 0x10, // 4
    0xf0, 0x80, 0xf0, 0x10, 0xf0, // 5
    0xf0, 0x80, 0xf0, 0x90, 0xf0, // 6
    0xf0, 0x10, 0x20, 0x40, 0x40, // 7
    0xf0, 0x90, 0xf0, 0x90, 0xf0, // 8
    0xf0, 0x90, 0xf0, 0x10, 0xf0, // 9
    0xf0, 0x90, 0xf0, 0x90, 0x90, // a
    0xe0, 0x90, 0xe0, 0x90, 0xe0, // b
    0xf0, 0x80, 0x80, 0x80, 0xf0, // c
    0xe0, 0x90, 0x90, 0x90, 0xe0, // d
    0xf0, 0x80, 0xf0, 0x80, 0xf0, // e
    0xf0, 0x80, 0xf0, 0x80, 0x80, // f
];

const HIRES_FONTS: &[u8] = &[
    0xff, 0xff, 0xc3, 0xc3, 0xc3, 0xc3, 0xc3, 0xc3, 0xff, 0xff, // 0
    0x18, 0x78, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0xff, 0xff, // 1
    0xff, 0xff, 0x03, 0x03, 0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, // 2
    0xff, 0xff, 0x03, 0x03, 0xff, 0xff, 0x03, 0x03, 0xff, 0xff, // 3
    0xc3, 0xc3, 0xc3, 0xc3, 0xff, 0xff, 0x03, 0x03, 0x03, 0x03, // 4
    0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, 0x03, 0x03, 0xff, 0xff, // 5
    0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, 0xc3, 0xc3, 0xff, 0xff, // 6
    0xff, 0xff, 0x03, 0x03, 0x06, 0x0c, 0x18, 0x18, 0x18, 0x18, // 7
    0xff, 0xff, 0xc3, 0xc3, 0xff, 0xff, 0xc3, 0xc3, 0xff, 0xff, // 8
    0xff, 0xff, 0xc3, 0xc3, 0xff, 0xff, 0x03, 0x03, 0xff, 0xff, // 9
    0x7e, 0xff, 0xc3, 0xc3, 0xc3, 0xff, 0xff, 0xc3, 0xc3, 0xc3, // a
    0xfc, 0xfc, 0xc3, 0xc3, 0xfc, 0xfc, 0xc3, 0xc3, 0xfc, 0xfc, // b
    0x3c, 0xff, 0xc3, 0xc0, 0xc0, 0xc0, 0xc0, 0xc3, 0xff, 0x3c, // c
    0xfc, 0xfe, 0xc3, 0xc3, 0xc3, 0xc3, 0xc3, 0xc3, 0xfe, 0xfc, // d
    0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, // e
    0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, 0xc0, 0xc0, 0xc0, 0xc0, // f
];

impl Chip8 {
    pub fn framebuffer(&self) -> (&[u8], &[u8]) {
        self.component.peripheral.framebuffer()
    }

    fn screen(&self) -> (bool, (&[u8], &[u8])) {
        if let Some(screen) = &self.component.screen {
            (screen.is_hi_res, (&screen.plane0, &screen.plane1))
        } else {
            (
                self.component.peripheral.is_hi_res(),
                self.component.peripheral.framebuffer(),
            )
        }
    }
}

const ORANGE: [u8; 4] = [0xff, 0x95, 0x0, 0xff];
const WHITE: [u8; 4] = [0xff; 4];
const DARK: [u8; 4] = [0x80, 0x50, 0x0, 0xff];
const GREY: [u8; 4] = [0xb0, 0x80, 0x0, 0xff];

fn pixel_color((plane0, plane1): (&[u8], &[u8]), i: usize) -> [u8; 4] {
    let bit = 7 - i % 8;
    let index = i / 8;

    let mut color = if plane0[index] & (1 << bit) == 0 {
        0b00
    } else {
        0b01
    };

    if !plane1.is_empty() {
        color |= if plane1[index] & (1 << bit) == 0 {
            0b00
        } else {
            0b10
        };
    }

    match color {
        0b00 => ORANGE,
        0b01 => WHITE,
        0b10 => GREY,
        0b11 => DARK,
        _ => panic!(),
    }
}

impl Renderable for Chip8 {
    const WIDTH: u32 = WIDTH as u32;
    const HEIGHT: u32 = HEIGHT as u32;

    fn render(&self, pixels: &mut Pixels<'_>) {
        let (is_hi_res, planes) = self.screen();
        // We can resize every time because if it's equal it's a noop function
        if is_hi_res {
            pixels
                .resize_buffer(Self::WIDTH * 2, Self::HEIGHT * 2)
                .unwrap();
        } else {
            pixels.resize_buffer(Self::WIDTH, Self::HEIGHT).unwrap();
        }

        let (chunks, _remainders) = pixels.frame_mut().as_chunks_mut::<4>();

        for (i, pixel) in chunks.iter_mut().enumerate() {
            pixel.copy_from_slice(&pixel_color(planes, i));
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
            winit::keyboard::KeyCode::PrintScreen => {
                if !is_pressed {
                    const SCREENSHOT_PATH: &str = "/tmp/framebuffer.data";
                    let (plane0, plane1) = self.framebuffer();

                    println!(
                        "Saving framebuffer in {}: {:?}",
                        SCREENSHOT_PATH,
                        std::fs::write(SCREENSHOT_PATH, [plane0, plane1].concat())
                    );
                }
                return;
            }
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

        assert_eq!(pixel_color((&fb, &[0; 0]), 0), WHITE);
        assert_eq!(pixel_color((&fb, &[0; 0]), 12), WHITE);

        for i in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15] {
            assert_eq!(
                pixel_color((&fb, &[0; 0]), i),
                ORANGE,
                "pixel {i} should be white"
            );
        }
    }
}
