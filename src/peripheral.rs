use crate::ca555::Ca555;
use crate::cd4515::Cd4515;
use crate::cpu::{Address, Beeper, Display, Keypad, Memory, SuperDisplay, XoAudio, XoDisplay};

const PIXEL_PER_U8: usize = 8;
pub const WIDTH: u8 = 64;
pub const HEIGHT: u8 = 32;
const WIDTH_IN_U8: usize = WIDTH as usize / PIXEL_PER_U8;
pub const FRAMEBUFFER_SIZE: usize = WIDTH_IN_U8 * HEIGHT as usize;
const HIRES_FRAMEBUFFER_SIZE: usize = FRAMEBUFFER_SIZE * 4;

pub const FRAMEBUFFER_START: usize = 0xf00;
pub const FRAMEBUFFER_END: usize = 0x1000;
pub const MEMORY_SIZE: usize = 0x1000;
pub const XO_MEMORY_SIZE: usize = 0x10000;

struct InternalFrameBuffer {
    plane0: [u8; HIRES_FRAMEBUFFER_SIZE],
    plane1: [u8; HIRES_FRAMEBUFFER_SIZE],
    is_hi_res: bool,
    selected_planes: u8,
}

impl InternalFrameBuffer {
    fn planes(&self) -> (&[u8], &[u8]) {
        if self.is_hi_res {
            (&self.plane0, &self.plane1)
        } else {
            (
                &self.plane0[..FRAMEBUFFER_SIZE],
                &self.plane1[..FRAMEBUFFER_SIZE],
            )
        }
    }

    fn planes_mut(&mut self) -> (&mut [u8], &mut [u8]) {
        if self.is_hi_res {
            (&mut self.plane0, &mut self.plane1)
        } else {
            (
                &mut self.plane0[..FRAMEBUFFER_SIZE],
                &mut self.plane1[..FRAMEBUFFER_SIZE],
            )
        }
    }
}

pub struct Peripheral {
    memory: Vec<u8>,
    pub(crate) beeper: Ca555,
    pub(crate) keypad: Cd4515,
    framebuffer: Option<InternalFrameBuffer>,
    clipping: bool,
}

impl Peripheral {
    pub fn new(framebuffer_in_memory: bool, clipping: bool, memory: Vec<u8>) -> Self {
        Self {
            memory,
            beeper: Ca555::default(),
            keypad: Cd4515::default(),
            framebuffer: if framebuffer_in_memory {
                None
            } else {
                Some(InternalFrameBuffer {
                    plane0: [0; HIRES_FRAMEBUFFER_SIZE],
                    plane1: [0; HIRES_FRAMEBUFFER_SIZE],
                    is_hi_res: false,
                    selected_planes: Self::PLANE0,
                })
            },
            clipping,
        }
    }

    pub fn framebuffer(&self) -> (&[u8], &[u8]) {
        if let Some(fb) = &self.framebuffer {
            fb.planes()
        } else {
            (&self.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], &[0; 0])
        }
    }

    fn framebuffer_mut(&mut self) -> (&mut [u8], &mut [u8]) {
        if let Some(fb) = &mut self.framebuffer {
            fb.planes_mut()
        } else {
            (
                &mut self.memory[FRAMEBUFFER_START..FRAMEBUFFER_END],
                &mut [0; 0],
            )
        }
    }

    fn draw_one(&mut self, index: usize, s: u8) -> bool {
        let selected_planes = self.selected_planes();
        let (plane0, plane1) = self.framebuffer_mut();
        let mut ret = false;

        if selected_planes & Self::PLANE0 == Self::PLANE0 {
            ret |= plane0[index] & s != 0x0;
            plane0[index] ^= s;
        }

        if selected_planes & Self::PLANE1 == Self::PLANE1 {
            ret |= plane1[index] & s != 0x0;
            plane1[index] ^= s;
        }

        ret
    }

    pub fn is_hi_res(&self) -> bool {
        if let Some(fb) = &self.framebuffer {
            fb.is_hi_res
        } else {
            false
        }
    }

    fn selected_planes(&self) -> u8 {
        if let Some(fb) = &self.framebuffer {
            fb.selected_planes
        } else {
            Self::PLANE0
        }
    }

    fn width_in_u8(&self) -> usize {
        if self.is_hi_res() {
            2 * WIDTH_IN_U8
        } else {
            WIDTH_IN_U8
        }
    }
}

impl Memory for Peripheral {
    fn read(&self, address: Address<16>) -> u8 {
        self.memory[address.value() as usize]
    }

    fn write(&mut self, address: Address<16>, value: u8) {
        self.memory[address.value() as usize] = value;
    }
}

impl Display for Peripheral {
    fn clear(&mut self) {
        let selected_planes = self.selected_planes();
        let (plane0, plane1) = self.framebuffer_mut();

        if selected_planes & Self::PLANE0 == Self::PLANE0 {
            plane0.fill(0);
        }

        if selected_planes & Self::PLANE1 == Self::PLANE1 {
            plane1.fill(0);
        }
    }

