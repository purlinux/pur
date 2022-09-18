use crate::error::{BuildError, ParseError, UpdateError};
use std::env::set_current_dir;
use std::io::Write;
use std::num::ParseIntError;
use std::process::Command;
use std::{
    convert::TryFrom,
    fs::{self, File},
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
pub struct Package {
    pub version: String,
    pub name: String,
    pub depends: Vec<String>,
    dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InstallData {
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct InstallFlags {
    pub link: bool,
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

                        if !r
                            .read_dir()
                            .expect("Failed to read directory.")
                            .flatten()
                            .any(|f| {
                                f.file_name()
                                    .as_os_str()
                                    .to_string_lossy()
                                    .ends_with("installed")
                            })
                        {
                            return false;
                        }

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

    pub fn is_built(&self) -> Option<InstallData> {
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

    pub fn update(&self) -> Result<(), ParseError> {
        self.remove_binaries()?;
        self.build()?;
        self.install()
            .map_err(|err| ParseError::Other(format!("{:?}", err)))?;

        Ok(())
    }

    pub fn build(&self) -> Result<(), ParseError> {
        let installed_dir = PathBuf::from(format!("/var/db/installed/{}", self.name));
        let files_dir = installed_dir.join("files");

        // We need these directories to move the data into.
        // These directories are required for 2 related reasons:
        // - We can't directly move the binaries into the global directories, as we still have to be able to delete the package.
        // - We have to be able to detect what package the binaries are related to
        execute_for_dirs::<ParseError>(&files_dir, &|_, _| Ok(()))?;

        // the version data
        let bytes = format!("{}", self.version).as_bytes().to_owned();
        let version_file = installed_dir.join("version");

        if version_file.exists() {
            let _ = fs::remove_file(&version_file); // can ignore this error
        }

        // this will automatically create all the parent directories (including &installed_dir)
        fs::create_dir_all(&files_dir).expect("meow todo error");

        let mut file = File::create(&version_file).map_err(|e| {
            ParseError::NoDirectory(format!(
                "{}: {}",
                e,
                &version_file.as_os_str().to_string_lossy()
            ))
        })?;

        file.write_all(&bytes)
            .map_err(|_| ParseError::MetadataWriting(String::from("Version Metadata")))?;

        // actually change the directory
        set_current_dir(&files_dir.as_os_str())
            .map_err(|_| ParseError::NoDirectory(String::from("Unable to change to directory")))?;

        let install_script = self.dir.join("install");

        // We're invoking the install script as a command here.
        Command::new(install_script.as_os_str())
            .args([&files_dir, &self.dir])
            .spawn()
            .map_err(|_| ParseError::NoInstallScript)?
            .wait_with_output()
            .map_err(|_| ParseError::FailedInstallScript)?;

        Ok(())
    }

    pub fn install(&self) -> Result<(), BuildError> {
        let installed_dir = PathBuf::from(format!("/var/db/installed/{}", self.name));
        let files_dir = installed_dir.join("files");

        let _ = File::create(installed_dir.join("installed"));

        execute_for_dirs::<BuildError>(&files_dir, &|dir, id| {
            link_file(dir, id).map_err(|_| BuildError::LinkError)
        })
    }

    pub fn uninstall(&self) -> Result<(), ParseError> {
        if self.is_built().is_none() {
            return Err(ParseError::NotInstalled);
        }

        let installed_dir = PathBuf::from(format!("/var/db/installed/{}", self.name));

        // first, we want to remove the binaries.
        // these binaries are stored within the `installed_dir` directory,
        // so we have to delete them before we delete the directory.
        self.remove_binaries()?;

        // now, we want to delete the actual binary data and the full installation directory.
        fs::remove_dir_all(installed_dir).expect("Unable to delete file, are you root?");

        Ok(())
    }

    pub fn remove_binaries(&self) -> Result<(), ParseError> {
        let installed_dir = PathBuf::from(format!("/var/db/installed/{}", self.name));
        let files_dir = installed_dir.join("files");

        execute_for_dirs::<ParseError>(&files_dir, &|path, id| {
            unlink_file(&path, id).map_err(|_| ParseError::NoDirectory(String::from("")))
        })
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

fn link_file(dir: &PathBuf, target: &str) -> std::io::Result<()> {
    do_recursive(dir, &|path| {
        let file_name = path.file_name();

        if let Some(file_name) = file_name {
            let new_link = format!("{}/{}", target, file_name.to_string_lossy());
            std::os::unix::fs::symlink(path.as_os_str(), &new_link).expect("meow");
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
                (true, false) => callback(&path),
                (false, true) => do_recursive(dir, callback)?,
                (_, _) => {
                    println!("what? {:?}", path);
                }
            }
        }
    }

    Ok(())
}

fn execute_for_dirs<T>(
    base_dir: &PathBuf,
    callback: &dyn Fn(&PathBuf, &String) -> Result<(), T>,
) -> Result<(), T> {
    let directories = vec!["usr/bin", "usr/lib", "usr/lib64", "usr/sbin", "usr/linuxrc"];

    for directory in directories {
        let dir = base_dir.join(directory);

        if !dir.exists() {
            fs::create_dir_all(&dir).expect("Unable to create directory.");
        }

        callback(&dir, &format!("/{}", directory))?;
    }

    Ok(())
}
