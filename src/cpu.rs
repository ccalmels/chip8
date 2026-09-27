use crate::Error;
use std::ops::{Index, IndexMut};

#[derive(Copy, Clone, PartialEq, Debug)]
#[must_use]
pub struct Address(u16);

impl Address {
    pub const fn new(value: u16) -> Self {
        Address(value & 0xfff)
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub fn wrapping_add(self, offset: u16) -> Self {
        Address::new(self.0.wrapping_add(offset))
    }

    pub fn wrapping_sub(self, offset: u16) -> Self {
        Address::new(self.0.wrapping_sub(offset))
    }
}

#[cfg(test)]
mod address_tests {
    use super::*;

    #[test]
    fn new_masks_value_to_12_bits() {
        assert_eq!(Address::new(0x200).value(), 0x200);
        assert_eq!(Address::new(0x1000).value(), 0x000);
        assert_eq!(Address::new(0xf000).value(), 0x000);
        assert_eq!(Address::new(0xffff).value(), 0xfff);
    }

    #[test]
    fn wrapping_add_wraps_at_max() {
        assert_eq!(Address::new(0xfff).wrapping_add(1).value(), 0x000);
    }

    #[test]
    fn wrapping_add_continue_after_wrap() {
        assert_eq!(Address::new(0xfff).wrapping_add(2).value(), 0x001);
    }

    #[test]
    fn wrapping_add_large_offsets() {
        assert_eq!(Address::new(0x001).wrapping_add(0xfff).value(), 0x000);
        assert_eq!(Address::new(0x002).wrapping_add(0xfff).value(), 0x001);
        assert_eq!(Address::new(0xfff).wrapping_add(0xfff).value(), 0xffe);
    }

    #[test]
    fn wrapping_sub_at_min() {
        assert_eq!(Address::new(0x0).wrapping_sub(1).value(), 0xfff);
    }

    #[test]
    fn wrapping_sub_after_wrap() {
        assert_eq!(Address::new(0x0).wrapping_sub(2).value(), 0xffe);
    }

    #[test]
    fn wrapping_sub_larg_offset() {
        assert_eq!(Address::new(0xfff).wrapping_sub(0xfff).value(), 0x0);
        assert_eq!(Address::new(0xffe).wrapping_sub(0xfff).value(), 0xfff);
        assert_eq!(Address::new(0x0).wrapping_sub(0xfff).value(), 0x1);
    }
}

struct Registers([u8; 16]);

impl Index<u8> for Registers {
    type Output = u8;

    fn index(&self, index: u8) -> &u8 {
        &self.0[index as usize]
    }
}

impl IndexMut<u8> for Registers {
    fn index_mut(&mut self, index: u8) -> &mut u8 {
        &mut self.0[index as usize]
    }
}

#[cfg(test)]
mod registers_tests {
    use super::*;

    #[test]
    fn read_right_slot() {
        let rs = Registers(core::array::from_fn(|i| i as u8));

        for i in 0..16 {
            assert_eq!(rs[i], i);
        }
    }

