use crate::shell::command::Command;

pub fn parse_command(input: &str) -> Option<Command> {
    let tokens: Vec<&str> = input.trim().split_ascii_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let cmd = tokens[0];
    let args = &tokens[1..];

    match cmd {
        "help" => Some(Command::Help),
        "ls" => Some(Command::Ls),
        "pwd" => Some(Command::Pwd),
        "mkdir" => args.get(0).map(|&name| Command::Mkdir(name.to_string())),
        "rmdir" => args.get(0).map(|&name| Command::Rmdir(name.to_string())),
        "create" => args.get(0).map(|&name| Command::Create(name.to_string())),
        "rm" => args.get(0).map(|&name| Command::Rm(name.to_string())),
        "cd" => args.get(0).map(|&name| Command::Cd(name.to_string())),
        "read" => args.get(0).map(|&name| Command::Read(name.to_string())),
        "wirte" => {
            if args.len() >= 2 {
                Some(Command::Write(
                    args.get(0)?.to_string(),
                    args[1..].join(" "),
                ))
            } else {
                None
            }
        }
        "stat" => args.get(0).map(|&name| Command::Stat(name.to_string())),
        "format" => Some(Command::Format),
        "exit" => Some(Command::Exit),
        _ => None,
    }
}
