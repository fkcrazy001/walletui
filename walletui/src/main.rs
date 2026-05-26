/// Application.
pub mod app;

pub mod crypto;

/// Terminal events handler.
pub mod event;

pub mod import;

pub mod storage;

pub mod tui;

pub mod ui;

pub mod update;

use app::App;
use color_eyre::{Result, eyre::eyre};
use event::{Event, EventHandler};
use import::ImportFormat;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};
use tui::Tui;
use update::update;

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "import") {
        return import_command(&args[1..]);
    }
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        print_help();
        return Ok(());
    }

    // Create an application.
    let mut app = App::new(read_master_key()?)?;

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.enter()?;

    // Start the main loop.
    while !app.should_quit {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => update(&mut app, key_event),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        };
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}

fn import_command(args: &[String]) -> Result<()> {
    let mut path = None;
    let mut format = ImportFormat::Csv;
    let mut replace = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| eyre!("--format 需要值：csv 或 tsv"))?;
                format = ImportFormat::parse(value)?;
                index += 2;
            }
            "--replace" => {
                replace = true;
                index += 1;
            }
            "--help" | "-h" => {
                print_import_help();
                return Ok(());
            }
            value if value.starts_with('-') => return Err(eyre!("未知参数：{value}")),
            value => {
                if path.is_some() {
                    return Err(eyre!("只能指定一个导入文件"));
                }
                path = Some(PathBuf::from(value));
                index += 1;
            }
        }
    }

    let path = path.ok_or_else(|| eyre!("缺少导入文件路径"))?;
    let imported = import::read_entries(&path, format)?;
    if imported.is_empty() {
        println!("没有可导入的记录。");
        return Ok(());
    }

    let master_key = read_master_key()?;
    let vault_path = storage::default_vault_path();
    let mut entries = if replace {
        Vec::new()
    } else {
        storage::load(&vault_path, &master_key)?
    };
    let old_len = entries.len();
    entries.extend(imported);
    storage::save(&vault_path, &master_key, &entries)?;

    println!(
        "导入完成：新增 {} 条，vault 当前共 {} 条。路径：{}",
        entries.len() - old_len,
        entries.len(),
        vault_path.display()
    );
    Ok(())
}

fn print_help() {
    println!(
        "walletui\n\n用法：\n  walletui                 启动 TUI\n  walletui import <file>   批量导入密码\n\n环境变量：\n  WALLETUI_MASTER          主密码\n  WALLETUI_VAULT           vault 文件路径"
    );
}

fn print_import_help() {
    println!(
        "用法：walletui import <file> [--format csv|tsv] [--replace]\n\n导入列：category,title,username,password,notes\n第一行可以是英文或中文表头。默认追加，--replace 会覆盖当前 vault。"
    );
}

fn read_master_key() -> Result<String> {
    if let Ok(value) = std::env::var("WALLETUI_MASTER")
        && !value.is_empty()
    {
        return Ok(value);
    }

    eprint!("Master password: ");
    io::stderr().flush()?;
    let value = rpassword::read_password()?;
    let value = value.trim_end_matches(['\r', '\n']).to_string();
    if value.is_empty() {
        return Err(eyre!("master password 不能为空"));
    }
    Ok(value)
}
