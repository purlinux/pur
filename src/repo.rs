use git2::{Error, Repository};
use std::env;
use std::{
    convert::{TryFrom, TryInto},
    fs::{self, ReadDir},
    path::PathBuf,
};

pub enum ParseError {
    NoVersion,
    NoDirectory,
}

#[derive(Debug)]
pub struct Package {
    version: String,
    name: String,
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

        Ok(Self { version, dir, name })
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
}

impl Package {
    fn is_installed(&self) -> bool {
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
}
