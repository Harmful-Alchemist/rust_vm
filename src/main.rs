use std::io::stdout;
use std::io::Read;
use std::io::Write;
use std::process;

enum Registers {
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

enum TrapCodes {
    GetCharacter = 0x20,
    Out = 0x21,
    Puts = 0x22,
    TRAP_IN = 0x23,
    PutsTwo = 0x24,
    Halt = 0x25,
}

impl TrapCodes {
    fn from_integer(x: u16) -> TrapCodes {
        unsafe { std::mem::transmute::<u8, TrapCodes>(x as u8) }
    }
}

enum ConditionFlags {
    Positive = 1 << 0,
    Zero = 1 << 1,
    Negative = 1 << 2,
}

struct VM {
    memory: [u16; std::u16::MAX as usize],
    reg: [u16; Registers::Count as usize],
}

impl VM {
    fn start(&mut self) {
        let StartPosition: u16 = 0x3000;

        self.reg[Registers::ProgramCounter as usize] = StartPosition;

        let running = true;
        while running {
            /* FETCH */
            let instr = self.mem_read(self.reg[Registers::ProgramCounter as usize]);
            let op = instr >> 12;
            self.reg[Registers::ProgramCounter as usize] += 1; // Post increment program counter
            match OperationCodes::from_integer(op) {
                OperationCodes::Add => self.add(op),
                OperationCodes::And => self.and(op),
                OperationCodes::Not => self.not(op),
                OperationCodes::Branch => self.branch(op),
                OperationCodes::Jump => self.jump(op),
                OperationCodes::JumpRegister => self.jump_register(op),
                OperationCodes::Load => self.load(op),
                OperationCodes::LoadIndirect => self.load_indirect(op),
                OperationCodes::LoadRegister => self.load_register(op),
                OperationCodes::LoadEffectiveAddress => self.load_effective_address(op),
                OperationCodes::Store => self.store(op),
                OperationCodes::StoreIndirect => self.store_indirect(op),
                OperationCodes::StoreRegister => self.store_register(op),
                OperationCodes::Trap => self.trap(op),
                _ => (),
            }
        }
    }
    fn add(&mut self, instr: u16) {
        /* destination register (DR) */
        let r0 = (instr >> 9) & 0x7;
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
        if cond_flag & self.reg[Registers::Condition as usize] > 0 {
            self.reg[Registers::ProgramCounter as usize] += pc_offset;
        }
    }

    fn jump(&mut self, instr: u16) {
        /* Also handles RET */
        let r1 = (instr >> 6) & 0x7;
        self.reg[Registers::ProgramCounter as usize] = self.reg[r1 as usize];
    }

    fn jump_register(&mut self, instr: u16) {
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
        self.reg[r0 as usize] =
            self.mem_read(self.reg[Registers::ProgramCounter as usize] + pc_offset);
        self.update_flags(r0);
    }

    fn update_flags(&mut self, r: u16) {
        let r_val = self.reg[r as usize];
        self.reg[Registers::Condition as usize] = if r_val > 0 {
            ConditionFlags::Zero as u16
        } else if (r_val >> 15) > 0 {
            ConditionFlags::Negative as u16
        } else {
            ConditionFlags::Positive as u16
        }
    }

    fn load_indirect(&mut self, instr: u16) {
        /* destination register (DR) */
        let r0 = (instr >> 9) & 0x7;
        /* PCoffset 9*/
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        /* add pc_offset to the current PC, look at that memory location to get the final address */
        let read = self.mem_read(self.reg[Registers::ProgramCounter as usize] + pc_offset);
        self.reg[r0 as usize] = self.mem_read(read);
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
        match TrapCodes::from_integer(instr & 0xFF) {
            TrapCodes::GetCharacter => self.get_character(),
            TrapCodes::Out => self.out(),
            TrapCodes::Puts => self.puts(),
            TrapCodes::TRAP_IN => self.scan(),
            TrapCodes::PutsTwo => self.putsp(),
            TrapCodes::Halt => self.halt(),
        }
    }

    fn get_character(&mut self) {
        let input: u16 = std::io::stdin()
            .bytes()
            .next()
            .and_then(|result| result.ok())
            .map(|byte| byte as u16)
            .expect("Could not read character!");

        self.reg[Registers::R0 as usize] = input & 0b1111_1111;
    }

    fn out(&mut self) {
        print!("{}", self.reg[Registers::R0 as usize] as u8 as char);
        stdout().flush().expect("Could not print!");
    }

    fn puts(&mut self) {
        let mut addr = self.reg[Registers::R0 as usize];
        let mut character = self.memory[addr as usize];

        while character != 0 {
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

        while character != 0 {
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
        process::exit(0);
    }

    fn mem_write(&mut self, adress: u16, val: u16) {
        self.memory[adress as usize] = val;
    }

    fn mem_read(&mut self, address: u16) -> u16 {
        0
        //TODO!
    }
}

fn main() {
    //  {Load Arguments, 12}
    //     {Setup, 12}
    let mut vm = VM {
        reg: [0; Registers::Count as usize],
        memory: [0; std::u16::MAX as usize],
    };
    vm.start();
    /* set the PC to starting position */
    /* 0x3000 is the default */

    // {Shutdown, 12}
}

fn sign_extend(x: u16, bit_count: i32) -> u16 {
    let mut y = x;
    if (x >> (bit_count - 1)) & 1 > 0 {
        y = x | (0xFFFF << bit_count);
    }
    y
}
