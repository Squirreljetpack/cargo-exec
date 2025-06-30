extern crate exec;

use std::env;
use std::process;

fn main() {
    let argv: Vec<String> = env::args().skip(2).collect();
    if argv.is_empty() {
        eprintln!("Error: Must specify command to execute");
        process::exit(1);
    }

    let err = if argv[0] == "-s" {
        if argv.len() < 2 {
            eprintln!("Error: -s requires a shell and a command");
            process::exit(1);
        }
        let shell = &argv[1];
        let script = &argv[2];
        let script_args = &argv[3..];
        
        let mut command = exec::Command::new(shell);
        command.arg("-c");
        command.arg(format!("f() {{ {script}; }}; f \"$@\""));
        command.arg("_"); // dummy $0
        command.args(script_args);
        command.exec()
    } else {
        exec::Command::new(&argv[0]).args(&argv[1..]).exec()
    };

    eprintln!("Error: {err}");
    process::exit(1);
}