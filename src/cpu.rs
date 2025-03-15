use std::borrow::BorrowMut;
use std::time::Instant;

use u4::{U4x2, U4};

use crate::audio::Audio;
use crate::instruction::Instruction;
use crate::keyboard::Keyboard;
use crate::memory::Memory;
use crate::program_counter::ProgramCounter;
use crate::renderer::Renderer;

const CARRY_REG_ADDRESS: usize = 0xF;

struct Registers {
    /// 16 general purpose 8-bit registers, usually referred to as Vx, where x is a hexadecimal digit (0 through F)
    general_registers: [u8; 16],
    /// register is generally used to store memory addresses, so only the lowest (rightmost) 12 bits are usually used
    i: u16,
    /// When register is non-zero, they are automatically decremented at a rate of 60Hz
    delay_timer: u8,
    /// When register is non-zero, they are automatically decremented at a rate of 60Hz
    sound_timer: u8,
    program_counter: ProgramCounter,
    /// points to topmost level of the stack
    stack_pointer: Option<u8>,
}

pub struct Cpu {
    registers: Registers,
    /// an array of 16 16-bit values, used to store the address that the interpreter should return to when finished with a subroutine
    stack: [u16; 16],

    memory: Memory,

    renderer: Renderer,

    keyboard: Keyboard,

    audio: Audio,

    time_since_timer_update: Option<Instant>,
}

impl Cpu {
    pub fn new(renderer: Renderer, keyboard: Keyboard) -> Cpu {
        return Cpu {
            registers: Registers {
                general_registers: [0; 16],
                i: 0,
                delay_timer: 0,
                sound_timer: 0,
                program_counter: ProgramCounter::new(),
                stack_pointer: None,
            },
            stack: [0; 16],
            memory: Memory::new(),
            renderer,
            keyboard,
            time_since_timer_update: None,
            audio: Audio::new(),
        };
    }

    pub fn load_program_into_memory(&mut self, program: &[u8]) {
        self.memory.load_program(program)
    }

    pub fn run_cycle(&mut self) {
        if self.time_since_timer_update.is_none() {
            self.time_since_timer_update = Some(Instant::now());
        }
        let elapsed_frames = self
            .time_since_timer_update
            .expect("timer exists")
            .elapsed()
            .as_millis()
            / 60;
        if elapsed_frames >= 1 {
            self.progress_timer_registers(elapsed_frames);
            self.time_since_timer_update = Some(Instant::now());
        }

        let mut instruction = [0, 0];
        instruction.clone_from_slice(
            self.memory
                .read_bytes(self.registers.program_counter.address(), 2),
        );
        self.evaluate_instructions(&instruction);
    }

    fn progress_timer_registers(&mut self, elapsed_frames: u128) {
        if self.registers.delay_timer > 0 {
            self.registers.delay_timer = self
                .registers
                .delay_timer
                .saturating_sub(elapsed_frames as u8);
        }
        if self.registers.sound_timer > 0 {
            self.audio.play(self.registers.sound_timer);
            self.registers.sound_timer = self
                .registers
                .sound_timer
                .saturating_sub(elapsed_frames as u8);
        } else {
            self.audio.stop();
        }
    }