    #[test]
    fn read_right_mut_slot() {
        let mut rs = Registers(core::array::from_fn(|i| i as u8));

        for i in 0..16 {
            rs[i] -= i;
        }

        assert_eq!(rs.0, [0; 16]);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn out_of_bounds() {
        let rs = Registers([0; 16]);
        let _ = rs[42];
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn out_of_bounds_mut() {
        let mut rs = Registers([0; 16]);

        rs[42] = 1;
    }
}

pub trait Memory {
    fn read(&self, address: Address) -> u8;
    fn write(&mut self, address: Address, value: u8);
}

pub trait Display {
    fn clear(&mut self);
    fn draw(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool;
}

pub trait Keypad {
    fn key_pressed(&self, key: u8) -> bool;
    fn wait_key(&mut self) -> Option<u8>;
}

pub trait Beeper {
    fn beep(&self, running: bool);
}

pub trait Rng {
    fn next(&self) -> u8;
}

pub trait SuperDisplay {
    fn switch_resolution(&mut self, hi_res: bool);
    fn scroll_down(&mut self, n: u8);
    fn scroll_right(&mut self);
    fn scroll_left(&mut self);
}

fn opcode_split(opcode: u16) -> ((u8, u8, u8, u8), u8, u16) {
    let nibbles = (
        (opcode >> 12) as u8,
        ((opcode >> 8) & 0xf) as u8,
        ((opcode >> 4) & 0xf) as u8,
        (opcode & 0xf) as u8,
    );
    let nn = (opcode & 0xff) as u8;
    let nnn = opcode & 0xfff;

    (nibbles, nn, nnn)
}

#[derive(PartialEq, Debug)]
enum OpCode {
    Clear,
    Return,
    Jump(Address),
    Call(Address),
    SkipEqual(u8, u8),
    SkipNotEqual(u8, u8),
    SkipRegistersEqual(u8, u8),
    Load(u8, u8),
    Inc(u8, u8),
    Set(u8, u8),
    Or(u8, u8),
    And(u8, u8),
    Xor(u8, u8),
    Add(u8, u8),
    Sub(u8, u8),
    RShift(u8, u8),
    InvSub(u8, u8),
    LShift(u8, u8),
    SkipRegistersNotEqual(u8, u8),
    LoadIndex(Address),
    Flow(Address),
    Rand(u8, u8),
    Draw(u8, u8, u8),
    IsKey(u8),
    IsNotKey(u8),
    SetDelay(u8),
    WaitKey(u8),
    Delay(u8),
    Sound(u8),
    IncIndex(u8),
    Sprite(u8),
    Bcd(u8),
    Write(u8),
    Read(u8),
}

fn opcode_match((nibbles, nn, nnn): ((u8, u8, u8, u8), u8, u16)) -> Option<OpCode> {
    match nibbles {
        (0x0, 0x0, 0xe, 0x0) => Some(OpCode::Clear),
        (0x0, 0x0, 0xe, 0xe) => Some(OpCode::Return),
        (0x1, _, _, _) => Some(OpCode::Jump(Address::new(nnn))),
        (0x2, _, _, _) => Some(OpCode::Call(Address::new(nnn))),
        (0x3, x, _, _) => Some(OpCode::SkipEqual(x, nn)),
        (0x4, x, _, _) => Some(OpCode::SkipNotEqual(x, nn)),
        (0x5, x, y, 0) => Some(OpCode::SkipRegistersEqual(x, y)),
        (0x6, x, _, _) => Some(OpCode::Load(x, nn)),
        (0x7, x, _, _) => Some(OpCode::Inc(x, nn)),
        (0x8, x, y, 0) => Some(OpCode::Set(x, y)),
        (0x8, x, y, 1) => Some(OpCode::Or(x, y)),
        (0x8, x, y, 2) => Some(OpCode::And(x, y)),
        (0x8, x, y, 3) => Some(OpCode::Xor(x, y)),
        (0x8, x, y, 4) => Some(OpCode::Add(x, y)),
        (0x8, x, y, 5) => Some(OpCode::Sub(x, y)),
        (0x8, x, y, 6) => Some(OpCode::RShift(x, y)),
        (0x8, x, y, 7) => Some(OpCode::InvSub(x, y)),
        (0x8, x, y, 0xe) => Some(OpCode::LShift(x, y)),
        (0x9, x, y, 0) => Some(OpCode::SkipRegistersNotEqual(x, y)),
        (0xa, _, _, _) => Some(OpCode::LoadIndex(Address::new(nnn))),
        (0xb, _, _, _) => Some(OpCode::Flow(Address::new(nnn))),
        (0xc, x, _, _) => Some(OpCode::Rand(x, nn)),
        (0xd, x, y, n) => Some(OpCode::Draw(x, y, n)),
        (0xe, x, 9, 0xe) => Some(OpCode::IsKey(x)),
        (0xe, x, 0xa, 1) => Some(OpCode::IsNotKey(x)),
        (0xf, x, 0, 7) => Some(OpCode::SetDelay(x)),
        (0xf, x, 0, 0xa) => Some(OpCode::WaitKey(x)),
        (0xf, x, 1, 5) => Some(OpCode::Delay(x)),
        (0xf, x, 1, 8) => Some(OpCode::Sound(x)),
        (0xf, x, 1, 0xe) => Some(OpCode::IncIndex(x)),
        (0xf, x, 2, 9) => Some(OpCode::Sprite(x)),
        (0xf, x, 3, 3) => Some(OpCode::Bcd(x)),
        (0xf, x, 5, 5) => Some(OpCode::Write(x)),
        (0xf, x, 6, 5) => Some(OpCode::Read(x)),
        _ => None,
    }
}

fn decode(opcode: u16) -> Result<OpCode, Error> {
    opcode_match(opcode_split(opcode)).ok_or(Error::UnknownOpCode(opcode))
}

#[cfg(test)]
mod decode_tests {
    use super::*;

    #[test]
    fn decode_unknown_opcode() {
        assert_eq!(decode(0x9999), Err(Error::UnknownOpCode(0x9999)));
        assert_eq!(decode(0x0fff), Err(Error::UnknownOpCode(0x0fff)));
    }

    #[test]
    fn decode_clear() {
        assert_eq!(decode(0x00e0), Ok(OpCode::Clear));
    }

    #[test]
    fn decode_jump() {
        assert_eq!(decode(0x1123), Ok(OpCode::Jump(Address::new(0x123))));
        assert_eq!(decode(0x1fed), Ok(OpCode::Jump(Address::new(0xfed))));
    }

    #[test]
    fn decode_call() {
        assert_eq!(decode(0x2123), Ok(OpCode::Call(Address::new(0x123))));
        assert_eq!(decode(0x2fed), Ok(OpCode::Call(Address::new(0xfed))));
    }
    #[test]
    fn decode_skip_equal() {
        assert_eq!(decode(0x3123), Ok(OpCode::SkipEqual(0x1, 0x23)));
        assert_eq!(decode(0x3fed), Ok(OpCode::SkipEqual(0xf, 0xed)));
    }

    #[test]
    fn decode_skip_not_equal() {
        assert_eq!(decode(0x4123), Ok(OpCode::SkipNotEqual(0x1, 0x23)));
        assert_eq!(decode(0x4fed), Ok(OpCode::SkipNotEqual(0xf, 0xed)));
    }

    #[test]
    fn decode_load() {
        assert_eq!(decode(0x6123), Ok(OpCode::Load(0x1, 0x23)));
        assert_eq!(decode(0x6fed), Ok(OpCode::Load(0xf, 0xed)));
    }

    #[test]
    fn decode_inc() {
        assert_eq!(decode(0x7123), Ok(OpCode::Inc(0x1, 0x23)));
        assert_eq!(decode(0x7fed), Ok(OpCode::Inc(0xf, 0xed)));
    }

    #[test]
    fn decode_skip_registers_not_equal() {
        assert_eq!(decode(0x9120), Ok(OpCode::SkipRegistersNotEqual(0x1, 0x2)));
        assert_eq!(decode(0x9fe0), Ok(OpCode::SkipRegistersNotEqual(0xf, 0xe)));
    }

    #[test]
    fn decode_set() {
        assert_eq!(decode(0x8120), Ok(OpCode::Set(0x1, 0x2)));
        assert_eq!(decode(0x8fe0), Ok(OpCode::Set(0xf, 0xe)));
    }

    #[test]
    fn decode_or() {
        assert_eq!(decode(0x8121), Ok(OpCode::Or(0x1, 0x2)));
        assert_eq!(decode(0x8fe1), Ok(OpCode::Or(0xf, 0xe)));
    }

    #[test]
    fn decode_and() {
        assert_eq!(decode(0x8122), Ok(OpCode::And(0x1, 0x2)));
        assert_eq!(decode(0x8fe2), Ok(OpCode::And(0xf, 0xe)));
    }

    #[test]
    fn decode_xor() {
        assert_eq!(decode(0x8123), Ok(OpCode::Xor(0x1, 0x2)));
        assert_eq!(decode(0x8fe3), Ok(OpCode::Xor(0xf, 0xe)));
    }

    #[test]
    fn decode_add() {
        assert_eq!(decode(0x8124), Ok(OpCode::Add(0x1, 0x2)));
        assert_eq!(decode(0x8fe4), Ok(OpCode::Add(0xf, 0xe)));
    }

    #[test]
    fn decode_sub() {
        assert_eq!(decode(0x8125), Ok(OpCode::Sub(0x1, 0x2)));
        assert_eq!(decode(0x8fe5), Ok(OpCode::Sub(0xf, 0xe)));
    }

    #[test]
    fn decode_rshift() {
        assert_eq!(decode(0x8126), Ok(OpCode::RShift(0x1, 0x2)));
        assert_eq!(decode(0x8fe6), Ok(OpCode::RShift(0xf, 0xe)));
    }

    #[test]
    fn decode_inv_sub() {
        assert_eq!(decode(0x8127), Ok(OpCode::InvSub(0x1, 0x2)));
        assert_eq!(decode(0x8fe7), Ok(OpCode::InvSub(0xf, 0xe)));
    }

    #[test]
    fn decode_lshift() {
        assert_eq!(decode(0x812e), Ok(OpCode::LShift(0x1, 0x2)));
        assert_eq!(decode(0x8fee), Ok(OpCode::LShift(0xf, 0xe)));
    }

    #[test]
    fn decode_load_index() {
        assert_eq!(decode(0xa123), Ok(OpCode::LoadIndex(Address::new(0x123))));
        assert_eq!(decode(0xafed), Ok(OpCode::LoadIndex(Address::new(0xfed))));
    }

    #[test]
    fn decode_flow() {
        assert_eq!(decode(0xb123), Ok(OpCode::Flow(Address::new(0x123))));
        assert_eq!(decode(0xbfed), Ok(OpCode::Flow(Address::new(0xfed))));
    }

    #[test]
    fn decode_rand() {
        assert_eq!(decode(0xc123), Ok(OpCode::Rand(0x1, 0x23)));
        assert_eq!(decode(0xcfed), Ok(OpCode::Rand(0xf, 0xed)));
    }

    #[test]
    fn decode_draw() {
        assert_eq!(decode(0xd123), Ok(OpCode::Draw(0x1, 0x2, 0x3)));
        assert_eq!(decode(0xdfed), Ok(OpCode::Draw(0xf, 0xe, 0xd)));
    }

    #[test]
    fn decode_is_key() {
        assert_eq!(decode(0xe19e), Ok(OpCode::IsKey(0x1)));
        assert_eq!(decode(0xef9e), Ok(OpCode::IsKey(0xf)));
    }

    #[test]
    fn decode_is_not_key() {
        assert_eq!(decode(0xe1a1), Ok(OpCode::IsNotKey(0x1)));
        assert_eq!(decode(0xefa1), Ok(OpCode::IsNotKey(0xf)));
    }

    #[test]
    fn decode_set_delay() {
        assert_eq!(decode(0xf107), Ok(OpCode::SetDelay(0x1)));
        assert_eq!(decode(0xff07), Ok(OpCode::SetDelay(0xf)));
    }

    #[test]
    fn decode_wait_key() {
        assert_eq!(decode(0xf10a), Ok(OpCode::WaitKey(0x1)));
        assert_eq!(decode(0xff0a), Ok(OpCode::WaitKey(0xf)));
    }

    #[test]
    fn decode_delay() {
        assert_eq!(decode(0xf115), Ok(OpCode::Delay(0x1)));
        assert_eq!(decode(0xff15), Ok(OpCode::Delay(0xf)));
    }

    #[test]
    fn decode_sound() {
        assert_eq!(decode(0xf118), Ok(OpCode::Sound(0x1)));
        assert_eq!(decode(0xff18), Ok(OpCode::Sound(0xf)));
    }

    #[test]
    fn decode_inc_index() {
        assert_eq!(decode(0xf31e), Ok(OpCode::IncIndex(0x3)));
        assert_eq!(decode(0xf51e), Ok(OpCode::IncIndex(0x5)));
    }

    #[test]
    fn decode_sprite() {
        assert_eq!(decode(0xf329), Ok(OpCode::Sprite(0x3)));
        assert_eq!(decode(0xf529), Ok(OpCode::Sprite(0x5)));
    }

    #[test]
    fn decode_bcd() {
        assert_eq!(decode(0xf333), Ok(OpCode::Bcd(0x3)));
        assert_eq!(decode(0xf533), Ok(OpCode::Bcd(0x5)));
    }

    #[test]
    fn decode_write() {
        assert_eq!(decode(0xf355), Ok(OpCode::Write(0x3)));
        assert_eq!(decode(0xf555), Ok(OpCode::Write(0x5)));
    }

    #[test]
    fn decode_read() {
        assert_eq!(decode(0xf365), Ok(OpCode::Read(0x3)));
        assert_eq!(decode(0xf565), Ok(OpCode::Read(0x5)));
    }
}

#[derive(PartialEq, Debug)]
enum SuperOpCode {
    ScrollDown(u8),
    ScrollRight,
    ScrollLeft,
    LowRes,
    HiRes,
    Draw16(u8, u8),
    HiSprite(u8),
    Store(u8),
    Load(u8),
    Chip8(OpCode),
}

fn super_opcode_match((nibbles, nn, nnn): ((u8, u8, u8, u8), u8, u16)) -> Option<SuperOpCode> {
    match nibbles {
        (0x0, 0x0, 0xc, n) => Some(SuperOpCode::ScrollDown(n)),
        (0x0, 0x0, 0xf, 0xb) => Some(SuperOpCode::ScrollRight),
        (0x0, 0x0, 0xf, 0xc) => Some(SuperOpCode::ScrollLeft),
        (0x0, 0x0, 0xf, 0xe) => Some(SuperOpCode::LowRes),
        (0x0, 0x0, 0xf, 0xf) => Some(SuperOpCode::HiRes),
        (0xf, x, 3, 0) => Some(SuperOpCode::HiSprite(x)),
        (0xf, x, 7, 5) => Some(SuperOpCode::Store(x)),
        (0xf, x, 8, 5) => Some(SuperOpCode::Load(x)),
        (0xd, x, y, 0) => Some(SuperOpCode::Draw16(x, y)),
        _ => opcode_match((nibbles, nn, nnn)).map(SuperOpCode::Chip8),
    }
}

fn super_decode(opcode: u16) -> Result<SuperOpCode, Error> {
    super_opcode_match(opcode_split(opcode)).ok_or(Error::UnknownOpCode(opcode))
}

#[cfg(test)]
mod super_decode_tests {
    use super::*;

    #[test]
    fn super_decode_unknown_opcode() {
        assert_eq!(super_decode(0x9999), Err(Error::UnknownOpCode(0x9999)));
        assert_eq!(super_decode(0x0fff), Err(Error::UnknownOpCode(0x0fff)));
    }

    #[test]
    fn super_decode_classic_opcode() {
        assert_eq!(super_decode(0x00e0), Ok(SuperOpCode::Chip8(OpCode::Clear)));
        assert_eq!(
            super_decode(0x51c0),
            Ok(SuperOpCode::Chip8(OpCode::SkipRegistersEqual(1, 0xc)))
        );
    }

    #[test]
    fn super_decode_scroll_down() {
        assert_eq!(super_decode(0x00c2), Ok(SuperOpCode::ScrollDown(2)));
        assert_eq!(super_decode(0x00ca), Ok(SuperOpCode::ScrollDown(0xa)));
    }

    #[test]
    fn super_decode_scroll_right() {
        assert_eq!(super_decode(0x00fb), Ok(SuperOpCode::ScrollRight));
    }

    #[test]
    fn super_decode_scroll_left() {
        assert_eq!(super_decode(0x00fc), Ok(SuperOpCode::ScrollLeft));
    }

    #[test]
    fn super_decode_hires_lowres() {
        assert_eq!(super_decode(0x00fe), Ok(SuperOpCode::LowRes));
        assert_eq!(super_decode(0x00ff), Ok(SuperOpCode::HiRes));
    }

    #[test]
    fn super_decode_draw16() {
        assert_eq!(super_decode(0xd1c0), Ok(SuperOpCode::Draw16(1, 0xc)));
        assert_eq!(super_decode(0xda30), Ok(SuperOpCode::Draw16(0xa, 3)));
    }

    #[test]
    fn super_decode_hisprite() {
        assert_eq!(super_decode(0xf230), Ok(SuperOpCode::HiSprite(2)));
        assert_eq!(super_decode(0xfa30), Ok(SuperOpCode::HiSprite(0xa)));
    }

    #[test]
    fn super_decode_store() {
        assert_eq!(super_decode(0xf275), Ok(SuperOpCode::Store(2)));
        assert_eq!(super_decode(0xfa75), Ok(SuperOpCode::Store(0xa)));
    }

    #[test]
    fn super_decode_load() {
        assert_eq!(super_decode(0xf285), Ok(SuperOpCode::Load(2)));
        assert_eq!(super_decode(0xfa85), Ok(SuperOpCode::Load(0xa)));
    }
}

fn fetch<M: Memory>(memory: &M, pc: Address) -> u16 {
    let high = memory.read(pc);
    let low = memory.read(pc.wrapping_add(1));

    (high as u16) << 8 | low as u16
}

#[cfg(test)]
mod fetch_tests {
    use super::*;

    pub struct FakeMemory(pub Vec<u8>);

    impl Memory for FakeMemory {
        fn read(&self, address: Address) -> u8 {
            self.0[address.value() as usize]
        }

        fn write(&mut self, address: Address, value: u8) {
            self.0[address.value() as usize] = value;
        }
    }

    #[test]
    fn fetch_opcode() {
        let memory = FakeMemory(vec![0xde, 0xad, 0xbe, 0xef]);

        assert_eq!(fetch(&memory, Address::new(0)), 0xdead);
        assert_eq!(fetch(&memory, Address::new(2)), 0xbeef);
    }
}

#[derive(Debug, PartialEq)]
enum Stack {
    Memory(Address),
    Internal(Vec<Address>),
}

impl Stack {
    fn push<M: Memory>(&mut self, memory: &mut M, address: Address) {
        match self {
            Stack::Memory(sp) => {
                let (high, low) = ((address.value() >> 8) as u8, (address.value() & 0xff) as u8);

                memory.write(*sp, high);
                *sp = sp.wrapping_sub(1);

                memory.write(*sp, low);
                *sp = sp.wrapping_sub(1);
            }
            Stack::Internal(stack) => stack.push(address),
        }
    }

    fn pop<M: Memory>(&mut self, memory: &M) -> Address {
        match self {
            Stack::Memory(sp) => {
                *sp = sp.wrapping_add(1);
                let low = memory.read(*sp);

                *sp = sp.wrapping_add(1);
                let high = memory.read(*sp);

                Address::new((high as u16) << 8 | low as u16)
            }
            Stack::Internal(stack) => stack.pop().unwrap_or(Address::new(0)),
        }
    }
}

#[cfg(test)]
mod stack_tests {
    use super::*;

    struct MockMemory([u8; 8]);

    impl Memory for MockMemory {
        fn read(&self, address: Address) -> u8 {
            self.0[address.value() as usize]
        }

        fn write(&mut self, address: Address, value: u8) {
            self.0[address.value() as usize] = value;
        }
    }

    #[test]
    fn push_pop_stack_memory() {
        let mut stack = Stack::Memory(Address::new(7));
        let mut memory = MockMemory([0; 8]);

        let expected = [0, 0, 0, 0, 0x23, 0x1, 0xbc, 0xa];

        stack.push(&mut memory, Address::new(0xabc));
        assert_eq!(stack, Stack::Memory(Address::new(5)));
        stack.push(&mut memory, Address::new(0x123));
        assert_eq!(stack, Stack::Memory(Address::new(3)));

        assert_eq!(memory.0, expected);

        let expected = [0, 0, 0, 0, 0x54, 0x6, 0xbc, 0xa];

        assert_eq!(stack.pop(&memory), Address::new(0x123));
        assert_eq!(stack, Stack::Memory(Address::new(5)));

        stack.push(&mut memory, Address::new(0x654));
        assert_eq!(memory.0, expected);
        assert_eq!(stack, Stack::Memory(Address::new(3)));

        assert_eq!(stack.pop(&memory), Address::new(0x654));
        assert_eq!(stack, Stack::Memory(Address::new(5)));
        assert_eq!(stack.pop(&memory), Address::new(0xabc));
        assert_eq!(stack, Stack::Memory(Address::new(7)));
    }

    #[test]
    fn push_pop_stack_internal() {
        let mut stack = Stack::Internal(vec![]);
        let mut memory = MockMemory([0; 8]);

        stack.push(&mut memory, Address::new(0xabc));
        assert_eq!(stack, Stack::Internal(vec![Address::new(0xabc)]));

        stack.push(&mut memory, Address::new(0x123));
        assert_eq!(
            stack,
            Stack::Internal(vec![Address::new(0xabc), Address::new(0x123)])
        );

        assert_eq!(stack.pop(&memory), Address::new(0x123));
        assert_eq!(stack, Stack::Internal(vec![Address::new(0xabc)]));

        stack.push(&mut memory, Address::new(0x654));

        assert_eq!(
            stack,
            Stack::Internal(vec![Address::new(0xabc), Address::new(0x654)])
        );

        assert_eq!(stack.pop(&memory), Address::new(0x654));
        assert_eq!(stack, Stack::Internal(vec![Address::new(0xabc)]));

        assert_eq!(stack.pop(&memory), Address::new(0xabc));
        assert_eq!(stack, Stack::Internal(vec![]));
    }
}

pub struct Quirks {
    vf_reset: bool,
    memory: bool,
    shifting: bool,
    jumping: bool,
}

impl Quirks {
    pub fn classic() -> Self {
        Self {
            vf_reset: true,
            memory: true,
            shifting: false,
            jumping: false,
        }
    }

    pub fn schip() -> Self {
        Self {
            vf_reset: false,
            memory: false,
            shifting: true,
            jumping: true,
        }
    }
}

pub struct Cpu {
    pc: Address,
    stack: Stack,
    i: Address,
    vs: Registers,
    dt: u8, // delay timer
    st: u8, // sound timer
    quirks: Quirks,
    storage: [u8; 16],
}

impl Cpu {
    pub const START_PC: Address = Address::new(0x200);
    pub const FONTS: Address = Address::new(0x050);
    const FONTS_SIZE: u16 = 0x50;
    const HI_FONTS: Address = Address::new(0x050 + Self::FONTS_SIZE);
    const SP: Address = Address::new(0xeff);

    pub fn new(stack_in_memory: bool, quirks: Quirks) -> Self {
        Cpu {
            pc: Cpu::START_PC,
            stack: if stack_in_memory {
                Stack::Memory(Self::SP)
            } else {
                Stack::Internal(vec![])
            },
            i: Address::new(0),
            vs: Registers([0; 16]),
            dt: 0,
            st: 0,
            quirks,
            storage: [0; 16],
        }
    }

    pub fn tick_timer<B: Beeper>(&mut self, beeper: &B) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        if self.st > 0 {
            self.st -= 1;
        }

        beeper.beep(self.st != 0);
    }

    fn execute<P: Memory + Display + Keypad, R: Rng>(
        &mut self,
        peripheral: &mut P,
        rng: &R,
        opcode: OpCode,
    ) {
        match opcode {
            OpCode::Clear => peripheral.clear(),
            OpCode::Return => self.pc = self.stack.pop(peripheral),
            OpCode::Jump(address) => self.pc = address,
            OpCode::Call(address) => {
                self.stack.push(peripheral, self.pc);
                self.pc = address
            }
            OpCode::SkipEqual(v, value) => {
                if self.vs[v] == value {
                    self.pc = self.pc.wrapping_add(2)
                }
            }
            OpCode::SkipNotEqual(v, value) => {
                if self.vs[v] != value {
                    self.pc = self.pc.wrapping_add(2)
                }
            }
            OpCode::SkipRegistersEqual(x, y) => {
                if self.vs[x] == self.vs[y] {
                    self.pc = self.pc.wrapping_add(2)
                }
            }
            OpCode::Load(v, value) => self.vs[v] = value,
            OpCode::Inc(v, value) => self.vs[v] = self.vs[v].wrapping_add(value),
            OpCode::Set(x, y) => self.vs[x] = self.vs[y],
            OpCode::Or(x, y) => {
                self.vs[x] |= self.vs[y];
                if self.quirks.vf_reset {
                    self.vs[0xf] = 0
                }
            }
            OpCode::And(x, y) => {
                self.vs[x] &= self.vs[y];
                if self.quirks.vf_reset {
                    self.vs[0xf] = 0
                }
            }
            OpCode::Xor(x, y) => {
                self.vs[x] ^= self.vs[y];
                if self.quirks.vf_reset {
                    self.vs[0xf] = 0
                }
            }
            OpCode::Add(x, y) => {
                let (res, overflowed) = self.vs[x].overflowing_add(self.vs[y]);
                self.vs[x] = res;
                self.vs[0xf] = overflowed as u8;
            }
            OpCode::Sub(x, y) => {
                let (res, overflowed) = self.vs[x].overflowing_sub(self.vs[y]);
                self.vs[x] = res;
                self.vs[0xf] = !overflowed as u8;
            }
            OpCode::RShift(x, y) => {
                let lsb = self.vs[y] & 0x1;

                if self.quirks.shifting {
                    self.vs[x] >>= 1;
                } else {
                    self.vs[x] = self.vs[y] >> 1;
                }

                self.vs[0xf] = lsb;
            }
            OpCode::InvSub(x, y) => {
                let (res, overflowed) = self.vs[y].overflowing_sub(self.vs[x]);
                self.vs[x] = res;
                self.vs[0xf] = !overflowed as u8;
            }
            OpCode::LShift(x, y) => {
                let msb = self.vs[y] >> 7;

                if self.quirks.shifting {
                    self.vs[x] <<= 1;
                } else {
                    self.vs[x] = self.vs[y] << 1;
                }

                self.vs[0xf] = msb;
            }
            OpCode::SkipRegistersNotEqual(x, y) => {
                if self.vs[x] != self.vs[y] {
                    self.pc = self.pc.wrapping_add(2)
                }
            }
            OpCode::LoadIndex(addr) => self.i = addr,
            OpCode::Flow(addr) => {
                if self.quirks.jumping {
                    let x = (addr.value() >> 8) as u8;

                    self.pc = addr.wrapping_add(self.vs[x] as u16)
                } else {
                    self.pc = addr.wrapping_add(self.vs[0] as u16)
                }
            }
            OpCode::Rand(x, nn) => self.vs[x] = rng.next() & nn,
            OpCode::Draw(x, y, n) => {
                let mut sprite = [0u8; 15];

                for (row, byte) in sprite.iter_mut().enumerate().take(n as usize) {
                    *byte = peripheral.read(self.i.wrapping_add(row as u16));
                }

                self.vs[0xf] = peripheral.draw(self.vs[x], self.vs[y], &sprite[..n as usize]) as u8;
            }
            OpCode::IsKey(x) => {
                if peripheral.key_pressed(self.vs[x] & 0xf) {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            OpCode::IsNotKey(x) => {
                if !peripheral.key_pressed(self.vs[x] & 0xf) {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            OpCode::SetDelay(x) => self.vs[x] = self.dt,
            OpCode::WaitKey(x) => {
                if let Some(k) = peripheral.wait_key() {
                    self.vs[x] = k;
                } else {
                    self.pc = self.pc.wrapping_sub(2);
                }
            }
            OpCode::Delay(x) => self.dt = self.vs[x],
            OpCode::Sound(x) => self.st = self.vs[x],
            OpCode::IncIndex(x) => self.i = self.i.wrapping_add(self.vs[x] as u16),
            OpCode::Sprite(x) => self.i = Self::FONTS.wrapping_add((self.vs[x] as u16 & 0xf) * 5),
            OpCode::Bcd(x) => {
                let a = self.vs[x] / 100;
                let b = (self.vs[x] / 10) % 10;
                let c = self.vs[x] % 10;
                let mut destination = self.i;

                peripheral.write(destination, a);
                destination = destination.wrapping_add(1);
                peripheral.write(destination, b);
                destination = destination.wrapping_add(1);
                peripheral.write(destination, c);
            }
            OpCode::Write(x) => {
                let mut dest = self.i;

                for i in 0..=x {
                    peripheral.write(dest, self.vs[i]);
                    dest = dest.wrapping_add(1);
                }

                if self.quirks.memory {
                    self.i = dest;
                }
            }
            OpCode::Read(x) => {
                let mut src = self.i;

                for i in 0..=x {
                    self.vs[i] = peripheral.read(src);
                    src = src.wrapping_add(1);
                }

                if self.quirks.memory {
                    self.i = src;
                }
            }
        }
    }

    fn super_execute<P: Memory + Display + Keypad + SuperDisplay, R: Rng>(
        &mut self,
        peripheral: &mut P,
        rng: &R,
        opcode: SuperOpCode,
    ) {
        match opcode {
            SuperOpCode::ScrollDown(n) => peripheral.scroll_down(n),
            SuperOpCode::ScrollRight => peripheral.scroll_right(),
            SuperOpCode::ScrollLeft => peripheral.scroll_left(),
            SuperOpCode::LowRes => peripheral.switch_resolution(false),
            SuperOpCode::HiRes => peripheral.switch_resolution(true),
            SuperOpCode::Draw16(x, y) => {
                let mut sprite = [0u8; 16];

                for (row, byte) in sprite.iter_mut().enumerate() {
                    *byte = peripheral.read(self.i.wrapping_add(2 * row as u16));
                }

                self.vs[0xf] = peripheral.draw(self.vs[x], self.vs[y], &sprite) as u8;

                for (row, byte) in sprite.iter_mut().enumerate() {
                    *byte = peripheral.read(self.i.wrapping_add(1 + 2 * row as u16));
                }

                self.vs[0xf] |= peripheral.draw(self.vs[x] + 8, self.vs[y], &sprite) as u8;
            }
            SuperOpCode::HiSprite(x) => {
                self.i = Self::HI_FONTS.wrapping_add((self.vs[x] as u16 & 0xf) * 10)
            }
            SuperOpCode::Store(x) => {
                for i in 0..=x {
                    self.storage[i as usize] = self.vs[i]
                }
            }
            SuperOpCode::Load(x) => {
                for i in 0..=x {
                    self.vs[i] = self.storage[i as usize]
                }
            }
            SuperOpCode::Chip8(opcode) => self.execute(peripheral, rng, opcode),
        }
    }

    pub fn step<P: Memory + Display + Keypad, R: Rng>(
        &mut self,
        peripheral: &mut P,
        rng: &R,
    ) -> Result<(), crate::Error> {
        let opcode = fetch(peripheral, self.pc);
        let opcode = decode(opcode)?;

        self.pc = self.pc.wrapping_add(2);

        self.execute(peripheral, rng, opcode);

        Ok(())
    }

    pub fn super_step<P: Memory + Display + Keypad + SuperDisplay, R: Rng>(
        &mut self,
        peripheral: &mut P,
        rng: &R,
    ) -> Result<(), crate::Error> {
        let opcode = fetch(peripheral, self.pc);
        let opcode = super_decode(opcode)?;

        self.pc = self.pc.wrapping_add(2);

        self.super_execute(peripheral, rng, opcode);

        Ok(())
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new(true, Quirks::classic())
    }
}

#[cfg(test)]
mod execute_tests {
    use super::*;

    struct MockPeripheral {
        memory: [u8; 16],
        cleared: bool,
        last_draw: Option<(u8, u8, Vec<u8>)>,
        draw_returns: bool,
        pressed: u16,
        waited_key: Option<u8>,
    }

    impl MockPeripheral {
        fn new() -> Self {
            MockPeripheral {
                memory: [0; 16],
                cleared: false,
                last_draw: None,
                draw_returns: false,
                pressed: 0,
                waited_key: None,
            }
        }
    }

    impl Memory for MockPeripheral {
        fn read(&self, address: Address) -> u8 {
            self.memory[address.value() as usize]
        }

        fn write(&mut self, address: Address, value: u8) {
            self.memory[address.value() as usize] = value;
        }
    }

    impl Display for MockPeripheral {
        fn clear(&mut self) {
            self.cleared = true;
        }

        fn draw(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool {
            self.last_draw = Some((x, y, sprite.to_vec()));
            self.draw_returns
        }
    }

    impl Keypad for MockPeripheral {
        fn key_pressed(&self, key: u8) -> bool {
            self.pressed & (1 << key) != 0
        }

        fn wait_key(&mut self) -> Option<u8> {
            self.waited_key
        }
    }

    struct FixedRng(u8);

    impl Rng for FixedRng {
        fn next(&self) -> u8 {
            self.0
        }
    }

    #[test]
    fn execute_rand() {
        let mut cpu = Cpu::default();
        let mut p = MockPeripheral::new();
        let rng = FixedRng(0xff);

        cpu.execute(&mut p, &rng, OpCode::Rand(2, 0x0));

        assert_eq!(cpu.vs[2], 0);

        cpu.execute(&mut p, &rng, OpCode::Rand(3, 0xa2));

        assert_eq!(cpu.vs[3], 0xa2);

        let rng = FixedRng(0xf0);

        cpu.execute(&mut p, &rng, OpCode::Rand(4, 0xa2));

        assert_eq!(cpu.vs[4], 0xa0);
    }
}