    fn draw(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool {
        let (width, height, width_in_u8) = if self.is_hi_res() {
            (WIDTH * 2, HEIGHT * 2, WIDTH_IN_U8 * 2)
        } else {
            (WIDTH, HEIGHT, WIDTH_IN_U8)
        };

        let x = (x % width) as usize;
        let y = (y % height) as usize;
        let x_index = x / PIXEL_PER_U8;
        let x_bits = x % PIXEL_PER_U8;
        let mut ret = false;

        for (n, &s) in sprite.iter().enumerate() {
            let row = y + n;

            if self.clipping && row >= height as usize {
                break;
            }

            let index = (row % height as usize) * width_in_u8 + x_index;

            ret |= self.draw_one(index, s >> x_bits);

            if x_bits != 0 {
                if x_index < width_in_u8 - 1 {
                    ret |= self.draw_one(index + 1, s << (PIXEL_PER_U8 - x_bits));
                } else if !self.clipping {
                    ret |= self.draw_one(index - x_index, s << (PIXEL_PER_U8 - x_bits));
                }
            }
        }

        ret
    }
}

impl Keypad for Peripheral {
    fn key_pressed(&self, key: u8) -> bool {
        self.keypad.key_pressed(key)
    }

    fn wait_key(&mut self) -> Option<u8> {
        self.keypad.wait_key()
    }
}

impl Beeper for Peripheral {
    fn sound_timer(&mut self, st: u8) {
        self.beeper.set(st);
    }
}

fn scroll_line_right(line: &mut [u8]) {
    let mut rest = 0;

    for element in line.iter_mut() {
        let next = (*element & 0xf) << 4;

        *element = rest | *element >> 4;

        rest = next;
    }
}

fn scroll_line_left(line: &mut [u8]) {
    let mut rest = 0;

    for element in line.iter_mut().rev() {
        let next = (*element & 0xf0) >> 4;

        *element = rest | *element << 4;

        rest = next;
    }
}

fn scroll(framebuffer: &mut [u8], n: usize, f: fn(&mut [u8])) {
    let chunks = framebuffer.chunks_exact_mut(n);

    for line in chunks {
        f(line);
    }
}

impl SuperDisplay for Peripheral {
    fn switch_resolution(&mut self, hi_res: bool) {
        if let Some(fb) = &mut self.framebuffer {
            fb.is_hi_res = hi_res;
        }
    }

    fn scroll_down(&mut self, n: u8) {
        let selected_planes = self.selected_planes();
        let to_scroll = self.width_in_u8() * n as usize;
        let (plane0, plane1) = self.framebuffer_mut();

        if selected_planes & 0b01 != 0 {
            plane0.copy_within(..plane0.len() - to_scroll, to_scroll);
            plane0[..to_scroll].fill(0);
        }
        if selected_planes & 0b10 != 0 {
            plane1.copy_within(..plane1.len() - to_scroll, to_scroll);
            plane1[..to_scroll].fill(0);
        }
    }

    fn scroll_right(&mut self) {
        let width_in_u8 = self.width_in_u8();
        let selected_planes = self.selected_planes();
        let (plane0, plane1) = self.framebuffer_mut();

        if selected_planes & 0b01 != 0 {
            scroll(plane0, width_in_u8, scroll_line_right);
        }
        if selected_planes & 0b10 != 0 {
            scroll(plane1, width_in_u8, scroll_line_right);
        }
    }

    fn scroll_left(&mut self) {
        let width_in_u8 = self.width_in_u8();
        let selected_planes = self.selected_planes();
        let (plane0, plane1) = self.framebuffer_mut();

        if selected_planes & 0b01 != 0 {
            scroll(plane0, width_in_u8, scroll_line_left);
        }
        if selected_planes & 0b10 != 0 {
            scroll(plane1, width_in_u8, scroll_line_left);
        }
    }
}

impl XoDisplay for Peripheral {
    fn planes(&mut self, planes: u8) {
        if let Some(fb) = &mut self.framebuffer {
            fb.selected_planes = planes;
        }
    }

    fn both_planes(&self) -> bool {
        if let Some(fb) = &self.framebuffer {
            fb.selected_planes == Self::BOTH
        } else {
            false
        }
    }

    fn scroll_up(&mut self, n: u8) {
        let selected_planes = self.selected_planes();
        let to_scroll = self.width_in_u8() * n as usize;
        let (plane0, plane1) = self.framebuffer_mut();
        let len = plane0.len();

        if selected_planes & 0b01 != 0 {
            plane0.copy_within(to_scroll..len, 0);
            plane0[len - to_scroll..].fill(0);
        }
        if selected_planes & 0b10 != 0 {
            plane1.copy_within(to_scroll..len, 0);
            plane1[len - to_scroll..].fill(0);
        }
    }
}

impl XoAudio for Peripheral {
    fn pattern(&mut self, pattern: [u8; 16]) {
        self.beeper.pattern(pattern);
    }

    fn pitch(&mut self, pitch: u8) {
        self.beeper.pitch(pitch);
    }
}

#[cfg(test)]
mod peripheral_tests {
    use super::*;

