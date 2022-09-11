pub mod error;
mod handle;
mod repo;

use crate::error::ExecuteError;
use crate::repo::Package;
use clap::{arg, command};

fn main() -> Result<(), ExecuteError> {
    let matches = command!()
        .arg(
            arg!(
                -i --install <packages> "Fetches & installs packages"
            )
            .required(false),
        )
        .arg(
            arg!(
                -r --remove <packages> "Removes package binaries & from local database"
            )
            .required(false),
        )
        .arg(
            arg!(
                -u --update "Updates the local repositories cached."
            )
            .required(false),
        )
        .get_matches();

    let repositories = repo::get_repositories();
    let packages = repositories
        .iter()
        .map(|repo| repo.get_packages())
        .flatten()
        .flatten()
        .collect::<Vec<Package>>();

    if let Some(to_install) = matches.get_many::<String>("install") {
        let to_install = to_install
            .into_iter()
            .map(|pkg| packages.iter().find(|x| &x.name == pkg))
            .flatten()
            .cloned()
            .collect::<Vec<Package>>();

        for package in to_install {
            handle::install(&package, &packages)?;
        }
    }

    if matches.is_present("update") {
        for repository in repositories {
            if let Err(_) = repository.update_repository() {
                panic!("Unable to synchronize repositories.")
            }
        }
    }

    Ok(())
}
