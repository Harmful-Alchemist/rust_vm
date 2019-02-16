use std::io::stdout;
use std::io::Read;
use std::io::Write;
use std::process;

enum Registers {
    R_R0 = 0,
    R_R1,
    R_R2,
    R_R3,
    R_R4,
    R_R5,
    R_R6,
    R_R7,
    R_PC, /* program counter */
    R_COND,
    R_COUNT,
}

enum opcodes {
    OP_BR = 0,
    /* branch */
    OP_ADD,
    /* add  */
    OP_LD,
    /* load */
    OP_ST,
    /* store */
    OP_JSR,
    /* jump register */
    OP_AND,
    /* bitwise and */
    OP_LDR,
    /* load register */
    OP_STR,
    /* store register */
    OP_RTI,
    /* unused */
    OP_NOT,
    /* bitwise not */
    OP_LDI,
    /* load indirect */
    OP_STI,
    /* store indirect */
    OP_JMP,
    /* jump */
    OP_RES,
    /* reserved (unused) */
    OP_LEA,
    /* load effective address */
    OP_TRAP,
    /* execute trap */
}

impl opcodes {
    fn from_integer(x: u16) -> opcodes {
        unsafe { std::mem::transmute::<u8, opcodes>(x as u8) }
    }
}

enum trap_codes {
    TRAP_GETC = 0x20,  /* get character from keyboard */
    TRAP_OUT = 0x21,   /* output a character */
    TRAP_PUTS = 0x22,  /* output a word string */
    TRAP_IN = 0x23,    /* input a string */
    TRAP_PUTSP = 0x24, /* output a byte string */
    TRAP_HALT = 0x25,  /* halt the program */
}

impl trap_codes {
    fn from_integer(x: u16) -> trap_codes {
        unsafe { std::mem::transmute::<u8, trap_codes>(x as u8) }
    }
}

enum condition_flags {
    FL_POS = 1 << 0,
    /* P */
    FL_ZRO = 1 << 1,
    /* Z */
    FL_NEG = 1 << 2,
    /* N */
}

struct VM {
    memory: [u16; std::u16::MAX as usize],
    reg: [u16; Registers::R_COUNT as usize],
}