    #[test]
    fn clear_only_framebuffer() {
        let mut p = Peripheral::new(true, true, vec![0xff; MEMORY_SIZE]);

        p.clear();

        assert_eq!(
            p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END],
            [0; FRAMEBUFFER_END - FRAMEBUFFER_START]
        );
        assert_eq!(p.memory[..FRAMEBUFFER_START], [0xff; FRAMEBUFFER_START]);
    }

    #[test]
    fn clear_only_framebuffer_internal() {
        let mut p = Peripheral::new(false, true, vec![0xff; MEMORY_SIZE]);

        p.clear();

        assert_eq!(p.framebuffer().0, [0; FRAMEBUFFER_SIZE]);
        assert_eq!(p.memory[..], [0xff; MEMORY_SIZE]);
    }

    #[test]
    fn draw_inside_with_one_line_aligned() {
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

        assert!(!p.draw(0, 0, &[0xff]));

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START], 0xff);
        assert_eq!(
            p.memory[FRAMEBUFFER_START + 1..],
            [0; MEMORY_SIZE - FRAMEBUFFER_START - 1]
        );
    }

    #[test]
    fn draw_inside_with_one_line_not_aligned() {
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

        assert!(!p.draw(1, 0, &[0xff]));

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START], 0x7f);
        assert_eq!(p.memory[FRAMEBUFFER_START + 1], 0x80);
        assert_eq!(
            p.memory[FRAMEBUFFER_START + 2..],
            [0; MEMORY_SIZE - FRAMEBUFFER_START - 2]
        );
    }

    #[test]
    fn draw_inside_with_one_line_not_aligned_large() {
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

        assert!(!p.draw(6, 0, &[0xff]));

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START], 0x03);
        assert_eq!(p.memory[FRAMEBUFFER_START + 1], 0xfc);
        assert_eq!(
            p.memory[FRAMEBUFFER_START + 2..],
            [0; MEMORY_SIZE - FRAMEBUFFER_START - 2]
        );
    }

    #[test]
    fn draw_inside_with_several_lines_aligned() {
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

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
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

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
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

        assert!(!p.draw(32, 30, &[0xde, 0xad, 0xbe, 0xef]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[30 * 8 + 4] = 0xde;
        expected[31 * 8 + 4] = 0xad;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_wraps_sprite_at_bottom() {
        let mut p = Peripheral::new(true, false, vec![0; MEMORY_SIZE]);

        assert!(!p.draw(32, 30, &[0xde, 0xad, 0xbe, 0xef]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[30 * 8 + 4] = 0xde;
        expected[31 * 8 + 4] = 0xad;

        expected[4] = 0xbe;
        expected[8 + 4] = 0xef;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_clips_sprite_at_right() {
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

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
    fn draw_wraps_sprite_at_right() {
        let mut p = Peripheral::new(true, false, vec![0; MEMORY_SIZE]);

        assert!(!p.draw(60, 16, &[0xde, 0xad, 0xbe, 0xef]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[16 * 8 + 7] = 0x0d;
        expected[17 * 8 + 7] = 0x0a;
        expected[18 * 8 + 7] = 0x0b;
        expected[19 * 8 + 7] = 0x0e;

        expected[16 * 8] = 0xe0;
        expected[17 * 8] = 0xd0;
        expected[18 * 8] = 0xe0;
        expected[19 * 8] = 0xf0;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_xor_report_collision() {
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

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
        let mut p = Peripheral::new(true, true, vec![0; MEMORY_SIZE]);

        assert!(!p.draw(0, 0, &[0xff]));
        assert!(p.draw(5, 0, &[0x81]));

        let mut expected = [0; FRAMEBUFFER_END - FRAMEBUFFER_START];
        expected[0] = 0xfb;
        expected[1] = 0x08;

        assert_eq!(p.memory[..FRAMEBUFFER_START], [0x0; FRAMEBUFFER_START]);
        assert_eq!(p.memory[FRAMEBUFFER_START..FRAMEBUFFER_END], expected);
    }

    #[test]
    fn draw_plane0() {
        let mut p = Peripheral::new(false, true, vec![]);

        assert_eq!(p.selected_planes(), Peripheral::PLANE0);

        assert!(!p.draw(32, 16, &[0xde, 0xad]));

        let mut expected = [0; FRAMEBUFFER_SIZE];

        expected[16 * 8 + 4] = 0xde;
        expected[17 * 8 + 4] = 0xad;

        let (plane0, plane1) = p.framebuffer();

        assert_eq!(plane0, expected);
        assert_eq!(plane1, [0; FRAMEBUFFER_SIZE]);
    }

    #[test]
    fn draw_plane1() {
        let mut p = Peripheral::new(false, true, vec![]);

        p.planes(Peripheral::PLANE1);
        assert_eq!(p.selected_planes(), Peripheral::PLANE1);

        assert!(!p.draw(32, 16, &[0xde, 0xad]));

        let mut expected = [0; FRAMEBUFFER_SIZE];

        expected[16 * 8 + 4] = 0xde;
        expected[17 * 8 + 4] = 0xad;

        let (plane0, plane1) = p.framebuffer();

        assert_eq!(plane0, [0; FRAMEBUFFER_SIZE]);
        assert_eq!(plane1, expected);
    }
}
