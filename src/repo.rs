use crate::error::ParseError;
use std::io::Write;
use std::process::Command;
use std::{
    convert::TryFrom,
    fs::{self, File},
    path::PathBuf,
};

use git2::Repository;

pub fn get_repositories() -> Vec<Repo> {
    let repositories = vec![
        PathBuf::from("/usr/repo/pur"),
        PathBuf::from("/usr/repo/pur-community"),
        PathBuf::from("/usr/repo/unofficial"),
    ];

    repositories
        .into_iter()
        .map(|buf| Repo::from(buf))
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
            .split("\n")
            .map(String::from)
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
    pub fn get_packages(&self) -> std::io::Result<Vec<Package>> {
        Ok(fs::read_dir(PathBuf::from("/var/db/installed/"))?
            .into_iter()
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .flat_map(|x| Package::try_from(x))
            .collect::<Vec<Package>>())
    }

    pub fn update_repository(&self) -> Result<(), git2::Error> {
        let repository = Repository::open(&self.dir)?;
        let remote = &mut repository.find_remote("origin")?;

        // we still have to pull (?)
        remote.fetch(&["main"], None, None)?;

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

        let install_script = self.dir.join("install");

        Command::new(install_script.as_os_str())
            .spawn()
            .map_err(|_| ParseError::NoInstallScript)?;

        Ok(())
    }
}
