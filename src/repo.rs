use crate::error::{ParseError, UpdateError};
use crate::package::Package;
use std::env::set_current_dir;
use std::num::ParseIntError;
use std::process::Command;
use std::{
    convert::TryFrom,
    fs::{self},
    path::PathBuf,
};

pub fn get_repositories() -> Vec<Repo> {
    let repo_var = match std::env::var("PUR_PATH") {
        Ok(val) => val,
        Err(_) => {
            let repos = vec![
                "/usr/repo/pur",
                "/usr/repo/pur-community",
                "/usr/repo/unofficial",
            ];

            repos.join(":")
        }
    };

    repo_var
        .split(":")
        .map(PathBuf::from)
        .map(Repo::from)
        .collect::<Vec<Repo>>()
}

#[derive(Debug, Clone)]
pub struct InstallData {
    pub version: String,
}

#[derive(Debug)]
pub struct Repo {
    pub dir: PathBuf,
}

impl From<PathBuf> for Repo {
    fn from(path: PathBuf) -> Self {
        Self { dir: path }
    }
}

impl TryFrom<PathBuf> for InstallData {
    type Error = ParseError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let version = fs::read_to_string(path.join("version"))
            .map_err(|_| ParseError::NoVersion)?
            .chars()
            .filter(|x| !x.is_whitespace())
            .collect::<String>();

        Ok(Self { version })
    }
}

impl Repo {
    /// This method fetches all packages from the local system, using the
    /// current repository as base directory.
    ///
    /// It will loop through every directory within the repository (not recursively),
    /// and it will attempt to add every directory to the return value as a Package.
    ///
    /// Every package will be re-fetched everytime this method is called, and not cached,
    /// so it's recommended to not call this method every single time you need packages;
    /// call it somewhere globally.
    pub fn get_packages(&self) -> std::io::Result<Vec<Package>> {
        Ok(fs::read_dir(&self.dir)?
            .into_iter()
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .flat_map(|x| Package::try_from(x))
            .collect::<Vec<Package>>())
    }

    pub fn update_repository(
        &self,
        update_callback: &mut dyn FnMut(Package, InstallData) -> Result<(), UpdateError>,
    ) -> Result<(), UpdateError> {
        let update_file = self.dir.join("update");
        let current_dir = std::env::current_dir();

        // if the update scrip doesn't exist, return early with an error.
        if !update_file.exists() {
            return Err(UpdateError::NoUpdateScript);
        }

        set_current_dir(&self.dir).map_err(|_| UpdateError::NoUpdateScript)?;

        // call the update script as a command
        Command::new(update_file.as_os_str())
            .spawn()
            .map_err(|_| UpdateError::UpdateScriptError)?
            .wait_with_output()
            .map_err(|_| UpdateError::UpdateScriptError)?;

        if let Ok(value) = current_dir {
            set_current_dir(value).map_err(|_| UpdateError::UpdateScriptError)?;
        }

        // here we want to update the packages themselves
        for (package, data) in self
            .get_packages()
            .map_err(|_| UpdateError::UpdateScriptError)?
            .iter()
            .map(|package| {
                let data = package.is_installed();

                match data {
                    Some(value) => Some((package, value)),
                    None => None,
                }
            })
            .flatten()
        {
            let x = package.version.clone();
            let y = data.version.clone();

            let cmp = comparse_version(&x, &y);

            if let Ok(val) = cmp {
                if val < 0 {
                    continue;
                }

                update_callback(package.clone(), data.clone())?;
            }
        }

        Ok(())
    }
}

fn comparse_version(x: &str, y: &str) -> Result<i32, ParseIntError> {
    let x = x
        .chars()
        .filter(|c| c.is_digit(10))
        .collect::<String>()
        .parse::<i32>()?;

    let y = y
        .chars()
        .filter(|c| c.is_digit(10))
        .collect::<String>()
        .parse::<i32>()?;

    Ok(x - y)
}
