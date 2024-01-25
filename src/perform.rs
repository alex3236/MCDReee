use std::{
    fs::{self, File},
    io::{self, Write as _},
    process::{exit, Command},
};

use crossterm::style::Color;
use terminal_menu::{
    back_button, button, label, list, menu, mut_menu, run, scroll, string, submenu,
};
use thiserror::Error;

use crate::util::{self, cprintln};

#[derive(Debug, Error)]
pub enum PerformError {
    #[error("Failed to act: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed to act: Executor returned non-zero code {0}")]
    NonZeroError(i32),

    #[error("Failed to download: {0}")]
    DownloadError(#[from] util::DownloadError),
}

pub fn execute(command: &str, args: Vec<&str>) -> Result<(), PerformError> {
    match Command::new(command)
        .args(args)
        .stdout(io::stdout())
        .status()
    {
        Ok(status) => {
            if let Some(code) = status.code() {
                if code != 0 {
                    return Err(PerformError::NonZeroError(code));
                }
            }
            return Ok(());
        }
        Err(e) => return Err(PerformError::IOError(e)),
    }
}

pub fn install_python(version: String) -> Result<(), PerformError> {
    println!("{}", t!("perform.download_py", "ver" => version));
    match util::download_file(&util::python_url(Some(&version)), "python-installer.exe") {
        Ok(_) => (),
        Err(e) => return Err(PerformError::DownloadError(e)),
    }
    cprintln(
        Color::Cyan,
        "

",
    );
    let result = execute(
        "python-installer.exe",
        vec![
            "InstallAllUsers=0",
            "PrependPath=1",
            "Include_test=0",
            "SimpleInstall=1",
        ],
    );
    let _ = fs::remove_file("python-installer.exe");
    return result;
}

pub fn install_mcdr() -> Result<(), PerformError> {
    println!("{}", t!("install_mcdr"));
    execute(
        "pip",
        vec![
            "config",
            "set",
            "global.index-url",
            "https://mirrors.bfsu.edu.cn/pypi/web/simple",
        ],
    )?;
    execute("pip", vec!["install", "mcdreforged", "-U"])?;
    Ok(())
}

pub fn initilize_mcdr() -> Result<(), PerformError> {
    println!("{}", t!("perform.init_mcdr"));
    execute("python", vec!["-m", "mcdreforged", "init"])?;
    let mut f = File::create("start.bat")?;
    f.write_all(
        b"
@echo off
set JAVA_TOOL_OPTIONS=-Dfile.encoding=UTF-8
python -m mcdreforged
pause
",
    )?;
    cprintln(Color::Cyan, &t!("message.mcdr_init"));
    Ok(())
}

pub fn install_modules() -> Result<(), PerformError> {
    let mut validate = true;
    loop {
        let mut menu_vec = vec![
            // label("Install / Upgrade PyPI modules"),
            label(""),
            string(t!("menu.pypi.input"), "|none|", false),
            label(""),
            button(t!("menu.pypi.install")),
            back_button("Back"),
            label(""),
        ];
        if !validate {
            menu_vec.push(label(t!("menu.pypi.invalid")).colorize(Color::Red));
        }
        let menu = menu(menu_vec);
        run(&menu);
        let menu_ref = mut_menu(&menu);
        if menu_ref.selected_item_name() != t!("menu.pypi.install") {
            break;
        }
        let modules = menu_ref.selection_value(&t!("menu.pypi.input"));
        if util::validate_modules(modules) {
            let mut args = vec!["-m", "pip", "install", "-U"];
            args.extend(modules.split(" "));
            println!("{}", t!("menu.pypi.installing", "module" => modules));
            execute("python", args)?;
            break;
        } else {
            validate = false;
            continue;
        }
    }
    Ok(())
}

pub fn open_console() -> Result<(), PerformError> {
    Command::new("cmd")
        .args(["/c", "start", "cmd"])
        .spawn()
        .expect("Failed to start child process");
    Ok(())
}

pub fn uncheck(r: Result<(), PerformError>) {
    match r {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{e}");
            util::panic_pause();
            exit(1);
        }
    }
}
