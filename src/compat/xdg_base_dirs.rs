//!
//!
//! ## XDG Base Directory (0.8)
//! The module handles all to do with the XDG Base Diretory Standard.
//!
//! View the original spec at [specification.freedesktop.org](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html).
//!

use std::{env, path::PathBuf};

pub trait BaseDir {
    ///
    /// Return the label of the corresponding environment
    /// variable for this base directory.
    ///
    fn env_variable(&self) -> &'static str;

    ///
    /// Return the default path for this base directory.
    /// Called when:
    /// * The folder at the explicitly path does not exist; or
    /// * The corresponding environment vaiable is not set.  
    ///
    ///
    fn default(&self) -> PathBuf;

    ///
    /// Returns the path of the base directory.
    /// Handles any error cases.
    ///
    ///
    fn path(&self) -> PathBuf {
        env::var(self.env_variable())
            .ok()
            .and_then(|p| {
                let p = PathBuf::from(p);
                p.exists().then_some(p)
            })
            .unwrap_or(self.default())
    }
}

///
/// Wrapper enum over the XDG base directories.
///
pub enum XdgBaseDir {
    ///
    /// From `$XDG_DATA_HOME`:
    /// > \[The] base directory relative to which user-specific data files should be stored.
    ///
    Data,

    ///
    /// From `$XDG_CONFIG_HOME`:
    /// > \[The] base directory relative to which user-specific configuration files should be stored.
    ///
    Config,

    ///
    /// From `$XDG_STATE_HOME`:
    /// > \[The] base directory relative to which user-specific state files should be stored.
    /// >
    /// > Contains state data that should persist between (application) restarts, but that is not important or portable enough to the user that it should be stored in $XDG_DATA_HOME. It may contain:
    /// > * actions history (logs, history, recently used files, …)
    /// > * current state of the application that can be reused on a restart (view, layout, open files, undo history, …)
    ///
    State,
}

impl BaseDir for XdgBaseDir {
    fn env_variable(&self) -> &'static str {
        match self {
            Self::Data => "XDG_DATA_HOME",
            Self::Config => "XDG_CONFIG_HOME",
            Self::State => "XDG_STATE_HOME",
        }
    }

    fn default(&self) -> PathBuf {
        let mut path: PathBuf = env::var("HOME").expect("NO $HOME SET! YOU MONSTER!").into();

        if !path.exists() {
            panic!("$HOME INEXISTANT -- PANIC!");
        }

        match self {
            XdgBaseDir::Data => path.push(".local/share"),
            XdgBaseDir::Config => path.push(".config"),
            XdgBaseDir::State => path.push(".local/state"),
        };

        path
    }
}
