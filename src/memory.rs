const MEMORY_SIZE: usize = 4096;

pub struct Memory {
    data: [u8; MEMORY_SIZE],
}

impl Memory {
    pub fn new() -> Self {
        let mut new_memory = Self {
            data: [0; MEMORY_SIZE],
        };
        new_memory.initialize_sprites();
        return new_memory;
    }

    pub fn read_bytes(&self, start: u16, count: u16) -> &[u8] {
        let start_address = start as usize;
        let end_address = start_address + count as usize;
        return self.data[start_address..end_address].as_ref();
    }

    pub fn write_bytes(&mut self, start: u16, replacement: &[u8]) {
        let end_address = start as usize + replacement.len();
        if end_address <= self.data.len() {
            self.data[(start as usize)..end_address].copy_from_slice(replacement);
        } else {
            panic!("Replacement would exceed destination slice bounds")
        }
    }

    fn initialize_sprites(&mut self) {
        let sprites: [u8; 80] = [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];

        self.write_bytes(0x0, &sprites);
    }

    pub fn load_program(&mut self, program: &[u8]) {
        self.write_bytes(0x200, program);
    }
}
