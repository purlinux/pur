#[derive(Debug)]
pub enum ExecuteError {
    NoDependFound,
    CompileFail,
    UninstallFail,
}

#[derive(Debug)]
pub enum ParseError {
    NoVersion,
    NoDirectory(String),
    MetadataWriting(String),
    AlreadyInstalled,
    NotInstalled,
    NoInstallScript,
    FailedInstallScript,
    NoDepends,
}

#[derive(Debug)]
pub enum UpdateError {
    NoUpdateScript,
    UpdateScriptError,
}
