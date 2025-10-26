pub mod command;
pub mod parse;

use crate::shell::{command::execute_command, parse::parse_command};
use colored::*;
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use indicatif::{ProgressBar, ProgressStyle};
use reedline::{DefaultCompleter, DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use std::{
    fs::ReadDir,
    io::{self, stdout, Write},
    path::PathBuf,
    thread,
    time::Duration,
};

pub fn start_shell() {
    boot_animation();

    let username = whoami::username();
    let hostname = whoami::hostname();
    let mut current_dir = String::from("/");

    println!(
        "{}",
        "Type 'help' for available commands. Use ↑↓ for history, Tab for auto-completion.\n"
            .bright_black()
    );

    // 初始化 reedline
    let histroy_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minifs_history");

    let mut line_editor = Reedline::create().with_history(Box::new(
        reedline::FileBackedHistory::with_file(100, histroy_path.clone()).unwrap(),
    ));

    // 命令补全
    let commands = vec![
        "help", "ls", "pwd", "mkdir", "rmdir", "create", "rm", "cd", "read", "write", "stat",
        "format", "exit",
    ];
    let completer = reedline::DefaultCompleter::new_with_wordlen(commands.clone(), 2);
    line_editor = line_editor.with_completer(Box::new(completer));

    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::BasicLeft(format!(
            "{}@{}",
            username.green().bold(),
            hostname.cyan().bold()
        )),
        DefaultPromptSegment::BasicRight("MiniFS".bright_blue().bold().to_string()),
    );

    loop {
        let full_prompt = format!(
            "{}:{}> ",
            format!("{}@{}", username, hostname).green(),
            current_dir.blue()
        );

        let input = line_editor.read_line(&prompt.clone().with_new_prompt_left(full_prompt));

        match input {
            Ok(Signal::Success(buffer)) => {
                let trimmed = buffer.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match parse_command(trimmed) {
                    Some(cmd) => {
                        if let Err(e) = execute_command(&cmd, &mut current_dir) {
                            println!("{} {}", "❌ Error:".red().bold(), e);
                        }
                        if matches!(cmd, command::Command::Exit) {
                            println!("{}", "👋 Bye!".bright_yellow());
                            break;
                        }
                    }
                    None => println!(
                        "{}",
                        "⚠️  Unknown command. Type 'help' for command list.".yellow()
                    ),
                }
            }
            Ok(Signal::CtrlC) => {
                println!();
                continue;
            }
            Ok(Signal::CtrlD) => {
                println!("{}", "Exiting MiniFS...".yellow());
                break;
            }
            Err(e) => {
                println!("Error reading line: {}", e);
                break;
            }
        }
    }

    println!("{}", "GoodBye!".bright_yellow());
}

///动态欢迎动画
fn boot_animation() {
    let mut stdout = stdout();

    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();
    println!("{}", "[MiniFS Booting...]".bright_yellow().bold());
    thread::sleep(Duration::from_millis(300));

    let steps = vec![
        "🧠 Initializing virtual disk...",
        "⚙️  Mounting file system...",
        "📁 Loading shell...",
    ];

    for step in steps {
        println!("{}", step);
        thread::sleep(Duration::from_millis(600));
    }

    // 模拟进度条
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::with_template("[{bar:40.cyan/blue}] {pos:>3}% {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    for i in 0..100 {
        pb.set_position(i);
        thread::sleep(Duration::from_millis(15));
    }
    pb.finish_with_message("✅ Ready!");

    thread::sleep(Duration::from_millis(400));
    execute!(
        stdout,
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        SetForegroundColor(Color::Cyan),
        Print("Welcome to MiniFS v0.3.0\n"),
        ResetColor
    )
    .unwrap();
}
