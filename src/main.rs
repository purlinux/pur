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

    // If we're here, it means the program has to do something with the repositories.
    // Therefore, we're free to fetch all repositories now.
    let repositories = repo::get_repositories();

    // We want to get all packages here, we could move this down later.
    // Currently, all commands require the packages to be fetched from the system,
    // and therefore it doesn't matter it's here.
    // We want some way to be able to detect if the command fetches packages later on,
    // because we don't want to have to refetch for every command.
    let packages = repositories
        .iter()
        .flat_map(|repo| repo.get_packages())
        .flatten()
        .collect::<Vec<Package>>();

    if let Some(to_install) = matches.get_many::<String>("install") {
        let to_install = to_install
            .into_iter()
            .flat_map(|pkg| packages.iter().find(|x| &x.name == pkg)) // find a package which matches the name given by the user.
            .cloned()
            .collect::<Vec<Package>>();

        // Install all packages.
        // We should manually handle the error thrown by handle::install() here,
        // but currently we're just panicing, so please do this in the future.
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
