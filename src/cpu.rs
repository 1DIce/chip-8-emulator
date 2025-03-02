use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::usize;

use anyhow::Result;
use u4::{AsNibbles, Stack, U4};

use crate::keyboard::Keyboard;
use crate::memory::Memory;
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
    /// used to store the currently executing address
    program_counter: u16,
    /// points to topmost level of the stack
    stack_pointer: u8,
}

pub struct Cpu<'a> {
    registers: Registers,
    /// an array of 16 16-bit values, used to store the address that the interpreter should return to when finished with a subroutine
    stack: [u16; 16],

    memory: Memory,

    renderer: &'a RefCell<Renderer>,

    keyboard: &'a RefCell<Keyboard>,
}

impl<'a> Cpu<'a> {
    pub fn new(renderer: &'a RefCell<Renderer>, keyboard: &'a RefCell<Keyboard>) -> Cpu<'a> {
        return Cpu {
            registers: Registers {
                general_registers: [0; 16],
                i: 0,
                delay_timer: 0,
                sound_timer: 0,
                program_counter: 0x200,
                stack_pointer: 0,
            },
            stack: [0; 16],
            memory: Memory::new(),
            renderer,
            keyboard,
        };
    }

    pub fn load_program_into_memory(&mut self, program: &[u8]) {
        self.memory.load_program(program)
    }

    pub fn progress_timers(&mut self) {
        if self.registers.sound_timer > 0 {
            self.registers.delay_timer -= 1;
        }
        if self.registers.sound_timer > 0 {
            self.registers.sound_timer -= 1;
        }
    }

    pub fn run_cycle(&mut self) {
        let program_start_address = 0x200;

        let mut instruction = [0, 0];
        instruction.clone_from_slice(self.memory.read_bytes(
            // multiple by 2 because each program instruction is 2 bytes long
            (self.registers.program_counter - program_start_address) * 2 + program_start_address,
            2,
        ));
        self.process_instructions(&instruction);
    }

    fn process_instructions(&mut self, instruction_bytes: &[u8]) {
        let instruction = Stack::from_iter(&AsNibbles(instruction_bytes));

        print!("Instruction: ");
        for nib in instruction.iter() {
            print!("{:x} ", nib);
        }
        println!("");

        assert_eq!(instruction.len(), 4);
        let first_nibble = instruction.get(0).expect("First nibble should exist");
        match first_nibble {
            u4::U4::Dec00 => {
                if get_second_nibble(&instruction) == u4::U4::Dec00
                    && get_third_nibble(&instruction) == u4::U4::Dec14
                    && get_last_nibble(&instruction) == u4::U4::Dec00
                {
                    self.process_clear_display(&instruction)
                } else if get_second_nibble(&instruction) == u4::U4::Dec00
                    && get_third_nibble(&instruction) == u4::U4::Dec14
                    && get_last_nibble(&instruction) == u4::U4::Dec14
                {
                    self.process_return_from_subroutine(&instruction)
                } else {
                    // ignoring 0nnn instruction
                }
            }
            u4::U4::Dec01 => self.process_jump(&instruction),
            u4::U4::Dec02 => self.process_call_subroutine(&instruction),
            u4::U4::Dec03 => self.process_skip_if_equal_byte(&instruction),
            u4::U4::Dec04 => self.process_skip_if_not_equal_byte(&instruction),
            u4::U4::Dec05 => self.process_skip_if_equal_register(&instruction),
            u4::U4::Dec06 => self.process_set_register(&instruction),
            u4::U4::Dec07 => self.process_add_byte(&instruction),

            u4::U4::Dec08 => match get_last_nibble(&instruction) {
                U4::Dec00 => self.process_copy_register_value(&instruction),
                U4::Dec01 => self.process_or_registers(&instruction),
                U4::Dec02 => self.process_and_registers(&instruction),
                U4::Dec03 => self.process_xor_registers(&instruction),
                U4::Dec04 => self.process_add_registers(&instruction),
                U4::Dec05 => self.process_sub_registers(&instruction),
                U4::Dec06 => self.process_shift_right(&instruction),
                U4::Dec07 => self.process_subn_registers(&instruction),
                U4::Dec14 => self.process_shift_left(&instruction),
                _ => panic!("Unexpected instruction"),
            },

            u4::U4::Dec09 => self.process_skip_if_not_equal_register(&instruction),
            u4::U4::Dec10 => self.process_set_register_i_to_address(&instruction),
            u4::U4::Dec11 => self.process_move_program_counter(&instruction),
            u4::U4::Dec12 => self.process_generate_random_number(&instruction),
            u4::U4::Dec13 => self.process_display_sprite(&instruction),

            u4::U4::Dec14 => match get_last_nibble(&instruction) {
                U4::Dec01 => self.process_skip_if_key_not_pressed(&instruction),
                U4::Dec14 => self.process_skip_if_key_pressed(&instruction),
                _ => panic!("Unexpected instruction "),
            },

            u4::U4::Dec15 => match get_third_nibble(&instruction) {
                //
                U4::Dec00 => match get_last_nibble(&instruction) {
                    U4::Dec07 => self.process_set_vx_to_delay_timer(&instruction),
                    U4::Dec10 => self.process_store_key_press(&instruction),
                    _ => panic!("Unexpected instruction"),
                },
                U4::Dec01 => match get_last_nibble(&instruction) {
                    U4::Dec05 => self.process_set_delay_timer(&instruction),
                    U4::Dec08 => self.process_set_sound_timer(&instruction),
                    U4::Dec14 => self.process_add_vx_to_i(&instruction),
                    _ => panic!("Unexpected instruction"),
                },
                U4::Dec02 => self.process_set_i_to_sprite_address(&instruction),
                U4::Dec03 => self.process_store_vx_as_bsd_in_memory(&instruction),
                U4::Dec05 => self.process_store_registers_in_memory(&instruction),
                U4::Dec06 => self.process_load_registers_from_memory(&instruction),
                _ => panic!("Unexpected instruction"),
            },
        };
    }

    fn process_return_from_subroutine(&mut self, _instruction: &Stack) {
        let stack_pointer = self.registers.stack_pointer as usize;
        let return_address = self
            .stack
            .get(stack_pointer)
            .expect("Stack entry should exist");
        assert!(self.registers.stack_pointer != 0);
        self.registers.stack_pointer -= 1;
        self.registers.program_counter = *return_address;
    }

    fn process_clear_display(&mut self, _instruction: &Stack) {
        self.renderer.borrow_mut().clear_display();
        self.registers.program_counter += 1;
    }

    /// The value of delay timer register is placed into Vx.
    fn process_set_vx_to_delay_timer(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        self.registers.general_registers[x] = self.registers.delay_timer;
        self.registers.program_counter += 1;
    }

    fn process_skip_if_key_not_pressed(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        if !self.keyboard.borrow().is_key_pressed_or_held(&vx) {
            self.registers.program_counter += 2;
        } else {
            self.registers.program_counter += 1;
        }
    }

    fn process_skip_if_key_pressed(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        if self.keyboard.borrow().is_key_pressed_or_held(&vx) {
            self.registers.program_counter += 2;
        } else {
            self.registers.program_counter += 1;
        }
    }

    /// The interpreter reads n bytes from memory, starting at the address stored in I.
    /// These bytes are then displayed as sprites on screen at coordinates (Vx, Vy)
    fn process_display_sprite(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let y = get_y_nibble(instruction) as usize;
        let n = get_last_nibble(instruction);

        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];
        let i = self.registers.i;
        let sprite = self.memory.read_bytes(i, n as u16);

        self.renderer.borrow_mut().draw_sprite(sprite, vx, vy);
        self.registers.program_counter += 1;
    }

    /// The interpreter generates a random number from 0 to 255,
    /// which is then ANDed with the value kk. The results are stored in Vx.
    /// See instruction 8xy2 for more information on AND.
    fn process_generate_random_number(&mut self, instruction: &Stack) {
        let kk = get_kk_byte(instruction);
        let x = get_x_nibble(instruction) as usize;
        let random_num: u8 = rand::random();
        self.registers.general_registers[x] = random_num & kk;
        self.registers.program_counter += 1;
    }

    /// The program counter is set to nnn plus the value of V0.
    fn process_move_program_counter(&mut self, instruction: &Stack) {
        let nnn = get_nnn_address(instruction);
        let v0 = self.registers.general_registers[0];
        self.registers.program_counter = nnn + v0 as u16;
    }

    /// The value of register I is set to nnn.
    fn process_set_register_i_to_address(&mut self, instruction: &Stack) {
        let nnn = get_nnn_address(instruction);
        self.registers.i = nnn;
        self.registers.program_counter += 1;
    }

    fn process_skip_if_not_equal_register(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let y = get_y_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];
        if vx != vy {
            self.registers.program_counter += 2;
        } else {
            self.registers.program_counter += 1;
        }
    }
    fn process_skip_if_equal_register(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let y = get_y_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];
        if vx == vy {
            self.registers.program_counter += 2;
        } else {
            self.registers.program_counter += 1;
        }
    }

    fn process_add_byte(&mut self, instruction: &Stack) {
        let register_address = get_x_nibble(instruction);
        let byte = get_kk_byte(instruction);
        let (result, _overflow) =
            self.registers.general_registers[register_address as usize].overflowing_add(byte);
        self.registers.general_registers[register_address as usize] = result;
        self.registers.program_counter += 1;
    }

    fn process_set_register(&mut self, instruction: &Stack) {
        let register_address = get_x_nibble(instruction);
        let byte = get_kk_byte(instruction);
        self.registers.general_registers[register_address as usize] = byte;
        self.registers.program_counter += 1;
    }

    fn process_skip_if_not_equal_byte(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction);
        let kk = get_kk_byte(instruction);

        if self.registers.general_registers[x as usize] != kk {
            self.registers.program_counter += 2
        } else {
            self.registers.program_counter += 1;
        }
    }

    fn process_skip_if_equal_byte(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction);
        let kk = get_kk_byte(instruction);

        if self.registers.general_registers[x as usize] == kk {
            self.registers.program_counter += 2
        } else {
            self.registers.program_counter += 1;
        }
    }

    fn process_call_subroutine(&mut self, instruction: &Stack) {
        self.registers.stack_pointer += 1;
        self.stack[self.registers.stack_pointer as usize] = self.registers.program_counter;

        let address = get_nnn_address(instruction);
        self.registers.program_counter = address;
    }

    fn process_jump(&mut self, instruction: &Stack) {
        let address = get_nnn_address(instruction);
        self.registers.program_counter = address;
    }

    /// Stores the value of register Vy in register Vx.
    fn process_copy_register_value(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction);
        let y = get_y_nibble(instruction);
        self.registers.general_registers[x as usize] = self.registers.general_registers[y as usize];
        self.registers.program_counter += 1;
    }

    /// Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
    /// A bitwise OR compares the corresponding bits from two values, and if either bit is 1,
    /// then the same bit in the result is also 1. Otherwise, it is 0.
    fn process_or_registers(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction);
        let y = get_y_nibble(instruction);
        self.registers.general_registers[x as usize] |=
            self.registers.general_registers[y as usize];
        self.registers.program_counter += 1;
    }

    /// Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
    /// A bitwise AND compares the corresponding bits from two values,
    /// and if both bits are 1, then the same bit in the result is also 1. Otherwise, it is 0.
    fn process_and_registers(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction);
        let y = get_y_nibble(instruction);
        self.registers.general_registers[x as usize] &=
            self.registers.general_registers[y as usize];
        self.registers.program_counter += 1;
    }

    fn process_xor_registers(&mut self, instruction: &Stack) {
        let vx = get_x_nibble(instruction);
        let vy = get_y_nibble(instruction);
        self.registers.general_registers[vx as usize] ^=
            self.registers.general_registers[vy as usize];
        self.registers.program_counter += 1;
    }

    /// The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1,
    /// otherwise 0. Only the lowest 8 bits of the result are kept, and stored in Vx.
    fn process_add_registers(&mut self, instruction: &Stack) {
        let vx = get_x_nibble(instruction) as usize;
        let vy = get_y_nibble(instruction) as usize;
        let result = (self.registers.general_registers[vx] as u16)
            + (self.registers.general_registers[vy] as u16);

        self.registers.general_registers[CARRY_REG_ADDRESS] = if result > 255 { 1 } else { 0 };
        self.registers.general_registers[vx] = result as u8;
        self.registers.program_counter += 1;
    }

    fn process_sub_registers(&mut self, instruction: &Stack) {
        let vx = get_x_nibble(instruction) as usize;
        let vy = get_y_nibble(instruction) as usize;
        let x = self.registers.general_registers[vx];
        let y = self.registers.general_registers[vy];

        if x > y {
            self.registers.general_registers[CARRY_REG_ADDRESS] = 1;
            self.registers.general_registers[vx] = x - y;
        } else {
            self.registers.general_registers[CARRY_REG_ADDRESS] = 0;
            self.registers.general_registers[vx] = y - x;
        }
        self.registers.program_counter += 1;
    }

    fn process_shift_right(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        self.registers.general_registers[CARRY_REG_ADDRESS] = vx % 2;
        self.registers.general_registers[x] = vx >> 1;
        self.registers.program_counter += 1;
    }

    fn process_subn_registers(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let y = get_y_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        let vy = self.registers.general_registers[y];

        if vy > vx {
            self.registers.general_registers[CARRY_REG_ADDRESS] = 1;
            self.registers.general_registers[x] = vy - vx;
        } else {
            self.registers.general_registers[CARRY_REG_ADDRESS] = 0;
            self.registers.general_registers[x] = vx - vy;
        }
        self.registers.program_counter += 1;
    }

    fn process_shift_left(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        self.registers.general_registers[CARRY_REG_ADDRESS] = if vx >= 128 { 1 } else { 0 };
        self.registers.general_registers[x] = vx << 1;
        self.registers.program_counter += 1;
    }

    /// All execution stops until a key is pressed, then the value of that key is stored in Vx.
    fn process_store_key_press(&mut self, instruction: &Stack) {
        let pressed_key = self.keyboard.borrow().get_pressed_key();
        if let Some(chip_8_key) = pressed_key {
            let x = get_x_nibble(instruction) as usize;
            self.registers.general_registers[x] = chip_8_key;
            self.registers.program_counter += 1;
        }
        // do not progress the program_counter if no key was pressed.
        // The instruction will be evaluated again until a key is pressed
    }

    /// Delay timer is set equal to the value of Vx.
    fn process_set_delay_timer(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        self.registers.delay_timer = vx;
        self.registers.program_counter += 1;
    }

    /// Sound timer is set equal to the value of Vx.
    fn process_set_sound_timer(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        self.registers.sound_timer = vx;
        self.registers.program_counter += 1;
    }

    /// The values of I and Vx are added, and the results are stored in I.
    fn process_add_vx_to_i(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        self.registers.i += vx as u16;
        self.registers.program_counter += 1;
    }

    /// The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx.
    /// See section 2.4, Display, for more information on the Chip-8 hexadecimal font.
    fn process_set_i_to_sprite_address(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];
        let sprite_address = (vx * 5) as u16; // a sprite is 5 bytes in size
        self.registers.i = sprite_address;
        self.registers.program_counter += 1;
    }

    /// Takes the decimal value of Vx, and places the hundreds digit in memory at location in I,
    /// the tens digit at location I+1, and the ones digit at location I+2
    fn process_store_vx_as_bsd_in_memory(&mut self, instruction: &Stack) {
        let x = get_x_nibble(instruction) as usize;
        let vx = self.registers.general_registers[x];

        let bcd_representation = [((vx as u16) % 1000) as u8, vx % 100, vx % 10];
        self.memory
            .borrow_mut()
            .write_bytes(self.registers.i, &bcd_representation);
        self.registers.program_counter += 1;
    }

    fn process_store_registers_in_memory(&mut self, instruction: &Stack) {
        let registers = self.registers.general_registers;
        let i = self.registers.i;
        self.memory.write_bytes(i, &registers);
        self.registers.program_counter += 1;
    }

    fn process_load_registers_from_memory(&mut self, instruction: &Stack) {
        let count = self.registers.general_registers.len() as u16;
        let from = self.registers.i;
        let read_data = self.memory.read_bytes(from, count);

        for (index, register) in self.registers.general_registers.iter_mut().enumerate() {
            *register = read_data[index];
        }
        self.registers.program_counter += 1;
    }
}

fn get_nnn_address(instruction: &Stack) -> u16 {
    let mut address: u16 = (instruction.get(1).unwrap()) as u16;
    address <<= 4;
    address |= instruction.get(2).unwrap() as u16;
    address <<= 4;
    address |= instruction.get(3).unwrap() as u16;
    return address;
}

fn get_x_nibble(instruction: &Stack) -> U4 {
    return instruction.get(1).unwrap();
}

fn get_y_nibble(instruction: &Stack) -> U4 {
    return instruction.get(2).unwrap();
}

fn get_last_nibble(instruction: &Stack) -> U4 {
    return instruction.get(3).unwrap();
}

fn get_third_nibble(instruction: &Stack) -> U4 {
    return instruction.get(2).unwrap();
}

fn get_second_nibble(instruction: &Stack) -> U4 {
    return instruction.get(1).unwrap();
}

fn get_kk_byte(instruction: &Stack) -> u8 {
    let mut address: u8 = (instruction.get(2).unwrap()) as u8;
    address = address << 4;
    address = address | instruction.get(3).unwrap() as u8;
    return address;
}
