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

    /// This method will fetch the external repository from the VCS.
    // pub fn update_repository(&self) -> Result<(), git2::Error> {
    //    let repository = Repository::open(&self.dir)?;
    //    let remote = &mut repository.find_remote("origin")?;
    //
    //    let branch = remote.default_branch()?;
    //
    //    let branch_name = match branch.as_str() {
    //        Some(v) => v,
    //        None => "main",
    //    };
    //
    //    remote.fetch(&[branch_name], None, None)?;
    //
    //    Ok(())
    //}

    pub fn update_repository(&self) -> Result<(), UpdateError> {
        let update_file = self.dir.join("update");

        // if the update scrip doesn't exist, return early with an error.
        if !update_file.exists() {
            return Err(UpdateError::NoUpdateScript);
        }

        // call the update script as a command
        Command::new(update_file.as_os_str())
            .spawn()
            .map_err(|_| UpdateError::UpdateScriptError)?
            .wait_with_output()
            .map_err(|_| UpdateError::UpdateScriptError)?;

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

        let installed_dir = PathBuf::from("/var/db/installed/");
        let bytes = format!("{}", self.version).as_bytes().to_owned();

        let mut file =
            File::create(installed_dir.join(&self.name)).map_err(|_| ParseError::NoDirectory)?;

        file.write_all(&bytes)
            .map_err(|_| ParseError::NoDirectory)?;

        // we want to change the current directory, so we can build stuff if desired.
        let dir_name = format!("/tmp/pur/{}", self.name);
        let tmp_dir = Path::new(&dir_name);

        // if the directory doesn't exist, we have to crate it
        if !tmp_dir.exists() {
            fs::create_dir_all(&dir_name).map_err(|_| ParseError::FailedInstallScript)?;
        }

        // actually change the directory.
        set_current_dir(&dir_name).map_err(|_| ParseError::FailedInstallScript)?;

        let install_script = self.dir.join("install");

        // We're invoking the install script as a command here.
        Command::new(install_script.as_os_str())
            .spawn()
            .map_err(|_| ParseError::NoInstallScript)?
            .wait_with_output()
            .map_err(|_| ParseError::FailedInstallScript)?;

        // here we want to clear the previously created temporary directory for building,
        // because considering the installation script is done; this is no longer needed.
        fs::remove_dir(&dir_name).map_err(|_| ParseError::FailedInstallScript)?;

        Ok(())
    }
}
