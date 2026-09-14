use crate::cpu::{Address, Display, Memory};

pub const WIDTH: u8 = 64;
pub const HEIGHT: u8 = 32;
pub const FRAMEBUFFER_START: usize = 0xf00;
pub const FRAMEBUFFER_END: usize = 0x1000;
pub const MEMORY_LENGTH: usize = 0x1000;

pub struct Peripheral {
    memory: [u8; MEMORY_LENGTH],
}

impl Peripheral {
    pub fn new(memory: [u8; MEMORY_LENGTH]) -> Self {
        Self { memory }
    }

    pub fn framebuffer(&self) -> &[u8] {
        &self.memory[FRAMEBUFFER_START..FRAMEBUFFER_END]
    }

    fn draw_one(&mut self, index: usize, s: u8) -> bool {
        let ret = self.memory[index] & s != 0x0;

        self.memory[index] ^= s;

        ret
    }
}

impl Memory for Peripheral {
    fn read(&self, address: Address) -> u8 {
        self.memory[address.value() as usize]
    }

    fn write(&mut self, address: Address, value: u8) {
        self.memory[address.value() as usize] = value;
    }
}

impl Display for Peripheral {
    fn clear(&mut self) {
        for i in FRAMEBUFFER_START..FRAMEBUFFER_END {
            self.memory[i] = 0;
        }
    }

    fn draw(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool {
        let x = x % WIDTH;
        let y = (y % HEIGHT) as usize;
        let x_index = (x / 8) as usize;
        let x_bits = x % 8;
        let mut ret = false;

        for (n, &s) in sprite.iter().enumerate() {
            let row = y + n;

            if row >= HEIGHT as usize {
                break;
            }

            let index = FRAMEBUFFER_START + row * 8 + x_index;

            ret |= self.draw_one(index, s >> x_bits);

            if x_bits != 0 && x_index < (WIDTH / 8) as usize - 1 {
                ret |= self.draw_one(index + 1, s << (8 - x_bits));
            }
        }

        ret
    }
}
