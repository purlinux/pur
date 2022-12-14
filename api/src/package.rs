use std::{
    env::set_current_dir,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::Command,
};

use crate::{
    error::{BuildError, ParseError},
    repo::InstallData,
    structure::{FileStructure, InstallFileStructure},
};

#[derive(Debug, Clone)]
pub struct Package {
    pub version: String,
    pub name: String,
    pub depends: Vec<String>,
    structure: InstallFileStructure,
    dir: PathBuf,
}

impl Package {
    pub fn is_installed(&self) -> Option<InstallData> {
        let path = PathBuf::from("/var/db/installed/");

        if !path.exists() {
            fs::create_dir_all(&path).expect("Couldn't create /var/db/installed/");
        }

        let dir = fs::read_dir(&path);

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
            Err(_) => None,
        }
    }

    // This method is exactly the same as [is_installed()], however
    // this skips the check for the `installed` file within the directory.
    //
    // For this reason, this method is also more efficient than the before-mentioned
    // alternative method.
    //
    // This is because our current file structure allows you to have non-installed but built
    // packages within the /var/db/installed/ directory. These should probably be re-categorized
    // into something like /var/db/built/, and after installation moved into /var/db/installed. But
    // for now, our structure is like this.
    pub fn is_built(&self) -> Option<InstallData> {
        let path = PathBuf::from("/var/db/installed/");

        if !path.exists() {
            fs::create_dir_all(&path).expect("Couldn't create /var/db/installed/");
        }

        let dir = fs::read_dir(&path);

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
            Err(_) => None,
        }
    }

    pub fn update(&self) -> Result<(), ParseError> {
        self.remove_binaries()?;
        self.build()?;
        self.install()?;

        Ok(())
    }

    pub fn build(&self) -> Result<(), ParseError> {
        let installed_dir = PathBuf::from(format!("/var/db/installed/{}", self.name));
        let files_dir = installed_dir.join("files");

        self.structure
            .create_all()
            .map_err(|e| ParseError::Other(e.to_string()))?;

        // the version data
        let bytes = format!("{}", self.version).as_bytes().to_owned();
        let version_file = installed_dir.join("version");

        if version_file.exists() {
            let _ = fs::remove_file(&version_file); // can ignore this error
        }

        let mut file = File::create(&version_file)?;
        file.write_all(&bytes)?;

        // actually change the directory
        set_current_dir(&files_dir.as_os_str())?;

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
        let _ = File::create(installed_dir.join("installed"));

        self.structure
            .symlink_out_scope()
            .map_err(|_| BuildError::LinkError)
    }

    pub fn uninstall(&self) -> Result<(), ParseError> {
        if self.is_built().is_none() {
            return Err(ParseError::NotInstalled);
        }

        // first, we want to remove the binaries.
        // these binaries are stored within the `installed_dir` directory,
        // so we have to delete them before we delete the directory.
        self.remove_binaries()?;

        // now, we want to remove the actual storage of the binaries and the installation data.
        self.structure
            .delete_all()
            .map_err(|e| ParseError::Other(e.to_string()))?;

        Ok(())
    }

    pub fn remove_binaries(&self) -> Result<(), ParseError> {
        self.structure
            .remove_symlinks()
            .map_err(|e| ParseError::NoDirectory(e.to_string()))?;

        Ok(())
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

        let structure = InstallFileStructure::new(&name);

        Ok(Self {
            version,
            dir,
            name,
            depends,
            structure,
        })
    }
}
