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
    fn wait_key(&self) -> Option<u8>;
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
    Draw(u8, u8, u8),
    IsKey(u8),
    IsNotKey(u8),
    SetDelay(u8),
    WaitKey(u8),
    Delay(u8),
    Sound(u8),
    IncIndex(u8),
    Bcd(u8),
    Write(u8),
    Read(u8),
}

fn decode(opcode: u16) -> Result<OpCode, Error> {
    let nibbles = (
        (opcode >> 12) as u8,
        ((opcode >> 8) & 0xf) as u8,
        ((opcode >> 4) & 0xf) as u8,
        (opcode & 0xf) as u8,
    );
    let nn = (opcode & 0xff) as u8;
    let nnn = opcode & 0xfff;

    match nibbles {
        (0x0, 0x0, 0xe, 0x0) => Ok(OpCode::Clear),
        (0x0, 0x0, 0xe, 0xe) => Ok(OpCode::Return),
        (0x1, _, _, _) => Ok(OpCode::Jump(Address::new(nnn))),
        (0x2, _, _, _) => Ok(OpCode::Call(Address::new(nnn))),
        (0x3, x, _, _) => Ok(OpCode::SkipEqual(x, nn)),
        (0x4, x, _, _) => Ok(OpCode::SkipNotEqual(x, nn)),
        (0x5, x, y, 0) => Ok(OpCode::SkipRegistersEqual(x, y)),
        (0x6, x, _, _) => Ok(OpCode::Load(x, nn)),
        (0x7, x, _, _) => Ok(OpCode::Inc(x, nn)),
        (0x8, x, y, 0) => Ok(OpCode::Set(x, y)),
        (0x8, x, y, 1) => Ok(OpCode::Or(x, y)),
        (0x8, x, y, 2) => Ok(OpCode::And(x, y)),
        (0x8, x, y, 3) => Ok(OpCode::Xor(x, y)),
        (0x8, x, y, 4) => Ok(OpCode::Add(x, y)),
        (0x8, x, y, 5) => Ok(OpCode::Sub(x, y)),
        (0x8, x, y, 6) => Ok(OpCode::RShift(x, y)),
        (0x8, x, y, 7) => Ok(OpCode::InvSub(x, y)),
        (0x8, x, y, 0xe) => Ok(OpCode::LShift(x, y)),
        (0x9, x, y, 0) => Ok(OpCode::SkipRegistersNotEqual(x, y)),
        (0xa, _, _, _) => Ok(OpCode::LoadIndex(Address::new(nnn))),
        (0xb, _, _, _) => Ok(OpCode::Flow(Address::new(nnn))),
        (0xd, x, y, n) => Ok(OpCode::Draw(x, y, n)),
        (0xe, x, 9, 0xe) => Ok(OpCode::IsKey(x)),
        (0xe, x, 0xa, 1) => Ok(OpCode::IsNotKey(x)),
        (0xf, x, 0, 7) => Ok(OpCode::SetDelay(x)),
        (0xf, x, 0, 0xa) => Ok(OpCode::WaitKey(x)),
        (0xf, x, 1, 5) => Ok(OpCode::Delay(x)),
        (0xf, x, 1, 8) => Ok(OpCode::Sound(x)),
        (0xf, x, 1, 0xe) => Ok(OpCode::IncIndex(x)),
        (0xf, x, 3, 3) => Ok(OpCode::Bcd(x)),
        (0xf, x, 5, 5) => Ok(OpCode::Write(x)),
        (0xf, x, 6, 5) => Ok(OpCode::Read(x)),
        _ => Err(Error::UnknownOpCode(opcode)),
    }
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

pub struct Cpu {
    pc: Address,
    sp: Address,
    i: Address,
    vs: Registers,
    dt: u8, // delay timer
    st: u8, // sound timer
}

impl Cpu {
    pub const START_PC: Address = Address::new(0x200);
    const SP: Address = Address::new(0xeff);

    pub fn new() -> Self {
        Cpu {
            pc: Cpu::START_PC,
            sp: Cpu::SP,
            i: Address::new(0),
            vs: Registers([0; 16]),
            dt: 0,
            st: 0,
        }
    }

    fn push<M: Memory>(&mut self, memory: &mut M, address: Address) {
        let (high, low) = ((address.value() >> 8) as u8, (address.value() & 0xff) as u8);

        memory.write(self.sp, high);
        self.sp = self.sp.wrapping_sub(1);

        memory.write(self.sp, low);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop<M: Memory>(&mut self, memory: &M) -> Address {
        self.sp = self.sp.wrapping_add(1);
        let low = memory.read(self.sp);

        self.sp = self.sp.wrapping_add(1);
        let high = memory.read(self.sp);

        Address::new((high as u16) << 8 | low as u16)
    }

    pub fn tick_timer(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
        }
    }

    fn execute<P: Memory + Display + Keypad>(&mut self, peripheral: &mut P, opcode: OpCode) {
        match opcode {
            OpCode::Clear => peripheral.clear(),
            OpCode::Return => self.pc = self.pop(peripheral),
            OpCode::Jump(address) => self.pc = address,
            OpCode::Call(address) => {
                self.push(peripheral, self.pc);
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
                self.vs[0xf] = 0
            }
            OpCode::And(x, y) => {
                self.vs[x] &= self.vs[y];
                self.vs[0xf] = 0
            }
            OpCode::Xor(x, y) => {
                self.vs[x] ^= self.vs[y];
                self.vs[0xf] = 0
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
                self.vs[x] = self.vs[y] >> 1;
                self.vs[0xf] = lsb;
            }
            OpCode::InvSub(x, y) => {
                let (res, overflowed) = self.vs[y].overflowing_sub(self.vs[x]);
                self.vs[x] = res;
                self.vs[0xf] = !overflowed as u8;
            }
            OpCode::LShift(x, y) => {
                let msb = self.vs[y] >> 7;
                self.vs[x] = self.vs[y] << 1;
                self.vs[0xf] = msb;
            }
            OpCode::SkipRegistersNotEqual(x, y) => {
                if self.vs[x] != self.vs[y] {
                    self.pc = self.pc.wrapping_add(2)
                }
            }
            OpCode::LoadIndex(addr) => self.i = addr,
            OpCode::Flow(addr) => self.pc = addr.wrapping_add(self.vs[0] as u16),
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
                for i in 0..=x {
                    peripheral.write(self.i, self.vs[i]);
                    self.i = self.i.wrapping_add(1);
                }
            }
            OpCode::Read(x) => {
                for i in 0..=x {
                    self.vs[i] = peripheral.read(self.i);
                    self.i = self.i.wrapping_add(1);
                }
            }
        }
    }

    pub fn step<P: Memory + Display + Keypad>(
        &mut self,
        peripheral: &mut P,
    ) -> Result<(), crate::Error> {
        let opcode = fetch(peripheral, self.pc);
        let opcode = decode(opcode)?;

        self.pc = self.pc.wrapping_add(2);

        self.execute(peripheral, opcode);

        Ok(())
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod cpu_tests {
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
    fn push_pop_stack() {
        let mut cpu = Cpu {
            pc: Address::new(0x0),
            sp: Address::new(0x7),
            i: Address::new(0),
            vs: Registers([0; 16]),
            dt: 0,
            st: 0,
        };
        let mut memory = MockMemory([0; 8]);

        let expected = [0, 0, 0, 0, 0x23, 0x1, 0xbc, 0xa];

        cpu.push(&mut memory, Address::new(0xabc));
        assert_eq!(cpu.sp, Address::new(5));
        cpu.push(&mut memory, Address::new(0x123));
        assert_eq!(cpu.sp, Address::new(3));

        assert_eq!(memory.0, expected);

        let expected = [0, 0, 0, 0, 0x54, 0x6, 0xbc, 0xa];

        assert_eq!(cpu.pop(&memory), Address::new(0x123));
        assert_eq!(cpu.sp, Address::new(5));

        cpu.push(&mut memory, Address::new(0x654));
        assert_eq!(memory.0, expected);
        assert_eq!(cpu.sp, Address::new(3));

        assert_eq!(cpu.pop(&memory), Address::new(0x654));
        assert_eq!(cpu.sp, Address::new(5));
        assert_eq!(cpu.pop(&memory), Address::new(0xabc));
        assert_eq!(cpu.sp, Address::new(7));
    }
}
