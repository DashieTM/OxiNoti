#![feature(cell_update)]
#![feature(string_remove_matches)]
use std::{env, fs, path::PathBuf};

use directories_next as dirs;
use ui::initialize_ui;

mod daemon;
mod ui;

fn main() {
    let mut css_string = "".to_string();
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let mut argiter = args.iter();
        argiter.next().unwrap();
        match argiter.next().unwrap().as_str() {
            "--css" => {
                let next = argiter.next();
                if next.is_some() {
                    css_string = next.unwrap().clone();
                }
            }
            _ => {
                print!(
                    "usage:
    --css: use a specific path to load a css style sheet.
    --help: show this message.\n"
                );
            }
        }
    } else {
        css_string = create_config_dir().to_str().unwrap().into();
        println!("{css_string}");
    }

    initialize_ui(css_string);
}

fn create_config_dir() -> PathBuf {
    let maybe_config_dir = dirs::ProjectDirs::from("com", "dashie", "oxidash");
    if maybe_config_dir.is_none() {
        panic!("Could not get config directory");
    }
    let config = maybe_config_dir.unwrap();
    let config_dir = config.config_dir();
    if !config_dir.exists() {
        fs::create_dir(config_dir).expect("Could not create config directory");
    }
    let file_path = config_dir.join("style.css");
    if !file_path.exists() {
        fs::File::create(&file_path).expect("Could not create css config file");
        fs::write(
            &file_path,
            "#MainWindow {
                border-radius: 10px;
            }",
        )
        .expect("Could not write default values");
    }
    file_path
}
