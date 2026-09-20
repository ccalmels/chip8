use crate::cpu::{Address, Display, Keypad, Memory};

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

    pub fn quirks_rom(&mut self, value: u8) {
        self.memory[0x1ff] = value;
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

impl Keypad for Peripheral {
    fn key_pressed(&self, _key: u8) -> bool {
        false
    }

    fn wait_key(&self) -> Option<u8> {
        None
    }
}

#[cfg(test)]
mod peripheral_tests {
    use super::*;

    #[test]
    fn clear_only_framebuffer() {
        let mut p = Peripheral::new([0xff; MEMORY_LENGTH]);

        p.clear();

        assert_eq!(
            p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END],
            [0; FRAMEBUFFER_END - FRAMEBUFFER_START]
        );
        assert_eq!(p.memory[..FRAMEBUFFER_START], [0xff; FRAMEBUFFER_START]);
    }

    #[test]
    fn draw_inside_with_one_line_aligned() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(0, 0, &[0xff]));

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START], 0xff);
        assert_eq!(
            p.memory[FRAMEBUFFER_START + 1..],
            [0; MEMORY_LENGTH - FRAMEBUFFER_START - 1]
        );
    }

    #[test]
    fn draw_inside_with_one_line_not_aligned() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(1, 0, &[0xff]));

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START], 0x7f);
        assert_eq!(p.memory[FRAMEBUFFER_START + 1], 0x80);
        assert_eq!(
            p.memory[FRAMEBUFFER_START + 2..],
            [0; MEMORY_LENGTH - FRAMEBUFFER_START - 2]
        );
    }

    #[test]
    fn draw_inside_with_one_line_not_aligned_large() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(6, 0, &[0xff]));

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START], 0x03);
        assert_eq!(p.memory[FRAMEBUFFER_START + 1], 0xfc);
        assert_eq!(
            p.memory[FRAMEBUFFER_START + 2..],
            [0; MEMORY_LENGTH - FRAMEBUFFER_START - 2]
        );
    }

    #[test]
    fn draw_inside_with_several_lines_aligned() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(32, 16, &[0xde, 0xad, 0xbe, 0xef]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[16 * 8 + 4] = 0xde;
        expected[17 * 8 + 4] = 0xad;
        expected[18 * 8 + 4] = 0xbe;
        expected[19 * 8 + 4] = 0xef;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_inside_with_several_lines_not_aligned() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(32 + 3, 16, &[0xde, 0xad, 0xbe, 0xef]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[16 * 8 + 4] = 0xde >> 3;
        expected[16 * 8 + 5] = 0xde << 5;
        expected[17 * 8 + 4] = 0xad >> 3;
        expected[17 * 8 + 5] = 0xad << 5;
        expected[18 * 8 + 4] = 0xbe >> 3;
        expected[18 * 8 + 5] = 0xbe << 5;
        expected[19 * 8 + 4] = 0xef >> 3;
        expected[19 * 8 + 5] = 0xef << 5;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_clips_sprite_at_bottom() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(32, 30, &[0xde, 0xad, 0xbe, 0xef]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[30 * 8 + 4] = 0xde;
        expected[31 * 8 + 4] = 0xad;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_clips_sprite_at_right() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(60, 16, &[0xde, 0xad, 0xbe, 0xef]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[16 * 8 + 7] = 0x0d;
        expected[17 * 8 + 7] = 0x0a;
        expected[18 * 8 + 7] = 0x0b;
        expected[19 * 8 + 7] = 0x0e;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_xor_report_collision() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(0, 0, &[0xff]));
        assert!(p.draw(7, 0, &[0xff]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[0] = 0xfe;
        expected[1] = 0xfe;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_xor_report_collision_with_overlap() {
        let mut p = Peripheral::new([0; MEMORY_LENGTH]);

        assert!(!p.draw(0, 0, &[0xff]));
        assert!(p.draw(5, 0, &[0x81]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[0] = 0xfb;
        expected[1] = 0x08;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }
}
