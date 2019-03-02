use std::env;
use std::fs::File;

mod lc3_vm;

fn main() {
    //TODO ???? disable and re-enable input buffering?????

    let mut vm = crate::lc3_vm::lc3_vm::VM::new();

    let args: Vec<String> = env::args().collect();
    let location = &args[1];
    let mut program = File::open(location).expect("Could not open program"); //TODO print file unable to open TODO mut

    vm.start(program);
}
