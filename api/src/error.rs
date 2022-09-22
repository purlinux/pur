use macros::DebugDisplay;
use std::{fmt::Display, io::ErrorKind};

#[derive(Debug, DebugDisplay)]
pub enum FileStructureError {
    FileCreateError(String),
    FileDeleteError(String),
    SymLinkError(String),
    FileCopyError(String),
    NoPermission,
    Other(String),
}

#[derive(Debug, DebugDisplay)]
pub enum ExecuteError {
    NoDependFound,
    CompileFail,
    UninstallFail,
}

#[derive(Debug, DebugDisplay)]
pub enum ParseError {
    NoVersion,
    NoDirectory(String),
    AlreadyInstalled,
    NotInstalled,
    NoInstallScript,
    FailedInstallScript,
    NoDepends,
    Other(String),
}

#[derive(Debug, DebugDisplay)]
pub enum BuildError {
    LinkError,
}

#[derive(Debug, DebugDisplay)]
pub enum UpdateError {
    NoUpdateScript,
    UpdateScriptError,
    PackageUpdateError(String),
}

impl From<BuildError> for ParseError {
    fn from(e: BuildError) -> Self {
        let val = e.to_string();

        match e {
            BuildError::LinkError => Self::Other(val),
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        let val = e.to_string();

        match e.kind() {
            ErrorKind::NotFound => Self::NoDirectory(val),
            _ => Self::Other(val),
        }
    }
}

impl From<std::io::Error> for FileStructureError {
    fn from(e: std::io::Error) -> Self {
        let val = e.to_string();

        match e.kind() {
            ErrorKind::AlreadyExists => Self::FileCreateError(val),
            ErrorKind::NotFound => Self::FileDeleteError(val),
            ErrorKind::PermissionDenied => Self::NoPermission,
            _ => Self::Other(e.to_string()),
        }
    }
}
