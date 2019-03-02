pub mod lc3_vm {
    use std::io::stdin;
    use std::io::stdout;
    use std::io::Read;
    use std::io::Write;
    use std::env;
    use std::fs::File;
    use std::io::prelude::*;

    const KeyboardStatusRegister: u16 = 0xFE00;
    const KeyboardDataRegister: u16 = 0xFE02;

    pub enum Registers {
        R0 = 0,
        R1,
        R2,
        R3,
        R4,
        R5,
        R6,
        R7,
        ProgramCounter,
        Condition,
        Count,
    }

    #[derive(Debug)]
    enum OperationCodes {
        Branch = 0,
        Add,
        Load,
        Store,
        JumpRegister,
        And,
        LoadRegister,
        StoreRegister,
        Unused,
        Not,
        LoadIndirect,
        StoreIndirect,
        Jump,
        Reserved,
        LoadEffectiveAddress,
        Trap,
    }

    impl OperationCodes {
        fn from_integer(x: u16) -> OperationCodes {
            unsafe { std::mem::transmute::<u8, OperationCodes>(x as u8) }
        }
    }

    const TRAP_GetCharacter: u16 = 0x20;
    const TRAP_Out: u16 = 0x21;
    const TRAP_Puts: u16 = 0x22;
    const TRAP_IN: u16 = 0x23;
    const TRAP_PutsTwo: u16 = 0x24;
    const TRAP_Halt: u16 = 0x25;

    const Positive: u16 = 1 << 0;
    const Zero: u16 = 1 << 1;
    const Negative: u16 = 1 << 2;

    pub struct VM {
        memory: [u16; std::u16::MAX as usize + 1],
        reg: [u16; Registers::Count as usize + 1],
        running: bool,
    }

    impl VM {
        pub fn new() -> VM {
            VM {
                reg: [0; crate::lc3_vm::lc3_vm::Registers::Count as usize + 1],
                memory: [0; std::u16::MAX as usize + 1],
                running: false,
            }
        }

        pub fn start(&mut self, mut program: File) {
            self.read_program(program);
            let start_position: u16 = 0x3000;

            self.reg[Registers::ProgramCounter as usize] = start_position;

            self.running = true;
            while self.running {
                /* FETCH */
                let instr = self.mem_read(self.reg[Registers::ProgramCounter as usize]);
                let op = instr >> 12;
                // println!(
                //     "instruction {:#b} for op {:?}",
                //     instr,
                //     OperationCodes::from_integer(op)
                // );
                self.reg[Registers::ProgramCounter as usize] += 1; // Post increment program counter
                // println!(
                //     "Incremented program counter is {}",
                //     self.reg[Registers::ProgramCounter as usize]
                // );
                match OperationCodes::from_integer(op) {
                    OperationCodes::Add => self.add(instr),
                    OperationCodes::And => self.and(instr),
                    OperationCodes::Not => self.not(instr),
                    OperationCodes::Branch => self.branch(instr),
                    OperationCodes::Jump => self.jump(instr),
                    OperationCodes::JumpRegister => self.jump_register(instr),
                    OperationCodes::Load => self.load(instr),
                    OperationCodes::LoadIndirect => self.load_indirect(instr),
                    OperationCodes::LoadRegister => self.load_register(instr),
                    OperationCodes::LoadEffectiveAddress => self.load_effective_address(instr),
                    OperationCodes::Store => self.store(instr),
                    OperationCodes::StoreIndirect => self.store_indirect(instr),
                    OperationCodes::StoreRegister => self.store_register(instr),
                    OperationCodes::Trap => self.trap(instr),
                    _ => panic!("Unknown instruction {:#b}", instr),
                }
            }
        }

        fn read_program(&mut self, mut program: File) {
            let mut buffer: [u8; 2] = [0; 2];
            program.read(&mut buffer).expect("Failed to read origin.");
            let mut origin = swap_endian(buffer);
            // println!("origin: {:#b}", origin);
            loop {
                match program.read(&mut buffer) {
                    Ok(2) => {
                        self.memory[origin as usize] = swap_endian(buffer);
                        origin = origin + 1;
                    }
                    Ok(0) => break,
                    Ok(_) => {
                        panic!("Unexpected error reading program.");
                    }
                    Err(_) => {
                        panic!("Unexpected error reading program.");
                    }
                }
            }
        }

        fn add(&mut self, instr: u16) {
            /* destination register (DR) */
            let r0 = (instr >> 9) & 0x7;
            // println!("Adding, was {}", self.reg[r0 as usize]);

            /* first operand (SR1) */
            let r1 = (instr >> 6) & 0x7;
            /* whether we are in immediate mode */
            let imm_flag = (instr >> 5) & 0x1;

            if imm_flag > 0 {
                let imm5 = sign_extend(instr & 0x1F, 5);
                self.reg[r0 as usize] = self.reg[r1 as usize] + imm5;
            } else {
                let r2 = instr & 0x7;
                self.reg[r0 as usize] = self.reg[r1 as usize] + self.reg[r2 as usize];
            }
            // println!("Adding, is now {}", self.reg[r0 as usize]);
            self.update_flags(r0);
        }

        fn and(&mut self, instr: u16) {
            /* destination register (DR) */
            let r0 = (instr >> 9) & 0x7;
            /* first operand (SR1) */
            let r1 = (instr >> 6) & 0x7;
            /* whether we are in immediate mode */
            let imm_flag = (instr >> 5) & 0x1;
            if imm_flag > 0 {
                let imm5 = sign_extend(instr & 0x1F, 5);
                self.reg[r0 as usize] = self.reg[r1 as usize] & imm5;
            } else {
                let r2 = instr & 0x7;
                self.reg[r0 as usize] = self.reg[r1 as usize] & self.reg[r2 as usize];
            }
            self.update_flags(r0);
        }

        fn not(&mut self, instr: u16) {
            /* destination register (DR) */
            let r0 = (instr >> 9) & 0x7;
            /* operand (SR) */
            let r1 = (instr >> 6) & 0x7;

            self.reg[r0 as usize] = !(self.reg[r1 as usize]);
            self.update_flags(r0);
        }

        fn branch(&mut self, instr: u16) {
            let pc_offset = sign_extend((instr) & 0x1ff, 9);
            let cond_flag = (instr >> 9) & 0x7;
            // println!(
            //     "cond flag: {:#b} if test {}",
            //     cond_flag,
            //     (cond_flag & self.reg[Registers::Condition as usize])
            // );
            if cond_flag & self.reg[Registers::Condition as usize] > 0 {
                // println!(
                //     "True branch! Program counter was {}",
                //     self.reg[Registers::ProgramCounter as usize]
                // );
                self.reg[Registers::ProgramCounter as usize] += pc_offset;
                // println!(
                //     "True branch! New program counter is {}",
                //     self.reg[Registers::ProgramCounter as usize]
                // );
            }
        }

        fn jump(&mut self, instr: u16) {
            /* Also handles RET */
            let r1 = (instr >> 6) & 0x7;
            self.reg[Registers::ProgramCounter as usize] = self.reg[r1 as usize];
        }

        fn jump_register(&mut self, instr: u16) {
            self.reg[Registers::R7 as usize] = self.reg[Registers::ProgramCounter as usize];
            let jsr = (instr >> 11) & 1;
            if jsr > 0 {
                let pc_offset = sign_extend(instr & 0x7FF, 11);
                self.reg[Registers::ProgramCounter as usize] += pc_offset;
            } else {
                //jsrr
                self.reg[Registers::ProgramCounter as usize] = (instr >> 6) & 0x7;
            }
        }

        fn load(&mut self, instr: u16) {
            /* destination register (DR) */
            let r0 = (instr >> 9) & 0x7;
            /* PCoffset 9*/
            let pc_offset = sign_extend(instr & 0x1ff, 9);
            /* add pc_offset to the current PC, look at that memory location to get the final address */
            let loaded = self.mem_read(self.reg[Registers::ProgramCounter as usize] + pc_offset);
            self.reg[r0 as usize] = loaded;
            self.update_flags(r0);
        }

        fn update_flags(&mut self, r: u16) {
            let r_val = self.reg[r as usize];
            self.reg[Registers::Condition as usize] = if r_val == 0 {
                Zero
            } else if (r_val >> 15) > 0 {
                Negative
            } else {
                Positive
            }
        }

        fn load_indirect(&mut self, instr: u16) {
            /* destination register (DR) */
            let r0 = (instr >> 9) & 0x7;
            /* PCoffset 9*/
            let pc_offset = sign_extend(instr & 0x1ff, 9);
            /* add pc_offset to the current PC, look at that memory location to get the final address */
            let read = self.mem_read(self.reg[Registers::ProgramCounter as usize] + pc_offset);
            //println!("register to read ldi: {:#b}", read);
            self.reg[r0 as usize] = self.mem_read(read);
            //println!("Contents of that register: {:#b}", self.reg[r0 as usize]);
            self.update_flags(r0);
        }

        fn load_register(&mut self, instr: u16) {
            // 0x40 ? 64 / 16 =  4 ???
            let offset: u16 = sign_extend(instr & 0b11_1111, 6);

            let dr = (instr >> 9) & 0b111;
            let baser = (instr >> 6) & 0b111;

            self.reg[dr as usize] = self.mem_read(self.reg[baser as usize] + offset);
            self.update_flags(dr);
        }

        fn load_effective_address(&mut self, instr: u16) {
            /* destination register (DR) */
            let r0 = (instr >> 9) & 0x7;
            /* PCoffset 9*/
            let pc_offset = sign_extend(instr & 0x1ff, 9);
            /* add pc_offset to the current PC, look at that memory location to get the final address */
            self.reg[r0 as usize] = self.reg[Registers::ProgramCounter as usize] + pc_offset;
            self.update_flags(r0);
        }

        fn store(&mut self, instr: u16) {
            let sr = (instr >> 9) & 0x7;
            /* PCoffset 9*/
            let pc_offset = sign_extend(instr & 0x1ff, 9);
            self.mem_write(
                self.reg[Registers::ProgramCounter as usize] + pc_offset,
                self.reg[sr as usize],
            );
        }

        fn store_indirect(&mut self, instr: u16) {
            let sr = (instr >> 9) & 0x7;
            /* PCoffset 9*/
            let pc_offset = sign_extend(instr & 0x1ff, 9);
            let read = self.mem_read(self.reg[Registers::ProgramCounter as usize] + pc_offset);
            self.mem_write(read, self.reg[sr as usize]);
        }

        fn store_register(&mut self, instr: u16) {
            let sr = (instr >> 9) & 0x7;
            /* PCoffset 6*/
            let offset: u16 = sign_extend(instr & 0b11_1111, 6);
            let baser = (instr >> 6) & 0b111;
            self.mem_write(self.reg[baser as usize] + offset, self.reg[sr as usize]);
        }

        fn trap(&mut self, instr: u16) {
            //println!("complete trap instruction {:#b}", instr);
            //println!("Got trap {:#b} or in hex {:#x}", instr & 0xFF, instr & 0xFF);
            match instr & 0xFF {
                TRAP_GetCharacter => self.get_character(),
                TRAP_Out => self.out(),
                TRAP_Puts => self.puts(),
                TRAP_TRAP_IN => self.scan(),
                TRAP_PutsTwo => self.putsp(),
                TRAP_Halt => self.halt(),
            }
        }

        fn get_character(&mut self) {
            //TODO ignore enter
            let input: u16 = std::io::stdin()
                .bytes()
                .next()
                .and_then(|result| result.ok())
                .map(|byte| byte as u16)
                .expect("Could not read character!");
            //println!("char was {}", input);
            self.reg[Registers::R0 as usize] = input & 0b1111_1111;
        }

        fn out(&mut self) {
            print!("{}", self.reg[Registers::R0 as usize] as u8 as char);
            // println!(" as u16 {}", self.reg[Registers::R0 as usize]);
            stdout().flush().expect("Could not print!");
        }

        fn puts(&mut self) {
            let mut addr = self.reg[Registers::R0 as usize];
            let mut character = self.memory[addr as usize];

            while character > 0 {
                print!("{}", (character & 0b1111_1111) as u8 as char); //Hmmmm.......
                addr = addr + 1;
                character = self.memory[addr as usize];
            }
            stdout().flush().expect("Could not print!");
        }

        // in
        fn scan(&mut self) {
            print!("Enter a character: ");
            stdout().flush().expect("Could not print!");

            self.get_character();
        }

        fn putsp(&mut self) {
            let mut addr = self.reg[Registers::R0 as usize];
            let mut character = self.memory[addr as usize];

            while character > 0 {
                print!("{}", (character & 0b1111_1111) as u8 as char);
                let second_part = (character >> 8) & 0b1111_1111;
                if second_part == 0 {
                    break;
                }
                print!("{}", second_part as u8 as char);
                addr = addr + 1;
                character = self.memory[addr as usize];
            }
            stdout().flush().expect("Could not print!");
        }

        fn halt(&mut self) {
            println!("Goodbye!");
            self.running = false;
        }

        fn mem_write(&mut self, adress: u16, val: u16) {
            self.memory[adress as usize] = val;
        }

        fn mem_read(&mut self, address: u16) -> u16 {
            if address == KeyboardStatusRegister {
                // println!("Reading keyboard!");
                match stdin().bytes().next() {
                    //TODO is this even correct? needs a timeout?
                    None => {
                        // println!("Didn't read a byte from the keyboard.");
                        self.memory[KeyboardStatusRegister as usize] = 0;
                    }
                    Some(a_byte) => {
                        let character = a_byte.expect("Could not read input.") as u16;
                        // println!("Read from keyboard char: {}", character);
                        if character != 10 {
                            //TODO ignore enters, but thats weird........
                            self.memory[KeyboardStatusRegister as usize] = 1 << 15;
                            self.memory[KeyboardDataRegister as usize] = character;
                        } else {
                            self.memory[KeyboardStatusRegister as usize] = 0;
                        }
                    }
                }
            }
            self.memory[address as usize]
        }
    }

    fn swap_endian(original: [u8; 2]) -> u16 {
        original[1] as u16 + ((original[0] as u16) << 8) //TODO the right way?
    }

    fn sign_extend(x: u16, bit_count: i32) -> u16 {
        let mut y = x;
        // for negative numbers
        if ((y >> (bit_count - 1)) & 1) > 0 {
            y |= 0xFFFF << bit_count;
        }
        y
    }
}