use git2::Repository;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Repo {
    origin: String,
    commit: String,
    dir: PathBuf,
}

impl Repo {
    pub fn parse_from(path: String) -> Result<Self, git2::Error> {
        let repository = Repository::open(path.clone())?;
        let remote = repository.find_remote("origin")?;

        Self {
            origin: remote
                .url()
                .or_else(|| Some("https://github.com/purlinux/community"))
                .unwrap()
                .to_owned(),
            commit: "".to_owned(),
            dir: PathBuf::from(path),
        };

        todo!("shutup")
    }
}
