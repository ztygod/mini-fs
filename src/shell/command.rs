use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::error::Error;
use std::{thread, time::Duration};

use crate::fs::directory::DirEntryType;
use crate::fs::FileSystem;

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

pub fn execute_command(
    cmd: &Command,
    current_dir: &mut String,
    fs: &mut FileSystem, // 添加 FileSystem 参数
) -> Result<(), Box<dyn Error>> {
    match cmd {
        Command::Help => print_help(),
        Command::Ls => match fs.list_dir(current_dir) {
            Ok(entries) => {
                for e in entries {
                    match e.entry_type {
                        DirEntryType::Directory => println!("📁  {}", e.name),
                        DirEntryType::File => println!("📄  {}", e.name),
                    }
                }
            }
            Err(e) => println!("❌ {}", e),
        },
        Command::Pwd => println!("📍 {}", current_dir.cyan()),
        Command::Mkdir(name) => match fs.create_dir(current_dir, name) {
            Ok(_) => println!(
                "✅ Created directory: {}",
                format!("{}/{}", current_dir, name).green()
            ),
            Err(e) => println!("❌ {}, current_dir: {}, name: {}", e, current_dir, name),
        },
        Command::Rmdir(name) => match fs.delete_dir(current_dir, name) {
            Ok(_) => println!(
                "🗑️ Removed directory: {}",
                format!("{}/{}", current_dir, name).red()
            ),
            Err(e) => println!("❌ {}", e),
        },
        Command::Create(name) => match fs.create_file(current_dir, name, &[]) {
            Ok(_) => println!(
                "📝 Created file: {}",
                format!("{}/{}", current_dir, name).green()
            ),
            Err(e) => println!("❌ {}", e),
        },
        Command::Rm(name) => match fs.delete_file(current_dir, name) {
            Ok(_) => println!(
                "❌ Deleted file: {}",
                format!("{}/{}", current_dir, name).red()
            ),
            Err(e) => println!("❌ {}", e),
        },
        Command::Cd(path) => {
            if path == ".." {
                if let Some(pos) = current_dir.rfind('/') {
                    current_dir.truncate(pos);
                    if current_dir.is_empty() {
                        *current_dir = "/".to_string();
                    }
                }
            } else {
                // 验证目录是否存在
                let target_path = if current_dir == "/" {
                    format!("/{}", path)
                } else {
                    format!("{}/{}", current_dir, path)
                };

                if fs.find_inode(&target_path).is_ok() {
                    if current_dir != "/" {
                        current_dir.push('/');
                    }
                    current_dir.push_str(path);
                } else {
                    println!("❌ Directory not found: {}", path);
                    return Ok(());
                }
            }
            println!("📂 Moved to {}", current_dir.blue());
        }
        Command::Read(file) => match fs.read_file(current_dir, file) {
            Ok(content) => {
                println!(
                    "📖 Reading file: {}",
                    format!("{}/{}", current_dir, file).cyan()
                );
                if let Ok(content_str) = String::from_utf8(content) {
                    println!("{}", content_str);
                } else {
                    println!("<binary data>");
                }
            }
            Err(e) => println!("❌ {}", e),
        },
        Command::Write(file, content) => {
            match fs.create_file(current_dir, file, content.as_bytes()) {
                Ok(_) => {
                    println!(
                        "✏️  Writing to {}",
                        format!("{}/{}", current_dir, file).cyan()
                    );
                    println!("{} {}", "✅ Content:".green(), content);
                }
                Err(e) => println!("❌ {}", e),
            }
        }
        Command::Stat(file) => match fs.stat(current_dir, file) {
            Ok((inode_id, file_type, size)) => {
                println!(
                    "{}\n{}: {}\n{}: {}\n{}: {} bytes\n",
                    "📊 File Info".bright_yellow().bold(),
                    "Name".blue(),
                    file,
                    "Type".blue(),
                    file_type,
                    "Size".blue(),
                    size
                );
            }
            Err(e) => println!("❌ {}", e),
        },
        Command::Format => match fs.format() {
            Ok(_) => {
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
            Err(e) => println!("❌ Format failed: {}", e),
        },
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