    fn evaluate_instructions(&mut self, instruction_bytes: &[u8; 2]) {
        let instruction = Instruction::new(instruction_bytes);

        print!("Instruction: ");
        instruction.print();

        let nibbles = instruction.nibbles_lo();
        match nibbles {
            (0x0, 0x0, 0x0, 0x0) => self.ignore_instruction(),
            (0x0, 0x0, 0xE, 0x0) => self.exec_clear_display(&instruction),
            (0x0, 0x0, 0xE, 0xE) => self.exec_return_from_subroutine(&instruction),

            (0x1, _, _, _) => self.exec_jump(&instruction),

            (0x2, _, _, _) => self.exec_call_subroutine(&instruction),

            (0x3, _, _, _) => self.exec_skip_if_equal_kk(&instruction),

            (0x4, _, _, _) => self.exec_skip_if_not_equal_kk(&instruction),

            (0x5, _, _, _) => self.exec_skip_if_equal_register(&instruction),

            (0x6, _, _, _) => self.exec_set_register(&instruction),

            (0x7, _, _, _) => self.exec_add_kk(&instruction),

            (0x8, _, _, 0x0) => self.exec_copy_register_value(&instruction),
            (0x8, _, _, 0x2) => self.exec_and(&instruction),
            (0x8, _, _, 0x1) => self.exec_or(&instruction),
            (0x8, _, _, 0x3) => self.exec_xor(&instruction),
            (0x8, _, _, 0x4) => self.exec_add(&instruction),
            (0x8, _, _, 0x5) => self.exec_sub(&instruction),
            (0x8, _, _, 0x6) => self.exec_shift_right(&instruction),
            (0x8, _, _, 0x7) => self.exec_subn(&instruction),
            (0x8, _, _, 0xE) => self.exec_shift_left(&instruction),

            (0x9, _, _, _) => self.exec_skip_if_not_equal_register(&instruction),

            (0xA, _, _, _) => self.exec_set_register_i_to_nnn(&instruction),

            (0xB, _, _, _) => self.exec_move_program_counter(&instruction),

            (0xC, _, _, _) => self.exec_generate_random_number(&instruction),

            (0xD, _, _, 0x0) => self.ignore_instruction(),
            (0xD, _, _, _) => self.exec_display_sprite_8xN(&instruction),

            (0xE, _, 0x9, 0xE) => self.exec_skip_if_key_pressed(&instruction),
            (0xE, _, 0xA, 0x1) => self.exec_skip_if_key_not_pressed(&instruction),

            (0xF, _, 0x0, 0x7) => self.exec_set_vx_to_delay_timer(&instruction),
            (0xF, _, 0x0, 0xA) => self.exec_wait_until_key_press(&instruction),
            (0xF, _, 0x1, 0x5) => self.exec_set_delay_timer(&instruction),
            (0xF, _, 0x1, 0x8) => self.exec_set_sound_timer(&instruction),
            (0xF, _, 0x1, 0xE) => self.exec_add_vx_to_i(&instruction),

            (0xF, _, 0x2, _) => self.exec_set_i_to_sprite_address(&instruction),
            (0xF, _, 0x3, _) => self.exec_store_vx_as_bsd_in_memory(&instruction),
            (0xF, _, 0x5, 0x5) => self.exec_store_registers_in_memory(&instruction),
            (0xF, _, 0x6, 0x5) => self.exec_load_registers_from_memory(&instruction),
            _ => panic!("unexpected instruction"),
        };
    }

    fn exec_return_from_subroutine(&mut self, _instruction: &Instruction) {
        let stack_pointer = self
            .registers
            .stack_pointer
            .expect("stack should not be empty");
        let return_address = self
            .stack
            .get(stack_pointer as usize)
            .expect("Stack entry should exist");
        self.registers.stack_pointer = if stack_pointer == 0 {
            None
        } else {
            Some(stack_pointer - 1)
        };
        self.registers
            .program_counter
            .set_to_address(*return_address);
    }

    fn exec_clear_display(&mut self, _instruction: &Instruction) {
        self.renderer.borrow_mut().clear_display();
        self.registers.program_counter.increment();
    }

    /// The value of delay timer register is placed into Vx.
    fn exec_set_vx_to_delay_timer(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        self.registers.general_registers[x] = self.registers.delay_timer;
        self.registers.program_counter.increment();
    }

