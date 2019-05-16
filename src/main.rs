use std::env;
use std::fs::File;

mod lc3_vm;

fn main() {
    let mut vm = lc3_vm::VM::new();

    let args: Vec<String> = env::args().collect();
    let location = &args[1];
    let program = File::open(location).expect("Could not open program");

    vm.start(program);
}
