use crossterm::style::Color;
use pep440_rs::{parse_version_specifiers, Version};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::{process::exit, str::FromStr};
use terminal_menu::{back_button, button, label, list, menu, mut_menu, run, scroll, submenu};
use util::PythonVersion;

#[macro_use]
extern crate rust_i18n;

use crate::{
    perform::uncheck,
    util::{cprintln, get_mcdr_data, MCDRResult},
};

mod perform;
mod util;

#[derive(Debug, Serialize, Deserialize)]
struct Registry {
    id: String,
    name: String,
    #[serde(rename = "type")]
    path_type: String,
    url: String,
}

macro_rules! crate_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

i18n!("locales");

fn main() {
    if sys_locale::get_locale()
        .unwrap_or_default()
        .starts_with("zh")
    {
        rust_i18n::set_locale("zh-CN");
    }
    let mut main_menu: Vec<terminal_menu::TerminalMenuItem> = vec![];
    r"
   __  ___________  ___             
  /  |/  / ___/ _ \/ _ \___ ___ ___ 
 / /|_/ / /__/ // / , _/ -_) -_) -_)
/_/  /_/\___/____/_/|_|\__/\__/\__/ 
"
    .lines()
    .for_each(|line| main_menu.push(label(line)));
    main_menu.extend([
        label(t!("desc", "version" => crate_version!())),
        label("*"),
        label(t!("check.env.title")),
    ]);
    let mut config_menu: Vec<terminal_menu::TerminalMenuItem> = vec![
        label(t!("menu.config.title")),
        label("::"),
        back_button(t!("menu.back")),
        label(""),
    ];

    // return;

    /*
        python_installed && module_installed:
            Run Console / Initialize MCDReforged
        else:
            Perform Install / Configure Installation
    */
    let mut python_installed = false;
    let mut module_installed = false;
    let mut module_outdated = false;
    let mut module_initialized = false;

    println!("{}", t!("fetch.mcdr"));
    let mcdr_data = get_mcdr_data();
    if mcdr_data.is_err() {
        println!("{}", t!("fetch.error", "err" => mcdr_data.err().unwrap()));
        util::panic_pause();
        return;
    }
    let mcdr_data = mcdr_data.unwrap();

    match util::check_python(&mcdr_data) {
        Ok(version) => {
            main_menu.push(
                label(format!("  {}", t!("check.py.installed", "ver" => version)))
                    .colorize(Color::Green),
            );
            python_installed = true;
        }
        Err(PythonVersion::NotFound) => {
            main_menu.push(label(format!("  {}", t!("check.py.nil"))).colorize(Color::DarkYellow));
            main_menu.push(
                label(format!("    {}", t!("check.py.nil_desc"))).colorize(Color::DarkYellow),
            );
        }
        Err(PythonVersion::Outdated) => {
            main_menu.push(label(format!("  {}", t!("check.py.outdated"))).colorize(Color::Red));
            main_menu
                .push(label(format!("    {}", t!("check.py.outdated_desc"))).colorize(Color::Red));
        }
    }

    if python_installed == true {
        match util::check_module(&mcdr_data) {
            Ok(version) => {
                main_menu.push(
                    label(format!(
                        "  {}",
                        t!("check.mcdr.installed", "ver" => version)
                    ))
                    .colorize(Color::Green),
                );
                module_installed = true;
            }
            Err(MCDRResult::NoMCDR) => {
                main_menu
                    .push(label(format!("  {}", t!("check.mcdr.nil"))).colorize(Color::DarkYellow));
            }
            Err(MCDRResult::Outdated) => {
                main_menu.push(
                    label(format!("  {}", t!("check.mcdr.outdated"))).colorize(Color::DarkYellow),
                );
                module_outdated = true;
            }
            Err(MCDRResult::NoPip) => {
                println!("{}", t!("check.mcdr.no_pip"));
                util::panic_pause();
                return;
            }
        }
    } else {
        println!("{}", t!("fetch.py"));
        let mut python_versions: Vec<String> = Vec::new();
        let resp = match reqwest::blocking::get(util::python_url(None)) {
            Ok(r) => r.json::<Vec<Registry>>(),
            Err(e) => {
                println!("{}", t!("fetch.error", "err" => e.to_string()));
                util::panic_pause();
                return;
            }
        };

        match resp {
            Ok(r) => {
                for i in r {
                    if &i.name[..1] == "3" {
                        let name = i.name[..i.name.len() - 1].to_string();
                        if parse_version_specifiers(&mcdr_data.requires_python)
                            .unwrap()
                            .iter()
                            .all(|s| s.contains(&Version::from_str(&name).unwrap()))
                        {
                            python_versions.push(name);
                        }
                    }
                }
            }
            Err(e) => {
                println!("{}", t!("fetch.parse_err", "err" => e.to_string()));
                util::panic_pause();
                return;
            }
        };

        // higher version first
        python_versions.sort_by(|a, b| {
            if Version::from_str(a).unwrap() > Version::from_str(b).unwrap() {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });

        /*
            make the second latest version the first
            for example: [1.2.3, 1.2.2, 1.1.1, 1.1.0, 1.0] -> [1.1.1, ...]
        */
        let latest = Version::from_str(&python_versions[0]).unwrap().release[1];
        for i in 0..python_versions.len() {
            if Version::from_str(&python_versions[i]).unwrap().release[1] != latest {
                let x = python_versions[i].to_owned();
                python_versions.remove(i);
                python_versions.insert(0, x);
                break;
            }
        }
        config_menu.push(scroll(t!("menu.config.py_ver"), python_versions));
    }

    if util::check_initialized() {
        main_menu.push(label(format!("  {}", t!("check.mcdr.initialized"))).colorize(Color::Green));
        module_initialized = true;
    } else {
        if util::check_empty_folder() {
            main_menu.push(label(format!("  {}", t!("check.env.not_blank"))).colorize(Color::Red));
            main_menu
                .push(label(format!("    {}", t!("check.env.blank_desc"))).colorize(Color::Red))
        }
    }

    main_menu.push(label("::"));
    if !python_installed {
        main_menu.push(button(t!("menu.main.install_py")));
        main_menu.push(submenu(t!("menu.main.configure"), config_menu));
    } else if !module_installed {
        main_menu.push(button(t!("menu.main.install_mcdr")));
        config_menu.push(list(t!("menu.config.init"), vec!["yes", "no"]));
        main_menu.push(submenu(t!("menu.main.configure"), config_menu));
    } else {
        if !module_initialized {
            main_menu.push(button(t!("menu.main.init_mcdr")));
        }
        if module_outdated {
            main_menu.push(button(t!("menu.main.upgrade_mcdr")));
        }
        main_menu.push(button(t!("menu.main.pypi")));
        main_menu.push(button(t!("menu.main.console")));
    }

    main_menu.push(back_button(t!("menu.exit")));

    println!("{}", t!("fetch.menu"));

    let menu = menu(main_menu);
    run(&menu);

    let mut menu_ref = mut_menu(&menu);

    let selected = menu_ref.selected_item_name();

    // match menu_ref.selected_item_name() {
    if selected == t!("menu.main.install_py") {
        let submenu = menu_ref.get_submenu(&t!("menu.main.configure"));
        let version = submenu.selection_value(&t!("menu.config.py_ver"));
        uncheck(perform::install_python(version.to_string()));
        cprintln(Color::Cyan, &t!("message.py_install"));
    } else if selected == t!("menu.main.install_mcdr") {
        uncheck(perform::install_mcdr());
        let submenu = menu_ref.get_submenu(&t!("menu.main.configure"));
        if submenu.selection_value(&t!("menu.config.init")) == "yes" {
            uncheck(perform::initilize_mcdr());
        } else {
            cprintln(Color::Cyan, &t!("message.mcdr_install"));
        }
        cprintln(Color::Cyan, &t!("message.setup_done"));
    } else if selected == t!("menu.main.init_mcdr") {
        uncheck(perform::initilize_mcdr());
    } else if selected == t!("menu.main.upgrade_mcdr") {
        uncheck(perform::install_mcdr());
        cprintln(Color::Cyan, &t!("message.mcdr_upgrade"));
    } else if selected == t!("menu.main.pypi") {
        uncheck(perform::install_modules());
    } else if selected == t!("menu.main.console") {
        uncheck(perform::open_console());
        exit(0); // !fixme: wrong handle
    } else {
        exit(0);
    }
    util::pause();
}