    fn exec_skip_if_key_not_pressed(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        if !self
            .keyboard
            .is_key_pressed_or_held(&U4x2::from(vx).right())
        {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    fn exec_skip_if_key_pressed(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        if self
            .keyboard
            .is_key_pressed_or_held(&U4x2::from(vx).right())
        {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    #[allow(non_snake_case)]
    /// The interpreter reads n bytes from memory, starting at the address stored in I.
    /// These bytes are then displayed as sprites on screen at coordinates (Vx, Vy)
    fn exec_display_sprite_8xN(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let n = instruction.fourth_nibble();

        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];
        let i = self.registers.i;
        let sprite = self.memory.read_bytes(i, n as u16);

        let pixel_erased = self.renderer.draw_sprite(sprite, vx, vy);
        self.registers.general_registers[CARRY_REG_ADDRESS] = if pixel_erased { 1 } else { 0 };
        self.registers.program_counter.increment();
    }

    /// The interpreter generates a random number from 0 to 255,
    /// which is then ANDed with the value kk. The results are stored in Vx.
    /// See instruction 8xy2 for more information on AND.
    fn exec_generate_random_number(&mut self, instruction: &Instruction) {
        let kk = instruction.kk();
        let x = instruction.x() as usize;
        let random_num: u8 = rand::random();
        self.registers.general_registers[x] = random_num & kk;
        self.registers.program_counter.increment();
    }

    /// The program counter is set to nnn plus the value of V0.
    fn exec_move_program_counter(&mut self, instruction: &Instruction) {
        let nnn = instruction.nnn();
        let v0 = self.registers.general_registers[0];
        self.registers
            .program_counter
            .set_to_address(nnn + v0 as u16);
    }

    /// The value of register I is set to nnn.
    fn exec_set_register_i_to_nnn(&mut self, instruction: &Instruction) {
        let nnn = instruction.nnn();
        self.registers.i = nnn;
        self.registers.program_counter.increment();
    }

    fn exec_skip_if_not_equal_register(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];
        if vx != vy {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }
    fn exec_skip_if_equal_register(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];
        if vx == vy {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    /// Add byte kk to the register x. No carry flag is set in case of an overflow
    fn exec_add_kk(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let kk = instruction.kk();
        let (result, _overflow) = self.registers.general_registers[x].overflowing_add(kk);
        self.registers.general_registers[x] = result;
        self.registers.program_counter.increment();
    }

    fn exec_set_register(&mut self, instruction: &Instruction) {
        let register_address = instruction.x() as usize;
        let byte = instruction.kk();
        self.registers.general_registers[register_address] = byte;
        self.registers.program_counter.increment();
    }

    fn exec_skip_if_not_equal_kk(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let kk = instruction.kk();

        if self.registers.general_registers[x] != kk {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    fn exec_skip_if_equal_kk(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let kk = instruction.kk();

        if self.registers.general_registers[x as usize] == kk {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    fn exec_call_subroutine(&mut self, instruction: &Instruction) {
        self.registers.stack_pointer = if self.registers.stack_pointer.is_none() {
            Some(0)
        } else {
            self.registers.stack_pointer.map(|p| p + 1)
        };
        let return_address = self.registers.program_counter.peek();
        self.stack[self.registers.stack_pointer.expect("Stack pointer exists") as usize] =
            return_address;

        let address = instruction.nnn();
        self.registers.program_counter.set_to_address(address);
    }

    fn exec_jump(&mut self, instruction: &Instruction) {
        let address = instruction.nnn();
        self.registers.program_counter.set_to_address(address);
    }

    /// Stores the value of register Vy in register Vx.
    fn exec_copy_register_value(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let y = instruction.y();
        self.registers.general_registers[x as usize] = self.registers.general_registers[y as usize];
        self.registers.program_counter.increment();
    }

    /// Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
    /// A bitwise OR compares the corresponding bits from two values, and if either bit is 1,
    /// then the same bit in the result is also 1. Otherwise, it is 0.
    fn exec_or(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let y = instruction.y();
        self.registers.general_registers[x as usize] |=
            self.registers.general_registers[y as usize];
        self.registers.general_registers[CARRY_REG_ADDRESS] = 0;
        self.registers.program_counter.increment();
    }

    /// Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
    /// A bitwise AND compares the corresponding bits from two values,
    /// and if both bits are 1, then the same bit in the result is also 1. Otherwise, it is 0.
    fn exec_and(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let y = instruction.y();
        self.registers.general_registers[x as usize] &=
            self.registers.general_registers[y as usize];
        self.registers.general_registers[CARRY_REG_ADDRESS] = 0;
        self.registers.program_counter.increment();
    }

    fn exec_xor(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let y = instruction.y();
        self.registers.general_registers[x as usize] ^=
            self.registers.general_registers[y as usize];
        self.registers.general_registers[CARRY_REG_ADDRESS] = 0;
        self.registers.program_counter.increment();
    }

    /// The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1,
    /// otherwise 0. Only the lowest 8 bits of the result are kept, and stored in Vx.
    fn exec_add(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let (result, overflow) = (self.registers.general_registers[x])
            .overflowing_add(self.registers.general_registers[y]);

        self.registers.general_registers[x] = result;
        self.registers.general_registers[CARRY_REG_ADDRESS] = if overflow { 1 } else { 0 };
        self.registers.program_counter.increment();
    }

    fn exec_sub(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];

        let (result, underflow) = vx.overflowing_sub(vy);
        self.registers.general_registers[x] = result;
        self.registers.general_registers[CARRY_REG_ADDRESS] = if underflow { 0 } else { 1 };
        self.registers.program_counter.increment();
    }

    fn exec_shift_right(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vy = self.registers.general_registers[y];

        self.registers.general_registers[x] = vy >> 1;
        self.registers.general_registers[CARRY_REG_ADDRESS] = vy % 2;
        self.registers.program_counter.increment();
    }

    fn exec_subn(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];

        let (result, underflow) = vy.overflowing_sub(vx);
        self.registers.general_registers[x] = result;
        self.registers.general_registers[CARRY_REG_ADDRESS] = if underflow { 0 } else { 1 };
        self.registers.program_counter.increment();
    }

    fn exec_shift_left(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vy = self.registers.general_registers[y];

        self.registers.general_registers[x] = vy << 1;
        self.registers.general_registers[CARRY_REG_ADDRESS] = if vy >= 128 { 1 } else { 0 };
        self.registers.program_counter.increment();
    }

    /// All execution stops until a key is pressed, then the value of that key is stored in Vx.
    fn exec_wait_until_key_press(&mut self, instruction: &Instruction) {
        let mut key_pressed: Option<U4> = None;
        loop {
            if let Some(key) = key_pressed {
                if !self.keyboard.is_key_pressed_or_held(&key) {
                    break;
                }
            } else if let Some(pressed_key) = self.keyboard.get_pressed_key() {
                key_pressed = Some(pressed_key);
                let x = instruction.x() as usize;
                self.registers.general_registers[x] = pressed_key as u8;
            }
        }
        self.registers.program_counter.increment();
    }

    /// Delay timer is set equal to the value of Vx.
    fn exec_set_delay_timer(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        self.registers.delay_timer = vx;
        self.registers.program_counter.increment();
    }

    /// Sound timer is set equal to the value of Vx.
    fn exec_set_sound_timer(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        self.registers.sound_timer = vx;
        self.registers.program_counter.increment();
    }

    /// The values of I and Vx are added, and the results are stored in I.
    fn exec_add_vx_to_i(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        self.registers.i += vx as u16;
        self.registers.program_counter.increment();
    }

    /// The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx.
    /// See section 2.4, Display, for more information on the Chip-8 hexadecimal font.
    fn exec_set_i_to_sprite_address(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        let sprite_address = (vx * 5) as u16; // a sprite is 5 bytes in size
        self.registers.i = sprite_address;
        self.registers.program_counter.increment();
    }

    /// Takes the decimal value of Vx, and places the hundreds digit in memory at location in I,
    /// the tens digit at location I+1, and the ones digit at location I+2
    fn exec_store_vx_as_bsd_in_memory(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];

        let bcd_representation = [(vx / 100) % 10, (vx / 10) % 10, vx % 10];
        self.memory
            .write_bytes(self.registers.i, &bcd_representation);
        self.registers.program_counter.increment();
    }

    ///  The value of each variable register from V0 to VX inclusive (if X is 0, then only V0)
    ///  will be stored in successive memory addresses, starting with the one that’s stored in I.
    ///  V0 will be stored at the address in I, V1 will be stored in I + 1, and so on, until VX is stored in I + X.
    ///
    ///  Chip-8 quirk: Each time it stored or loaded one register, it incremented I.
    ///  After the instruction was finished, I would end up being set to the new value I + X + 1.
    fn exec_store_registers_in_memory(&mut self, instruction: &Instruction) {
        let x = instruction.x();

        let registers = self.registers.general_registers;
        self.memory
            .write_bytes(self.registers.i, &registers[0..=x as usize]);
        self.registers.i += x as u16 + 1;
        self.registers.program_counter.increment();
    }

    ///  Values from V0 to VX inclusive (if X is 0, then only V0)
    ///  will be loaded from successive memory addresses, starting with the one that’s stored in I.
    ///  V0 will be loaded from the address in I, V1 will be loaded from I + 1, and so on, until VX is loaded from I + X.
    ///
    ///  Chip-8 quirk: Each time it loaded one register, it incremented I.
    ///  After the instruction was finished, I would end up being set to the new value I + X + 1.
    fn exec_load_registers_from_memory(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let read_data = self.memory.read_bytes(self.registers.i, 1 + x as u16);

        for (index, value) in read_data.iter().enumerate() {
            self.registers.general_registers[index] = *value;
            self.registers.i += 1;
        }
        self.registers.program_counter.increment();
    }

    fn ignore_instruction(&mut self) {
        self.registers.program_counter.increment();
    }
}