impl VM {
    fn start(&mut self) {
        enum position {
            PC_START = 0x3000,
        };

        self.reg[Registers::R_PC as usize] = position::PC_START as u16;

        let running = true;
        while running {
            /* FETCH */
            let instr = self.mem_read(self.reg[Registers::R_PC as usize]);
            let op = instr >> 12;
            self.reg[Registers::R_PC as usize] += 1; // Post increment program counter
            match opcodes::from_integer(op) {
                opcodes::OP_ADD => self.add(op),
                opcodes::OP_AND => self.and(op),
                opcodes::OP_NOT => self.not(op),
                opcodes::OP_BR => self.br(op),
                opcodes::OP_JMP => self.jmp(op),
                opcodes::OP_JSR => self.jsr(op),
                opcodes::OP_LD => self.ld(op),
                opcodes::OP_LDI => self.ldi(op),
                opcodes::OP_LDR => self.ldr(op),
                opcodes::OP_LEA => self.lea(op),
                opcodes::OP_ST => self.st(op),
                opcodes::OP_STI => self.sti(op),
                opcodes::OP_STR => self.str(op),
                opcodes::OP_TRAP => self.trap(op),
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

    fn br(&mut self, instr: u16) {
        let pc_offset = sign_extend((instr) & 0x1ff, 9);
        let cond_flag = (instr >> 9) & 0x7;
        if cond_flag & self.reg[Registers::R_COND as usize] > 0 {
            self.reg[Registers::R_PC as usize] += pc_offset;
        }
    }

    fn jmp(&mut self, instr: u16) {
        /* Also handles RET */
        let r1 = (instr >> 6) & 0x7;
        self.reg[Registers::R_PC as usize] = self.reg[r1 as usize];
    }

    fn jsr(&mut self, instr: u16) {
        let jsr = (instr >> 11) & 1;
        if jsr > 0 {
            let pc_offset = sign_extend(instr & 0x7FF, 11);
            self.reg[Registers::R_PC as usize] += pc_offset;
        } else {
            //jsrr
            self.reg[Registers::R_PC as usize] = (instr >> 6) & 0x7;
        }
    }

    fn ld(&mut self, instr: u16) {
        /* destination register (DR) */
        let r0 = (instr >> 9) & 0x7;
        /* PCoffset 9*/
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        /* add pc_offset to the current PC, look at that memory location to get the final address */
        self.reg[r0 as usize] = self.mem_read(self.reg[Registers::R_PC as usize] + pc_offset);
        self.update_flags(r0);
    }

    fn update_flags(&mut self, r: u16) {
        let r_val = self.reg[r as usize];
        self.reg[Registers::R_COND as usize] = if r_val > 0 {
            condition_flags::FL_ZRO as u16
        } else if (r_val >> 15) > 0 {
            condition_flags::FL_NEG as u16
        } else {
            condition_flags::FL_POS as u16
        }
    }

    fn ldi(&mut self, instr: u16) {
        /* destination register (DR) */
        let r0 = (instr >> 9) & 0x7;
        /* PCoffset 9*/
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        /* add pc_offset to the current PC, look at that memory location to get the final address */
        let read = self.mem_read(self.reg[Registers::R_PC as usize] + pc_offset);
        self.reg[r0 as usize] = self.mem_read(read);
        self.update_flags(r0);
    }

    fn ldr(&mut self, instr: u16) {
        // 0x40 ? 64 / 16 =  4 ???
        let offset: u16 = sign_extend(instr & 0b11_1111, 6);

        let dr = (instr >> 9) & 0b111;
        let baser = (instr >> 6) & 0b111;

        self.reg[dr as usize] = self.mem_read(self.reg[baser as usize] + offset);
        self.update_flags(dr);
    }

    fn lea(&mut self, instr: u16) {
        /* destination register (DR) */
        let r0 = (instr >> 9) & 0x7;
        /* PCoffset 9*/
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        /* add pc_offset to the current PC, look at that memory location to get the final address */
        self.reg[r0 as usize] = self.reg[Registers::R_PC as usize] + pc_offset;
        self.update_flags(r0);
    }

    fn st(&mut self, instr: u16) {
        let sr = (instr >> 9) & 0x7;
        /* PCoffset 9*/
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        self.mem_write(
            self.reg[Registers::R_PC as usize] + pc_offset,
            self.reg[sr as usize],
        );
    }

    fn sti(&mut self, instr: u16) {
        let sr = (instr >> 9) & 0x7;
        /* PCoffset 9*/
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        let read = self.mem_read(self.reg[Registers::R_PC as usize] + pc_offset);
        self.mem_write(read, self.reg[sr as usize]);
    }

    fn str(&mut self, instr: u16) {
        let sr = (instr >> 9) & 0x7;
        /* PCoffset 6*/
        let offset: u16 = sign_extend(instr & 0b11_1111, 6);
        let baser = (instr >> 6) & 0b111;
        self.mem_write(self.reg[baser as usize] + offset, self.reg[sr as usize]);
    }

    fn trap(&mut self, instr: u16) {
        match trap_codes::from_integer(instr & 0xFF) {
            trap_codes::TRAP_GETC => self.get_character(),
            trap_codes::TRAP_OUT => self.out(),
            trap_codes::TRAP_PUTS => self.puts(),
            trap_codes::TRAP_IN => self.scan(),
            trap_codes::TRAP_PUTSP => self.putsp(),
            trap_codes::TRAP_HALT => self.halt()
        }
    }

    fn get_character(&mut self) {
        let input: u16 = std::io::stdin()
            .bytes()
            .next()
            .and_then(|result| result.ok())
            .map(|byte| byte as u16)
            .expect("Could not read character!");

        self.reg[Registers::R_R0 as usize] = input & 0b1111_1111; 
    }

    fn out(&mut self){
        print!("{}", self.reg[Registers::R_R0 as usize] as u8 as char);
        stdout().flush().expect("Could not print!");
    }

    fn puts(&mut self) {
        let mut addr = self.reg[Registers::R_R0 as usize];
        let mut character = self.memory[addr as usize];

        while character != 0 {
            print!("{}", (character & 0b1111_1111) as u8 as char); //Hmmmm.......
            addr = addr + 1;
            character = self.memory[addr as usize];
        }
        stdout().flush().expect("Could not print!");
    }

    // in
    fn scan(&mut self){
        print!("Enter a character: ");
        stdout().flush().expect("Could not print!");

        self.get_character();
    }

    fn putsp(&mut self){
        let mut addr = self.reg[Registers::R_R0 as usize];
        let mut character = self.memory[addr as usize];

        while character != 0 {
            print!("{}", (character & 0b1111_1111) as u8 as char);
            let second_part = (character >> 8) & 0b1111_1111;
            if second_part == 0{
                break;
            }
            print!("{}", second_part as u8 as char);
            addr = addr + 1;
            character = self.memory[addr as usize];
        }
        stdout().flush().expect("Could not print!"); 
    }

    fn halt(&mut self){
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
        reg: [0; Registers::R_COUNT as usize],
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
