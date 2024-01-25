use crossterm::style::Color;
use pep440_rs::{parse_version_specifiers, Version};
use serde::{Deserialize, Serialize};
use std::{process::exit, str::FromStr};
use terminal_menu::{
    back_button, button, label, list, menu, mut_menu, run, scroll, submenu,
};
use util::PythonVersion;

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

fn main() {
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
        label(format!(
            "Your next MCDR installer. ver {}",
            crate_version!()
        )),
        label("_"),
        label("Environment Check:"),
    ]);
    let mut config_menu: Vec<terminal_menu::TerminalMenuItem> = vec![
        label("Configurations"),
        label("_"),
        back_button("Back"),
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

    println!("Fetching MCDReforged metadata...");
    let mcdr_data = get_mcdr_data();
    if mcdr_data.is_err() {
        println!("Error: {}", mcdr_data.err().unwrap());
        util::panic_pause();
        return;
    }
    let mcdr_data = mcdr_data.unwrap();

    match util::check_python(&mcdr_data) {
        Ok(version) => {
            main_menu
                .push(label("  Python installed: ".to_string() + &version).colorize(Color::Green));
            python_installed = true;
        }
        Err(PythonVersion::NotFound) => {
            main_menu.push(label("  Python not detected.").colorize(Color::DarkYellow));
            main_menu.push(label("    If you have python installed, It is recommended to uninstall it before continue.").colorize(Color::DarkYellow));
        }
        Err(PythonVersion::Outdated) => {
            main_menu.push(label("  Python outdated").colorize(Color::Red));
            main_menu.push(
                label("    It is recommended to uninstall it before continue.")
                    .colorize(Color::Red),
            );
        }
    }

    if python_installed == true {
        match util::check_module(&mcdr_data) {
            Ok(version) => {
                main_menu.push(
                    label(format!("  MCDReforged installed: {}", version)).colorize(Color::Green),
                );
                module_installed = true;
            }
            Err(MCDRResult::NoMCDR) => {
                main_menu.push(label("  MCDReforged not detected.").colorize(Color::DarkYellow));
            }
            Err(MCDRResult::Outdated) => {
                main_menu
                    .push(label("  MCDReforged: Installed (Outdated)").colorize(Color::DarkYellow));
                module_outdated = true;
            }
            Err(MCDRResult::NoPip) => {
                println!("We found a Python but with no pip. Uninstall it before continue.");
                util::panic_pause();
                return;
            }
        }
    } else {
        println!("Fetching Python versions...");
        let mut python_versions: Vec<String> = Vec::new();
        let resp = match reqwest::blocking::get(util::python_url(None)) {
            Ok(r) => r.json::<Vec<Registry>>(),
            Err(e) => {
                println!("Error fetching: {}", e);
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
                println!("Error parsing response: {}", e);
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
        config_menu.push(scroll("Python version", python_versions));
    }

    if util::check_initialized() {
        main_menu.push(label("  MCDReforged initialized").colorize(Color::Green));
        module_initialized = true;
    } else {
        if util::check_empty_folder() {
            main_menu.push(label("  Folder is not empty.").colorize(Color::Red));
            main_menu.push(
                label("    It's advisable to run this in an empty folder.").colorize(Color::Red),
            )
        }
    }

    main_menu.push(label("_"));
    if !python_installed {
        main_menu.push(button("Install Python"));
        main_menu.push(submenu("Configure installation", config_menu));
    } else if !module_installed {
        main_menu.push(button("Install MCDReforged"));
        config_menu.push(list("Initialize after install", vec!["yes", "no"]));
        main_menu.push(submenu("Configure installation", config_menu));
    } else {
        if !module_initialized {
            main_menu.push(button("Initialize MCDReforged"));
        }
        if module_outdated {
            main_menu.push(button("Upgrade MCDReforged"));
        }
        main_menu.push(button("Install / Upgrade PyPI modules"));
        main_menu.push(button("Open Console"));
    }

    main_menu.push(back_button("Exit"));

    println!("Displaying menu...");

    let menu = menu(main_menu);
    run(&menu);

    let mut menu_ref = mut_menu(&menu);

    match menu_ref.selected_item_name() {
        "Install Python" => {
            let submenu = menu_ref.get_submenu("Configure installation");
            let version = submenu.selection_value("Python version");
            uncheck(perform::install_python(version.to_string()));
            cprintln(
                Color::Cyan,
                "
* Python installed successfully.
* You may close this window, and run MCDReee again to install MCDReforged.
",
            );
        }
        "Install MCDReforged" => {
            uncheck(perform::install_mcdr());
            let submenu = menu_ref.get_submenu("Configure installation");
            if submenu.selection_value("Initialize after install") == "yes" {
                uncheck(perform::initilize_mcdr());
            } else {
                cprintln(
                    Color::Cyan,
                    "
* MCDReforged installed successfully.
* You may restart MCDReee to initialize it, or do it yourself.
",
                );
            }
            cprintln(
                Color::Cyan,
                "
* Keep MCDReee to upgrade MCDReforged and/or install pip modules,
* or feel free to delete it.
",
            );
        }
        "Initialize MCDReforged" => {
            uncheck(perform::initilize_mcdr());
        }
        "Upgrade MCDReforged" => {
            uncheck(perform::install_mcdr());
            cprintln(
                Color::Cyan,
                "
* MCDReforged has been upgraded.
* Running instances will not be affected before restart.
",
            );
        }
        "Install / Upgrade PyPI modules" => {
            uncheck(perform::install_modules());
        }
        "Open Console" => {
            uncheck(perform::open_console());
            exit(0); // !fixme: wrong handle
        }
        _ => {
            exit(0);
        }
    }
    util::pause();
}
