pub struct ProgramCounter {
    /// used to store the currently executing address
    ptr: u16,
}

impl ProgramCounter {
    pub fn new() -> Self {
        return Self { ptr: 0x200 };
    }

    pub fn address(&self) -> u16 {
        return self.ptr;
    }

    pub fn peek(&self) -> u16 {
        return self.ptr + 2;
    }

    pub fn increment(&mut self) {
        self.ptr += 2;
    }

    pub fn skip_instruction(&mut self) {
        self.ptr += 4;
    }

    pub fn set_to_address(&mut self, address: u16) {
        assert!(
            address >= 0x200,
            "stack pointer address should be at least the first program address"
        );
        self.ptr = address;
    }
}
