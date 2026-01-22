const USER: u64 = 1 << 4;
const EXECUTE: u64 = 1 << 3;
const WRITE: u64 = 1 << 2;
const READ: u64 = 1 << 1;
const VALID: u64 = 1;

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct PageTableEntry(u64);

impl hal::vm::PageTableEntry for PageTableEntry {
    fn set_user_mode(&mut self) -> &mut Self {
        self.0 = hal::set_bit(USER, self.0);
        self
    }

    fn set_write(&mut self) -> &mut Self {
        self.0 = hal::set_bit(WRITE, self.0);
        self
    }

    fn set_read(&mut self) -> &mut Self {
        self.0 = hal::set_bit(READ, self.0);
        self
    }

    fn set_execute(&mut self) -> &mut Self {
        self.0 = hal::set_bit(EXECUTE, self.0);
        self
    }

    fn set_valid(&mut self) -> &mut Self {
        self.0 = hal::set_bit(VALID, self.0);
        self
    }

    fn is_valid(&self) -> bool {
        if self.0 & VALID == 1 { true } else { false }
    }

    fn get_physical_address(&self) -> usize {
        (self.0 as usize >> 10) << 12
    }

    fn set_physical_address(&mut self, pa: usize) -> &mut Self {
        let mut original = self.0;
        original |= ((pa >> 12) << 10) as u64;
        self.0 = original;
        self
    }

    fn is_leaf_pte(&self) -> bool {
        let bits = self.0;

        if bits & READ == 0 && bits & WRITE == 0 && bits & EXECUTE == 0 {
            true
        } else {
            false
        }
    }

    fn clear_bits(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    fn readable(&self) -> bool {
        let bits = self.0;

        if bits & READ != 0 { true } else { false }
    }

    fn writeable(&self) -> bool {
        let bits = self.0;

        if bits & WRITE != 0 { true } else { false }
    }

    fn executable(&self) -> bool {
        let bits = self.0;

        if bits & EXECUTE != 0 { true } else { false }
    }

    fn user_mode_accessible(&self) -> bool {
        let bits = self.0;

        if bits & USER != 0 { true } else { false }
    }

    fn supervisor_accessible(&self) -> bool {
        let bits = self.0;

        if bits & USER != 0 {
            riscv::register::sstatus::read().sum()
        } else {
            true
        }
    }
}
