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
use reedline::{
    default_emacs_keybindings, ColumnarMenu, DefaultCompleter, DefaultPrompt, Emacs,
    FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu,
    Signal,
};
use std::{io::stdout, path::PathBuf, thread, time::Duration};
use whoami::fallible;

pub fn start_shell() {
    boot_animation();

    let username = whoami::username();
    let hostname = fallible::hostname().unwrap();
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

    // 命令补全
    let commands = vec![
        "help", "ls", "pwd", "mkdir", "rmdir", "create", "rm", "cd", "read", "write", "stat",
        "format", "exit",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();

    let completer = Box::new(DefaultCompleter::new_with_wordlen(commands.clone(), 2));

    // 使用互动菜单从补全器中选择选项
    let menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));

    // 设置所需的键位绑定
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));
    let mut line_editor = Reedline::create()
        .with_history(Box::new(
            FileBackedHistory::with_file(100, histroy_path.clone()).unwrap(),
        ))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(menu))
        .with_edit_mode(edit_mode);

    loop {
        let left_prompt = format!(
            "{}@{}:{}",
            username.green().bold(),
            hostname.cyan().bold(),
            current_dir.blue()
        );

        let prompt = DefaultPrompt::new(
            reedline::DefaultPromptSegment::Basic(left_prompt),
            reedline::DefaultPromptSegment::Basic("MiniFS".bright_blue().bold().to_string()),
        );

        let input = line_editor.read_line(&prompt);

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
}

// 动态欢迎动画
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
