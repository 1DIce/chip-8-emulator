use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::time::Instant;

use crate::instruction::Instruction;
use crate::keyboard::Keyboard;
use crate::memory::Memory;
use crate::renderer::Renderer;

const CARRY_REG_ADDRESS: usize = 0xF;

pub struct ProgramCounter {
    /// used to store the currently executing address
    ptr: u16,
}

impl ProgramCounter {
    fn new() -> Self {
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

pub struct Cpu<'a> {
    registers: Registers,
    /// an array of 16 16-bit values, used to store the address that the interpreter should return to when finished with a subroutine
    stack: [u16; 16],

    memory: Memory,

    renderer: &'a RefCell<Renderer>,

    keyboard: &'a RefCell<Keyboard>,

    run_timer: Option<Instant>,
}

impl<'a> Cpu<'a> {
    pub fn new(renderer: &'a RefCell<Renderer>, keyboard: &'a RefCell<Keyboard>) -> Cpu<'a> {
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
            run_timer: None,
        };
    }

    pub fn load_program_into_memory(&mut self, program: &[u8]) {
        self.memory.load_program(program)
    }

    fn progress_timers(&mut self) {
        if self.registers.delay_timer > 0 {
            self.registers.delay_timer -= 1;
        }
        if self.registers.sound_timer > 0 {
            self.registers.sound_timer -= 1;
        }
    }

    pub fn run_cycle(&mut self) {
        if self.run_timer.is_none() {
            self.run_timer = Some(Instant::now());
        }
        if self.run_timer.expect("timer exists").elapsed().as_millis() >= (1000 / 60) {
            self.progress_timers();
            self.run_timer = Some(Instant::now());
        }

        let mut instruction = [0, 0];
        instruction.clone_from_slice(
            self.memory
                .read_bytes(self.registers.program_counter.address(), 2),
        );
        self.process_instructions(&instruction);
    }

    fn process_instructions(&mut self, instruction_bytes: &[u8; 2]) {
        let instruction = Instruction::new(instruction_bytes);

        print!("Instruction: ");
        instruction.print();

        let nibbles = instruction.nibbles_lo();
        match nibbles {
            (0x0, 0x0, 0xE, 0x0) => self.process_clear_display(&instruction),
            (0x0, 0x0, 0xE, 0xE) => self.process_return_from_subroutine(&instruction),

            (0x1, _, _, _) => self.process_jump(&instruction),

            (0x2, _, _, _) => self.process_call_subroutine(&instruction),

            (0x3, _, _, _) => self.process_skip_if_equal_byte(&instruction),

            (0x4, _, _, _) => self.process_skip_if_not_equal_byte(&instruction),

            (0x5, _, _, _) => self.process_skip_if_equal_register(&instruction),

            (0x6, _, _, _) => self.process_set_register(&instruction),

            (0x7, _, _, _) => self.process_add_byte(&instruction),

            (0x8, _, _, 0x0) => self.process_copy_register_value(&instruction),
            (0x8, _, _, 0x1) => self.process_or_registers(&instruction),
            (0x8, _, _, 0x2) => self.process_and_registers(&instruction),
            (0x8, _, _, 0x3) => self.process_xor_registers(&instruction),
            (0x8, _, _, 0x4) => self.process_add_registers(&instruction),
            (0x8, _, _, 0x5) => self.process_sub_registers(&instruction),
            (0x8, _, _, 0x6) => self.process_shift_right(&instruction),
            (0x8, _, _, 0x7) => self.process_subn_registers(&instruction),
            (0x8, _, _, 0xE) => self.process_shift_left(&instruction),

            (0x9, _, _, _) => self.process_skip_if_not_equal_register(&instruction),

            (0xA, _, _, _) => self.process_set_register_i_to_address(&instruction),

            (0xB, _, _, _) => self.process_move_program_counter(&instruction),

            (0xC, _, _, _) => self.process_generate_random_number(&instruction),

            (0xD, _, _, _) => self.process_display_sprite(&instruction),

            (0xE, _, _, 0x1) => self.process_skip_if_key_not_pressed(&instruction),
            (0xE, _, _, 0xE) => self.process_skip_if_key_pressed(&instruction),

            (0xF, _, 0x0, 0x7) => self.process_set_vx_to_delay_timer(&instruction),
            (0xF, _, 0x0, 0xA) => self.process_store_key_press(&instruction),
            (0xF, _, 0x1, 0x5) => self.process_set_delay_timer(&instruction),
            (0xF, _, 0x1, 0x8) => self.process_set_sound_timer(&instruction),
            (0xF, _, 0x1, 0xE) => self.process_add_vx_to_i(&instruction),

            (0xF, _, 0x2, _) => self.process_set_i_to_sprite_address(&instruction),
            (0xF, _, 0x3, _) => self.process_store_vx_as_bsd_in_memory(&instruction),
            (0xF, _, 0x5, _) => self.process_store_registers_in_memory(&instruction),
            (0xF, _, 0x6, _) => self.process_load_registers_from_memory(&instruction),
            _ => panic!("unexpected instruction"),
        };
    }

    fn process_return_from_subroutine(&mut self, _instruction: &Instruction) {
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

    fn process_clear_display(&mut self, _instruction: &Instruction) {
        self.renderer.borrow_mut().clear_display();
        self.registers.program_counter.increment();
    }

    /// The value of delay timer register is placed into Vx.
    fn process_set_vx_to_delay_timer(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        self.registers.general_registers[x] = self.registers.delay_timer;
        self.registers.program_counter.increment();
    }

    fn process_skip_if_key_not_pressed(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        if !self.keyboard.borrow().is_key_pressed_or_held(&vx) {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    fn process_skip_if_key_pressed(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        if self.keyboard.borrow().is_key_pressed_or_held(&vx) {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    /// The interpreter reads n bytes from memory, starting at the address stored in I.
    /// These bytes are then displayed as sprites on screen at coordinates (Vx, Vy)
    fn process_display_sprite(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let n = instruction.fourth_nibble();

        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];
        let i = self.registers.i;
        let sprite = self.memory.read_bytes(i, n as u16);

        let pixel_erased = self.renderer.borrow_mut().draw_sprite(sprite, vx, vy);
        self.registers.general_registers[CARRY_REG_ADDRESS] = if pixel_erased { 1 } else { 0 };
        self.registers.program_counter.increment();
    }

    /// The interpreter generates a random number from 0 to 255,
    /// which is then ANDed with the value kk. The results are stored in Vx.
    /// See instruction 8xy2 for more information on AND.
    fn process_generate_random_number(&mut self, instruction: &Instruction) {
        let kk = instruction.kk();
        let x = instruction.x() as usize;
        let random_num: u8 = rand::random();
        self.registers.general_registers[x] = random_num & kk;
        self.registers.program_counter.increment();
    }

    /// The program counter is set to nnn plus the value of V0.
    fn process_move_program_counter(&mut self, instruction: &Instruction) {
        let nnn = instruction.nnn();
        let v0 = self.registers.general_registers[0];
        self.registers
            .program_counter
            .set_to_address(nnn + v0 as u16);
    }

    /// The value of register I is set to nnn.
    fn process_set_register_i_to_address(&mut self, instruction: &Instruction) {
        let nnn = instruction.nnn();
        self.registers.i = nnn;
        self.registers.program_counter.increment();
    }

    fn process_skip_if_not_equal_register(&mut self, instruction: &Instruction) {
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
    fn process_skip_if_equal_register(&mut self, instruction: &Instruction) {
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
    fn process_add_byte(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let byte = instruction.kk();
        let (result, _overflow) = self.registers.general_registers[x].overflowing_add(byte);
        self.registers.general_registers[x] = result;
        self.registers.program_counter.increment();
    }

    fn process_set_register(&mut self, instruction: &Instruction) {
        let register_address = instruction.x() as usize;
        let byte = instruction.kk();
        self.registers.general_registers[register_address] = byte;
        self.registers.program_counter.increment();
    }

    fn process_skip_if_not_equal_byte(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let kk = instruction.kk();

        if self.registers.general_registers[x] != kk {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    fn process_skip_if_equal_byte(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let kk = instruction.kk();

        if self.registers.general_registers[x as usize] == kk {
            self.registers.program_counter.skip_instruction();
        } else {
            self.registers.program_counter.increment();
        }
    }

    fn process_call_subroutine(&mut self, instruction: &Instruction) {
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

    fn process_jump(&mut self, instruction: &Instruction) {
        let address = instruction.nnn();
        self.registers.program_counter.set_to_address(address);
    }

    /// Stores the value of register Vy in register Vx.
    fn process_copy_register_value(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let y = instruction.y();
        self.registers.general_registers[x as usize] = self.registers.general_registers[y as usize];
        self.registers.program_counter.increment();
    }

    /// Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
    /// A bitwise OR compares the corresponding bits from two values, and if either bit is 1,
    /// then the same bit in the result is also 1. Otherwise, it is 0.
    fn process_or_registers(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let y = instruction.y();
        self.registers.general_registers[x as usize] |=
            self.registers.general_registers[y as usize];
        self.registers.program_counter.increment();
    }

    /// Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
    /// A bitwise AND compares the corresponding bits from two values,
    /// and if both bits are 1, then the same bit in the result is also 1. Otherwise, it is 0.
    fn process_and_registers(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let y = instruction.y();
        self.registers.general_registers[x as usize] &=
            self.registers.general_registers[y as usize];
        self.registers.program_counter.increment();
    }

    fn process_xor_registers(&mut self, instruction: &Instruction) {
        let x = instruction.x();
        let y = instruction.y();
        self.registers.general_registers[x as usize] ^=
            self.registers.general_registers[y as usize];
        self.registers.program_counter.increment();
    }

    /// The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1,
    /// otherwise 0. Only the lowest 8 bits of the result are kept, and stored in Vx.
    fn process_add_registers(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let (result, overflow) = (self.registers.general_registers[x])
            .overflowing_add(self.registers.general_registers[y]);

        self.registers.general_registers[x] = result;
        self.registers.general_registers[CARRY_REG_ADDRESS] = if overflow { 1 } else { 0 };
        self.registers.program_counter.increment();
    }

    fn process_sub_registers(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];

        let (result, underflow) = vx.overflowing_sub(vy);
        self.registers.general_registers[x] = result;
        self.registers.general_registers[CARRY_REG_ADDRESS] = if underflow { 0 } else { 1 };
        self.registers.program_counter.increment();
    }

    fn process_shift_right(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vy = self.registers.general_registers[y];

        self.registers.general_registers[x] = vy >> 1;
        self.registers.general_registers[CARRY_REG_ADDRESS] = vy % 2;
        self.registers.program_counter.increment();
    }

    fn process_subn_registers(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];

        let (result, underflow) = vy.overflowing_sub(vx);
        self.registers.general_registers[x] = result;
        self.registers.general_registers[CARRY_REG_ADDRESS] = if underflow { 0 } else { 1 };
        self.registers.program_counter.increment();
    }

    fn process_shift_left(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let y = instruction.y() as usize;
        let vy = self.registers.general_registers[y];

        self.registers.general_registers[x] = vy << 1;
        self.registers.general_registers[CARRY_REG_ADDRESS] = if vy >= 128 { 1 } else { 0 };
        self.registers.program_counter.increment();
    }

    /// All execution stops until a key is pressed, then the value of that key is stored in Vx.
    fn process_store_key_press(&mut self, instruction: &Instruction) {
        let pressed_key = self.keyboard.borrow().get_pressed_key();
        if let Some(chip_8_key) = pressed_key {
            let x = instruction.x() as usize;
            self.registers.general_registers[x] = chip_8_key;
            self.registers.program_counter.increment();
        }
        // do not progress the program_counter if no key was pressed.
        // The instruction will be evaluated again until a key is pressed
    }

    /// Delay timer is set equal to the value of Vx.
    fn process_set_delay_timer(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        self.registers.delay_timer = vx;
        self.registers.program_counter.increment();
    }

    /// Sound timer is set equal to the value of Vx.
    fn process_set_sound_timer(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        self.registers.sound_timer = vx;
        self.registers.program_counter.increment();
    }

    /// The values of I and Vx are added, and the results are stored in I.
    fn process_add_vx_to_i(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        self.registers.i += vx as u16;
        self.registers.program_counter.increment();
    }

    /// The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx.
    /// See section 2.4, Display, for more information on the Chip-8 hexadecimal font.
    fn process_set_i_to_sprite_address(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];
        let sprite_address = (vx * 5) as u16; // a sprite is 5 bytes in size
        self.registers.i = sprite_address;
        self.registers.program_counter.increment();
    }

    /// Takes the decimal value of Vx, and places the hundreds digit in memory at location in I,
    /// the tens digit at location I+1, and the ones digit at location I+2
    fn process_store_vx_as_bsd_in_memory(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        let vx = self.registers.general_registers[x];

        let bcd_representation = [(vx / 100) % 10, (vx / 10) % 10, vx % 10];
        self.memory
            .borrow_mut()
            .write_bytes(self.registers.i, &bcd_representation);
        self.registers.program_counter.increment();
    }

    fn process_store_registers_in_memory(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        //let vx = self.registers.general_registers[x];

        let registers = self.registers.general_registers;
        let i = self.registers.i;
        self.memory.write_bytes(i, &registers[0..=(x as usize)]);
        self.registers.program_counter.increment();
    }

    fn process_load_registers_from_memory(&mut self, instruction: &Instruction) {
        let x = instruction.x() as usize;
        //let vx = self.registers.general_registers[x];

        let from = self.registers.i;
        let read_data = self.memory.read_bytes(from, 1 + x as u16);

        for (index, value) in read_data.iter().enumerate() {
            self.registers.general_registers[index] = *value;
        }
        self.registers.program_counter.increment();
    }
}
