/*
 * Copyright (c) 2022 McSib
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::fs::{read_to_string, write};
use std::io;
use std::path::Path;
use std::process::exit;

use anyhow::{Context, Error};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};

pub(crate) mod parser;
pub(crate) mod tag;

/// Name of the configuration file.
pub(crate) const CONFIG_NAME: &str = "config.json";

/// Name of the login file.
pub(crate) const LOGIN_NAME: &str = "login.json";

/// Config that is used to do general setup.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Config {
    /// The location of the download directory.
    #[serde(rename = "downloadDirectory")]
    download_directory: String,
    /// The file naming convention (e.g "md5", "id").
    #[serde(rename = "fileNamingConvention")]
    naming_convention: String,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

impl Config {
    /// The location of the download directory.
    pub(crate) fn download_directory(&self) -> &str {
        &self.download_directory
    }

    /// The file naming convention (e.g "md5", "id").
    pub(crate) fn naming_convention(&self) -> &str {
        &self.naming_convention
    }

    /// Checks config and ensure it isn't missing.
    pub(crate) fn config_exists() -> bool {
        if !Path::new(CONFIG_NAME).exists() {
            trace!("config.json: does not exist!");
            return false;
        }

        true
    }

    /// Creates config file.
    pub(crate) fn create_config() -> Result<(), Error> {
        let json = to_string_pretty(&Config::default())?;
        write(Path::new(CONFIG_NAME), json)?;

        Ok(())
    }

    /// Get the global instance of the `Config`.
    pub(crate) fn get() -> &'static Self {
        CONFIG.get().expect("Config has not been initialized!")
    }

    /// Initializes the global `Config` instance.
    pub(crate) fn initialize() -> Result<(), Error> {
        let config = Self::load_config()?;
        CONFIG
            .set(config)
            .map_err(|_| anyhow::anyhow!("Config has already been initialized!"))?;
        Ok(())
    }

    /// Loads and returns `config` for quick management and settings.
    fn load_config() -> Result<Self, Error> {
        let config_str = read_to_string(CONFIG_NAME)
            .context(format!("Failed to read config file: {CONFIG_NAME}"))?;
        let mut config: Config =
            from_str(&config_str).context(format!("Failed to parse config file: {CONFIG_NAME}"))?;
        config.naming_convention = config.naming_convention.to_lowercase();
        let convention = ["md5", "id"];
        if !convention.contains(&config.naming_convention.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid naming convention: {}. Must be one of: [\"md5\", \"id\"]",
                config.naming_convention
            ));
        }

        Ok(config)
    }
}

impl Default for Config {
    /// The default configuration for `Config`.
    fn default() -> Self {
        Config {
            download_directory: String::from("downloads/"),
            naming_convention: String::from("md5"),
        }
    }
}

fn default_true() -> bool {
    true
}

/// `Login` contains all login information for obtaining information about a certain user.
/// This is currently only used for the blacklist.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Login {
    /// Username of user.
    #[serde(rename = "Username", default)]
    username: String,
    /// The password hash (also known as the API key) for the user.
    #[serde(rename = "APIKey", default)]
    api_key: String,
    /// Whether or not the user wishes to download their favorites.
    #[serde(rename = "DownloadFavorites", default = "default_true")]
    download_favorites: bool,
    /// Whether or not the user wishes to ignore the blacklist when downloading favorites.
    #[serde(rename = "IgnoreBlacklistOnFavorites", default = "default_true")]
    ignore_blacklist_on_favorites: bool,
}

static LOGIN: OnceCell<Login> = OnceCell::new();

impl Login {
    /// Username of user.
    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    /// The password hash (also known as the API key) for the user.
    pub(crate) fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Whether or not the user wishes to download their favorites.
    pub(crate) fn download_favorites(&self) -> bool {
        self.download_favorites
    }

    /// Whether or not the user wishes to ignore the blacklist when downloading favorites.
    pub(crate) fn ignore_blacklist_on_favorites(&self) -> bool {
        self.ignore_blacklist_on_favorites
    }

    /// Gets the global instance of [Login].
    pub(crate) fn get() -> &'static Self {
        LOGIN.get().expect("Login has not been initialized!")
    }

    /// Initializes the global `Login` instance.
    pub(crate) fn initialize() -> Result<(), Error> {
        let login = match Self::load() {
            Ok(login) => login,
            Err(e) => {
                error!("Unable to load `login.json`. Error: {e}");
                warn!(
                    "The program will use default values, but it is highly recommended to check your login.json file to \
			       ensure that everything is correct."
                );
                Login::default()
            }
        };
        LOGIN
            .set(login)
            .map_err(|_| anyhow::anyhow!("Login has already been initialized!"))?;
        Ok(())
    }

    /// Loads the login file or creates one if it doesn't exist.
    fn load() -> Result<Self, Error> {
        let login_path = Path::new(LOGIN_NAME);
        if !login_path.exists() {
            let login = Login::default();
            login.create_login()?;
            return Ok(login);
        }

        let content = read_to_string(login_path)?;
        let login: Login = from_str(&content)?;

        let expected_keys = [
            "Username",
            "APIKey",
            "DownloadFavorites",
            "IgnoreBlacklistOnFavorites",
        ];
        if expected_keys.iter().any(|key| !content.contains(key)) {
            warn!(
                "The login.json file was missing some options and has been updated with default values."
            );
            login.save_to_file()?;
        }

        Ok(login)
    }

    /// Checks if the login user and password is empty.
    pub(crate) fn is_empty(&self) -> bool {
        if self.username.is_empty() || self.api_key.is_empty() {
            return true;
        }

        false
    }

    /// Saves the login to the login file.
    fn save_to_file(&self) -> Result<(), Error> {
        write(LOGIN_NAME, to_string_pretty(self)?)?;

        Ok(())
    }

    /// Creates a new login file.
    fn create_login(&self) -> Result<(), Error> {
        self.save_to_file()?;

        info!("The login file was created.");
        info!(
            "If you wish to use your Blacklist, \
             be sure to give your username and API hash key."
        );
        info!(
            "Do not give out your API hash unless you trust this software completely, \
             always treat your API hash like your own password."
        );

        Ok(())
    }
}

impl Default for Login {
    /// The default state for the login if none exists.
    fn default() -> Self {
        Login {
            username: String::new(),
            api_key: String::new(),
            download_favorites: true,
            ignore_blacklist_on_favorites: true,
        }
    }
}

/// Exits the program after message explaining the error and prompting the user to press `ENTER`.
///
/// # Arguments
///
/// * `error`: The error message to print.
pub(crate) fn emergency_exit(error: &str) {
    info!("{error}");
    println!("Press ENTER to close the application...");

    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap_or_default();

    exit(0x00FF);
}
