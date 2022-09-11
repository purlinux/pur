#[derive(Debug)]
pub enum ExecuteError {
    NoDependFound,
    CompileFail,
}

#[derive(Debug)]
pub enum ParseError {
    NoVersion,
    NoDirectory,
    AlreadyInstalled,
    NoInstallScript,
    NoDepends,
}
