/*
Copyright © 2023 Fabio Lenherr

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <http://www.gnu.org/licenses/>.
*/

#![feature(cell_update)]
#![feature(string_remove_matches)]
use std::{env, fs, path::PathBuf};

use directories_next as dirs;
use ui::initialize_ui;

mod daemon;
pub mod ui;

fn main() {
    let mut config_strings: (String, String) = ("".to_string(), "".to_string());
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let mut argiter = args.iter();
        argiter.next().unwrap();
        match argiter.next().unwrap().as_str() {
            "--css" => {
                let next = argiter.next();
                if next.is_some() {
                    config_strings.0 = next.unwrap().clone();
                    config_strings.1 = "".to_string();
                }
            }
            _ => {
                print!(
                    "usage:
    --css: use a specific path to load a css style sheet.
    --help: show this message.\n"
                );
                return;
            }
        }
    } else {
        config_strings = create_config_dir(config_strings.0, config_strings.1)
    }

    initialize_ui(config_strings.0, config_strings.1);
}

fn create_config_dir(css_string: String, toml_string: String) -> (String, String) {
    let maybe_config_dir = dirs::ProjectDirs::from("com", "dashie", "oxinoti");
    if maybe_config_dir.is_none() {
        panic!("Could not get config directory");
    }
    let config = maybe_config_dir.unwrap();
    let config_dir = config.config_dir();
    if !config_dir.exists() {
        fs::create_dir(config_dir).expect("Could not create config directory");
    }
    let mut file_path: PathBuf = "".to_string().into();
    if css_string == "" {
        file_path = config_dir.join("style.css");
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
    }
    let mut config_path: PathBuf = "".to_string().into();
    if toml_string == "" {
        config_path = config_dir.join("oxinoti.toml");
        if !config_path.exists() {
            fs::File::create(&config_path).expect("Could not create config file");
            fs::write(
                &config_path,
                "timeout = 3
             dnd_override = 2",
            )
            .expect("Could not write default values");
        }
    }
    (
        file_path.to_str().unwrap().into(),
        config_path.to_str().unwrap().into(),
    )
}
