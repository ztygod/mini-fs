use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::error::Error;
use std::{thread, time::Duration};

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
        Command::Help => print_help(),
        Command::Ls => {
            println!("📂  .");
            println!("📁  ..");
            println!("📄  example.txt");
        }
        Command::Pwd => println!("📍 {}", current_dir.cyan()),
        Command::Mkdir(name) => {
            println!(
                "✅ Created directory: {}",
                format!("{}/{}", current_dir, name).green()
            );
        }
        Command::Rmdir(name) => {
            println!(
                "🗑️ Removed directory: {}",
                format!("{}/{}", current_dir, name).red()
            );
        }
        Command::Create(name) => {
            println!(
                "📝 Created file: {}",
                format!("{}/{}", current_dir, name).green()
            );
        }
        Command::Rm(name) => {
            println!(
                "❌ Deleted file: {}",
                format!("{}/{}", current_dir, name).red()
            );
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
            println!("📂 Moved to {}", current_dir.blue());
        }
        Command::Read(file) => {
            println!(
                "📖 Reading file: {}",
                format!("{}/{}", current_dir, file).cyan()
            );
            println!("{}", "(mock content: Hello World)".bright_black());
        }
        Command::Write(file, content) => {
            println!(
                "✏️  Writing to {}",
                format!("{}/{}", current_dir, file).cyan()
            );
            println!("{} {}", "✅ Content:".green(), content);
        }
        Command::Stat(file) => {
            println!(
                "{}\n{}: {}\n{}: {}\n{}: {} bytes\n",
                "📊 File Info".bright_yellow().bold(),
                "Name".blue(),
                file,
                "Type".blue(),
                "File",
                "Size".blue(),
                42
            );
        }
        Command::Format => {
            println!("💾 Formatting virtual disk...");
            let pb = ProgressBar::new(100);
            pb.set_style(
                ProgressStyle::with_template("[{bar:40.green/black}] {pos:>3}% {msg}")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            for i in 0..=100 {
                pb.set_position(i);
                thread::sleep(Duration::from_millis(20));
            }
            pb.finish_with_message("✅ Disk formatted successfully!");
        }
        Command::Exit => println!("{}", "👋 Exiting MiniFS shell...".yellow().bold()),
    }

    Ok(())
}

fn print_help() {
    println!("{}", "📘 MiniFS Commands".bright_cyan().bold());
    println!(
        "{}",
        "
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
"
        .bright_black()
    );
}
