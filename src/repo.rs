use crate::error::{ParseError, UpdateError};
use std::env::set_current_dir;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::{
    convert::TryFrom,
    fs::{self, File},
    path::PathBuf,
};

pub fn get_repositories() -> Vec<Repo> {
    let repo_var = match std::env::var("PUR_REPOS") {
        Ok(val) => val,
        Err(_) => "/usr/repo/pur:/usr/repo/pur-community:/usr/repo/unofficial".to_owned(), // default value, in case the environment variable is not present.
    };

    repo_var
        .split(":")
        .map(PathBuf::from)
        .map(Repo::from)
        .collect::<Vec<Repo>>()
}

#[derive(Debug, Clone)]
pub struct Package {
    pub version: String,
    pub name: String,
    pub depends: Vec<String>,
    dir: PathBuf,
}

#[derive(Debug)]
pub struct Repo {
    dir: PathBuf,
}

impl From<PathBuf> for Repo {
    fn from(path: PathBuf) -> Self {
        Self { dir: path }
    }
}

impl TryFrom<PathBuf> for Package {
    type Error = ParseError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let dir = value;

        let name: String = dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or("".into());

        let version = fs::read_to_string(dir.join("version")).map_err(|_| ParseError::NoVersion)?;
        let depends = fs::read_to_string(dir.join("depends"))
            .map_err(|_| ParseError::NoDepends)?
            .lines()
            .map(String::from)
            .filter(|x| !x.is_empty())
            .collect::<Vec<String>>();

        Ok(Self {
            version,
            dir,
            name,
            depends,
        })
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

    pub fn update_repository(&self) -> Result<(), UpdateError> {
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

        Ok(())
    }
}

impl Package {
    pub fn is_installed(&self) -> bool {
        let dir = fs::read_dir(PathBuf::from("/var/db/installed/"));

        match dir {
            Ok(value) => value
                .into_iter()
                .filter(|r| r.is_ok())
                .map(|r| r.unwrap().path())
                .any(|r| r.starts_with(self.name.clone())),
            // Not sure what kind of behaviour we should expect here.
            // /var/db/installed/ is not present, while it should be.
            // We should either produce an error here, or we should make the directory.
            Err(_) => false,
        }
    }

    pub fn install(&self) -> Result<(), ParseError> {
        if self.is_installed() {
            return Err(ParseError::AlreadyInstalled);
        }

        // this way of installing should DEFINITELY be redone.
        // this curently requires you to add the /var/db/installed/
        // directory to your PATH recursively, with something like export PATH="$PATH:$(find /var/db/installed/ -type d -printf ":%p")".
        // I'm still figuring out a good way to do this, if someone else wants to do it, be my guest.
        let installed_dir = PathBuf::from(format!("/var/db/installed/{}", self.name));
        let bin_dir = installed_dir.join("bin");

        // the version data
        let bytes = format!("{}", self.version).as_bytes().to_owned();

        if !bin_dir.exists() {
            fs::create_dir_all(bin_dir.as_os_str()).map_err(|_| ParseError::FailedInstallScript)?;
        }

        let mut file =
            File::create(installed_dir.join("version")).map_err(|_| ParseError::NoDirectory)?;

        file.write_all(&bytes)
            .map_err(|_| ParseError::NoDirectory)?;

        // we want to change the current directory, so we can build stuff if desired.
        let dir_name = format!("/tmp/pur/{}", self.name);
        let tmp_dir = Path::new(&dir_name);

        // if the directory doesn't exist, we have to crate it
        if !tmp_dir.exists() {
            fs::create_dir_all(&dir_name).map_err(|_| ParseError::FailedInstallScript)?;
        }

        // we want to copy the temporary files into the /var/db/installed/chroot/ directory
        // these errors can be ignored, because it doesn't matter if they error.
        let _ = fs::copy(tmp_dir, &bin_dir);
        let _ = fs::remove_dir(&dir_name);

        // actually change the directory.
        set_current_dir(&bin_dir.as_os_str()).map_err(|_| ParseError::FailedInstallScript)?;

        let install_script = self.dir.join("install");

        // We're invoking the install script as a command here.
        Command::new(install_script.as_os_str())
            .spawn()
            .map_err(|_| ParseError::NoInstallScript)?
            .wait_with_output()
            .map_err(|_| ParseError::FailedInstallScript)?;

        // here we want to clear the previously created temporary directory for building,
        // because considering the installation script is done; this is no longer needed.

        Ok(())
    }
}
