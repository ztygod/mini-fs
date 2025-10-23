use std::error::Error;

/// 支持所有的命令
#[derive(Debug)]
pub enum Command {
    Help,
    Ls,
    Pwd,
    Mkdir(String),
    Rmdir(String),
    Create(String),
    Rm(String),
    Cd(String),
    Read(String),
    Write(String, String),
    Stat(String),
    Format,
    Exit,
}

pub fn execute_command(cmd: &Command, current_dir: &mut String) -> Result<(), Box<dyn Error>> {
    match cmd {
        Command::Help => {
            print_help();
        }
        Command::Ls => {
            println!(".   ..   [dummy files]");
        }
        Command::Pwd => {
            println!("{}", current_dir);
        }
        Command::Mkdir(name) => {
            println!("Created directory: {}/{}", current_dir, name);
        }
        Command::Rmdir(name) => {
            println!("Removed directory: {}/{}", current_dir, name);
        }
        Command::Create(name) => {
            println!("Created file: {}/{}", current_dir, name);
        }
        Command::Rm(name) => {
            println!("Removed file: {}/{}", current_dir, name);
        }
        Command::Cd(path) => {
            if path == ".." {
                if let Some(pos) = current_dir.rfind('/') {
                    current_dir.truncate(pos);
                    if current_dir.is_empty() {
                        *current_dir = "/".to_string();
                    }
                }
            } else {
                if current_dir != "/" {
                    current_dir.push('/');
                }
                current_dir.push_str(path);
            }
        }
        Command::Read(file) => {
            println!("Reading file: {}/{}", current_dir, file);
            println!("(mock content)");
        }
        Command::Write(file, content) => {
            println!("Writing to file: {}/{}", current_dir, file);
            println!("> {}", content);
        }
        Command::Stat(file) => {
            println!("Stat for file: {}/{}", current_dir, file);
            println!("inode=1, size=42 bytes, type=file");
        }
        Command::Format => {
            println!("Formatting virtual disk... done!");
        }
        Command::Exit => {
            println!("Exiting MiniFS shell...");
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"MiniFS Command List:
---------------------------------
  ls                 List files in current directory
  pwd                Print current path
  mkdir <dir>        Create directory
  rmdir <dir>        Remove directory
  create <file>      Create file
  rm <file>          Remove file
  cd <dir>           Change directory
  read <file>        Read file content
  write <file> <str> Write string into file
  stat <file>        Show file info
  format             Format virtual disk
  help               Show this help message
  exit               Quit the shell
---------------------------------"#
    );
}
