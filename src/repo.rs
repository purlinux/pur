use crate::error::{ParseError, UpdateError};
use std::env::set_current_dir;
use std::io::Write;
use std::num::ParseIntError;
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

#[derive(Debug, Clone)]
pub struct InstallData {
    version: String,
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

impl TryFrom<PathBuf> for Package {
    type Error = ParseError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let dir = value;

        let name: String = dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or("".into());

        let version = fs::read_to_string(dir.join("version"))
            .map_err(|_| ParseError::NoVersion)?
            .chars()
            .filter(|x| !x.is_whitespace())
            .collect::<String>();

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

            if let Ok(val) = cmp && val < 0 {
                package.update();
            }
        }

        Ok(())
    }
}

impl Package {
    pub fn is_installed(&self) -> Option<InstallData> {
        let dir = fs::read_dir(PathBuf::from("/var/db/installed/"));

        match dir {
            Ok(value) => {
                let first = value
                    .into_iter()
                    .filter(|r| r.is_ok())
                    .map(|r| r.unwrap().path())
                    .find(|r| {
                        let lossy_str = r.as_os_str().to_string_lossy();
                        let split = lossy_str.split("/");
                        let name = split.last().or_else(|| Some("")).unwrap().to_owned();

                        name == self.name.clone()
                    });

                match first {
                    Some(value) => InstallData::try_from(value).ok(),
                    None => None,
                }
            }
            // Not sure what kind of behaviour we should expect here.
            // /var/db/installed/ is not present, while it should be.
            // We should either produce an error here, or we should make the directory.
            Err(_) => None,
        }
    }

    pub fn update(&self) {
        println!("todo: update package");
    }

    pub fn install(&self) -> Result<(), ParseError> {
        if self.is_installed().is_some() {
            return Err(ParseError::AlreadyInstalled);
        }

        let installed_dir = PathBuf::from(format!("/var/db/installed/{}", self.name));

        let files_dir = installed_dir.join("files");

        // We need these directories to move the data into.
        // These directories are required for 2 related reasons:
        // - We can't directly move the binaries into the global directories, as we still have to be able to delete the package.
        // - We have to be able to detect what package the binaries are related to
        let lib = get_dir(&files_dir, "lib");
        let lib64 = get_dir(&files_dir, "lib64");
        let bin = get_dir(&files_dir, "bin");

        // the version data
        let bytes = format!("{}", self.version).as_bytes().to_owned();

        let mut file = File::create(installed_dir.join("version")).map_err(|_| {
            ParseError::NoDirectory(format!("{}", installed_dir.as_os_str().to_string_lossy()))
        })?;

        file.write_all(&bytes)
            .map_err(|_| ParseError::MetadataWriting(String::from("Version Metadata")))?;

        // we want to change the current directory, so we can build stuff if desired.
        let dir_name = format!("/tmp/pur/{}", self.name);
        let tmp_dir = Path::new(&dir_name);

        // if the directory doesn't exist, we have to crate it
        if !tmp_dir.exists() {
            fs::create_dir_all(&dir_name).map_err(|_| ParseError::FailedInstallScript)?;
        }

        // we want to copy the temporary files into the /var/db/installed/chroot/ directory
        // these errors can be ignored, because it doesn't matter if they error.
        let _ = fs::copy(tmp_dir, &files_dir);
        let _ = fs::remove_dir(&dir_name);

        // actually change the directory
        set_current_dir(&files_dir.as_os_str()).map_err(|_| ParseError::FailedInstallScript)?;

        let install_script = self.dir.join("install");

        // We're invoking the install script as a command here.
        Command::new(install_script.as_os_str())
            .spawn()
            .map_err(|_| ParseError::NoInstallScript)?
            .wait_with_output()
            .map_err(|_| ParseError::FailedInstallScript)?;

        // make symlinks for the data within the data directories
        let _ = link_file(&lib, "/usr/lib");
        let _ = link_file(&lib64, "/usr/lib64");
        let _ = link_file(&bin, "/usr/bin");

        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), ParseError> {
        if self.is_installed().is_none() {
            return Err(ParseError::NotInstalled);
        }

        let installed_dir = PathBuf::from(format!("/var/db/installed/{}", self.name));
        let files_dir = installed_dir.join("files");

        // We need these directories to move the data into.
        // These directories are required for 2 related reasons:
        // - We can't directly move the binaries into the global directories, as we still have to be able to delete the package.
        // - We have to be able to detect what package the binaries are related to
        let lib = get_dir(&files_dir, "lib");
        let lib64 = get_dir(&files_dir, "lib64");
        let bin = get_dir(&files_dir, "bin");

        // first, we want to unlink the symlinks.
        // errors can be ignored here.
        let _ = unlink_file(&lib, "/usr/lib");
        let _ = unlink_file(&lib64, "/usr/lib64");
        let _ = unlink_file(&bin, "/usr/bin");

        // now, we want to delete the actual binary data and the full installation directory.
        fs::remove_dir_all(installed_dir).expect("Unable to delete file, are you root?");

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

fn get_dir(parent: &PathBuf, file_name: &str) -> PathBuf {
    let path = parent.join(file_name);

    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }

    path
}

fn link_file(dir: &PathBuf, target: &str) -> std::io::Result<()> {
    do_recursive(dir, &|path| {
        let file_name = path.file_name();

        if let Some(file_name) = file_name {
            let new_link = format!("{}/{}", target, file_name.to_string_lossy());
            let _ = std::os::unix::fs::symlink(path.as_os_str(), new_link);
        }
    })
}

fn unlink_file(dir: &PathBuf, target: &str) -> std::io::Result<()> {
    do_recursive(dir, &|path| {
        let file_name = path.file_name();

        if let Some(file_name) = file_name {
            let new_link = format!("{}/{}", target, file_name.to_string_lossy());
            let path = PathBuf::from(&new_link);

            if path.exists() {
                fs::remove_file(&new_link).expect("Unable to remove symlink, are you root?");
            }
        }
    })
}

fn do_recursive(dir: &PathBuf, callback: &dyn Fn(&PathBuf) -> ()) -> std::io::Result<()> {
    for entry in dir.read_dir() {
        for entry in entry {
            let entry = entry?;
            let path = entry.path();

            match (path.is_file(), path.is_dir()) {
                (true, false) => callback(dir),
                (false, true) => do_recursive(dir, callback)?,
                (_, _) => continue,
            }
        }
    }

    Ok(())
}
