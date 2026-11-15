// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Wakunguma Kalimukwa

use crate::Result;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::Read;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use tracing::info;

#[derive(Debug, Clone)]
pub enum SettingsBuffer {
    File(PathBuf),
    // The in-memory buffer is used to make testing easier.
    Memory,
}

#[derive(Clone)]
pub struct SettingsStore {
    buffer: SettingsBuffer,
    inner: Rc<RefCell<Settings>>,
}

impl SettingsStore {
    pub fn set_currency_code(&self, code: &str) -> Result<()> {
        self.settings_mut().currency_code = code.to_owned();
        self.write()?;
        info!("Updated currency code to {code}");
        Ok(())
    }

    pub fn set_font_family(&self, family: &str) -> Result<()> {
        self.settings_mut().appearance.font_family = family.to_owned();
        self.write()?;
        info!("Updated font family to {family}");
        Ok(())
    }

    pub fn set_color_scheme(&self, scheme: ColorScheme) -> Result<()> {
        self.settings_mut().appearance.color_scheme = scheme;
        self.write()?;
        info!("Updated color scheme to {scheme}");
        Ok(())
    }

    /// Returns the curreny code.
    pub fn currency_code(&self) -> String {
        self.settings().currency_code.clone()
    }

    /// Returns the font family.
    pub fn font_family(&self) -> String {
        self.settings().appearance.font_family.clone()
    }

    /// Returns the color scheme.
    pub fn color_scheme(&self) -> ColorScheme {
        self.settings().appearance.color_scheme
    }

    fn settings(&self) -> Ref<'_, Settings> {
        self.inner.borrow()
    }

    fn settings_mut(&self) -> RefMut<'_, Settings> {
        self.inner.borrow_mut()
    }

    /// Writes the settings to the file. Noop if the `SettingsStore` uses an in-memory buffer.
    fn write(&self) -> Result<()> {
        if let SettingsBuffer::File(path) = &self.buffer {
            let settings = self.settings();
            let contents = toml::to_string(&*settings)?;
            fs::write(path, contents)?;
        }
        Ok(())
    }

    // Used for testing
    #[allow(unused)]
    pub fn open_in_memory() -> SettingsStore {
        SettingsStore {
            buffer: SettingsBuffer::Memory,
            inner: Rc::new(RefCell::new(Settings::default())),
        }
    }

    /// Opens or initializes a `SettingsStore` from the provided path.
    pub fn open(path: impl AsRef<Path>) -> Result<SettingsStore> {
        match File::open(&path) {
            Ok(mut file) => {
                info!("Loading settings from {:?}", path.as_ref());

                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                let settings: Settings = toml::from_slice(&buffer)?;
                let store = SettingsStore {
                    buffer: SettingsBuffer::File(path.as_ref().to_path_buf()),
                    inner: Rc::new(RefCell::new(settings)),
                };
                Ok(store)
            }
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(err.into());
                }

                let settings = Settings::default();
                let contents = toml::to_string(&settings)?;
                fs::write(&path, contents)?;

                info!("Initialised settings at {:?}", path.as_ref());
                let store = SettingsStore {
                    buffer: SettingsBuffer::File(path.as_ref().to_path_buf()),
                    inner: Rc::new(RefCell::new(settings)),
                };
                Ok(store)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

impl From<slint::language::ColorScheme> for ColorScheme {
    fn from(value: slint::language::ColorScheme) -> Self {
        match value {
            slint::language::ColorScheme::Dark => Self::Dark,
            _ => Self::Light,
        }
    }
}

impl From<ColorScheme> for slint::language::ColorScheme {
    fn from(value: ColorScheme) -> Self {
        match value {
            ColorScheme::Dark => slint::language::ColorScheme::Dark,
            ColorScheme::Light => slint::language::ColorScheme::Light,
        }
    }
}

impl Display for ColorScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorScheme::Light => write!(f, "Light"),
            ColorScheme::Dark => write!(f, "Dark"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    /// The ISO 4217 currency code.
    currency_code: String,
    #[serde(default)]
    appearance: Appearance,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Appearance {
    font_family: String,
    #[serde(default)]
    color_scheme: ColorScheme,
}

impl Default for Appearance {
    fn default() -> Self {
        let default_font = if cfg!(target_os = "windows") {
            "Segoe UI"
        } else if cfg!(target_os = "macos") {
            "SF Pro"
        } else {
            // Slint will use the default font
            ""
        };

        Self {
            font_family: String::from(default_font),
            color_scheme: ColorScheme::default(),
        }
    }
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            currency_code: String::from("USD"),
            appearance: Appearance::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use tempfile::tempdir;

    use super::*;
    use crate::SettingsStore;
    use std::fs;

    #[test]
    fn init_settings_if_not_found() -> Result<()> {
        let temp = tempdir()?;
        let path = temp.path().join("settings.toml");
        SettingsStore::open(&path)?;
        assert!(fs::exists(path)?);
        Ok(())
    }
}
