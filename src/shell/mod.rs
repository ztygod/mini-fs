pub mod command;
pub mod parse;

use std::io::{self, Write};

use crate::shell::{command::execute_command, parse::parse_command};

pub fn start_shell() {
    println!("MiniFS v0.1.0");
    println!("Using virtual disk: disk.img");
    println!("Type 'help' for command list.\n");

    let mut current_dir = String::from("/");

    loop {
        print!("{}>", current_dir);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Error: failed to read input");
            continue;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match parse_command(input) {
            Some(cmd) => {
                let result = execute_command(&cmd, &mut current_dir);
                if let Err(e) = result {
                    println!("Error: {}", e);
                }
                // exit 命令特殊处理
                if matches!(cmd, command::Command::Exit) {
                    break;
                }
            }
            None => println!("Invalid command. Type 'help' for command list."),
        }
    }

    println!("Bye!");
}
