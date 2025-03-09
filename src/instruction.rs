use u4::U4x2;
use u4::U4;

pub struct Instruction {
    bytes: [U4x2; 2],
}

impl Instruction {
    pub fn new(instruction_bytes: &[u8; 2]) -> Self {
        let first_byte = U4x2::from_byte(instruction_bytes[0]);
        let second_byte = U4x2::from_byte(instruction_bytes[1]);
        return Self {
            bytes: [first_byte, second_byte],
        };
    }

    pub fn nibbles_lo(&self) -> (u8, u8, u8, u8) {
        return (
            self.first_nibble() as u8,
            self.second_nibble() as u8,
            self.third_nibble() as u8,
            self.fourth_nibble() as u8,
        );
    }

    pub fn first_nibble(&self) -> U4 {
        return self.bytes[0].left();
    }

    pub fn second_nibble(&self) -> U4 {
        return self.bytes[0].right();
    }
    pub fn third_nibble(&self) -> U4 {
        return self.bytes[1].left();
    }
    pub fn fourth_nibble(&self) -> U4 {
        return self.bytes[1].right();
    }

    pub fn x(&self) -> U4 {
        return self.second_nibble();
    }

    pub fn y(&self) -> U4 {
        return self.third_nibble();
    }

    pub fn kk(&self) -> u8 {
        return self.bytes[1].packed;
    }

    pub fn nnn(&self) -> u16 {
        let mut nnn = self.second_nibble() as u16;
        nnn <<= 8;
        nnn |= self.bytes[1].packed as u16;
        return nnn;
    }

    pub fn print(&self) {
        for byte in self.bytes.iter() {
            let left = byte.left();
            let right = byte.right();
            print!("{left:x}{right:x}");
        }
        println!();
    }
}
