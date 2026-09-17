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

#[derive(PartialEq, Debug)]
enum OpCode {
    Clear,
    Jump(Address),
    Load(u8, u8),
    Add(u8, u8),
    LoadIndex(Address),
    Draw(u8, u8, u8),
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
        (0x1, _, _, _) => Ok(OpCode::Jump(Address::new(nnn))),
        (0x6, x, _, _) => Ok(OpCode::Load(x, nn)),
        (0x7, x, _, _) => Ok(OpCode::Add(x, nn)),
        (0xa, _, _, _) => Ok(OpCode::LoadIndex(Address::new(nnn))),
        (0xd, x, y, n) => Ok(OpCode::Draw(x, y, n)),
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
    fn decode_load() {
        assert_eq!(decode(0x6123), Ok(OpCode::Load(0x1, 0x23)));
        assert_eq!(decode(0x6fed), Ok(OpCode::Load(0xf, 0xed)));
    }

    #[test]
    fn decode_add() {
        assert_eq!(decode(0x7123), Ok(OpCode::Add(0x1, 0x23)));
        assert_eq!(decode(0x7fed), Ok(OpCode::Add(0xf, 0xed)));
    }

    #[test]
    fn decode_load_index() {
        assert_eq!(decode(0xa123), Ok(OpCode::LoadIndex(Address::new(0x123))));
        assert_eq!(decode(0xafed), Ok(OpCode::LoadIndex(Address::new(0xfed))));
    }

    #[test]
    fn decode_draw() {
        assert_eq!(decode(0xd123), Ok(OpCode::Draw(0x1, 0x2, 0x3)));
        assert_eq!(decode(0xdfed), Ok(OpCode::Draw(0xf, 0xe, 0xd)));
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
    i: Address,
    vs: Registers,
}

impl Cpu {
    pub const START_PC: Address = Address::new(0x200);

    pub fn new() -> Self {
        Cpu {
            pc: Cpu::START_PC,
            i: Address::new(0),
            vs: Registers([0; 16]),
        }
    }

    pub fn step<P: Memory + Display>(&mut self, peripherals: &mut P) -> Result<(), crate::Error> {
        let opcode = fetch(peripherals, self.pc);
        let opcode = decode(opcode)?;

        self.pc = self.pc.wrapping_add(2);

        match opcode {
            OpCode::Clear => peripherals.clear(),
            OpCode::Jump(address) => self.pc = address,
            OpCode::Load(v, value) => self.vs[v] = value,
            OpCode::Add(v, value) => self.vs[v] += value,
            OpCode::LoadIndex(addr) => self.i = addr,
            OpCode::Draw(x, y, n) => {
                let mut sprite = [0u8; 15];

                for (row, byte) in sprite.iter_mut().enumerate().take(n as usize) {
                    *byte = peripherals.read(self.i.wrapping_add(row as u16));
                }

                self.vs[0xf] =
                    peripherals.draw(self.vs[x], self.vs[y], &sprite[..n as usize]) as u8;
            }
        }

        Ok(())
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}
