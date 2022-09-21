#[derive(Debug)]
pub enum FileStructureError {
    FileCreateError(String),
    FileDeleteError(String),
    SymLinkError(String),
    FileCopyError(String),
}

#[derive(Debug)]
pub enum ExecuteError {
    NoDependFound,
    CompileFail,
    UninstallFail,
}

#[derive(Debug)]
pub enum ParseError {
    NoVersion,
    Other(String),
    NoDirectory(String),
    MetadataWriting(String),
    AlreadyInstalled,
    NotInstalled,
    NoInstallScript,
    FailedInstallScript,
    NoDepends,
}

#[derive(Debug)]
pub enum BuildError {
    LinkError,
}

#[derive(Debug)]
pub enum UpdateError {
    NoUpdateScript,
    UpdateScriptError,
    PackageUpdateError(String),
}
