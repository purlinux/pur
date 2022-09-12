#[derive(Debug)]
pub enum ExecuteError {
    NoDependFound,
    CompileFail,
}

#[derive(Debug)]
pub enum ParseError {
    NoVersion,
    NoDirectory(String),
    MetadataWriting(String),
    AlreadyInstalled,
    NoInstallScript,
    FailedInstallScript,
    NoDepends,
}

#[derive(Debug)]
pub enum UpdateError {
    NoUpdateScript,
    UpdateScriptError,
}
