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

use serde::{self, Deserialize};
use std::fs;
use toml;

fn default_config() -> String {
    format!(
        r#"timeout = 3
        dnd_override = 2"#,
    )
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub timeout: u64,
    pub dnd_override: i32,
}

#[derive(Deserialize)]
pub struct ConfigOptional {
    timeout: Option<u64>,
    dnd_override: Option<i32>,
}

pub fn parse_config(path: &str) -> Config {
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => default_config(),
    };
    let parsed_conf: ConfigOptional = match toml::from_str(&contents) {
        Ok(d) => d,
        Err(_) => toml::from_str(&default_config()).unwrap(),
    };
    Config {
        timeout: parsed_conf.timeout.unwrap_or_else(|| 3),
        dnd_override: parsed_conf.dnd_override.unwrap_or_else(|| 2),
    }
}
