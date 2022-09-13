pub mod error;
mod handle;
mod repo;

use crate::error::ExecuteError;
use crate::repo::Package;
use clap::{arg, command, Command};

fn main() -> Result<(), ExecuteError> {
    let command = command!()
        .arg_required_else_help(true)
        .propagate_version(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("install")
                .about("Fetches & installs packages")
                .arg(arg!([NAME])),
        )
        .subcommand(Command::new("update").about("Updates the local repositories cached"))
        .subcommand(
            Command::new("search")
                .about("Search packages in local repositories.")
                .arg(arg!(-i --installed "List all packages that are installed").required(false))
                .arg(arg!(-n --name [NAME] "Filter packages starting with a string")),
        )
        .subcommand(
            Command::new("remove")
                .about("Removes package binaries & from local database")
                .arg(arg!([NAME])),
        );

    let matches = command.clone().get_matches();

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

    match matches.subcommand() {
        Some(("install", matches)) => {
            if let Some(to_install) = matches.get_many::<String>("NAME") {
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
        }
        Some(("update", _)) => {
            for repository in repositories {
                // here we're ignoring the error result of the update_repository() method.
                // not every repository has to be updated, and therefore we can simply ignore
                // errors within this method.
                let _ = repository.update_repository();
            }
        }
        Some(("remove", matches)) => {
            if let Some(to_remove) = matches.get_many::<String>("NAME") {
                let to_remove = to_remove
                    .into_iter()
                    .flat_map(|pkg| packages.iter().find(|x| &x.name == pkg)) // find a package which matches the name given by the user.
                    .cloned()
                    .collect::<Vec<Package>>();

                // Install all packages.
                // We should manually handle the error thrown by handle::install() here,
                // but currently we're just panicing, so please do this in the future.
                for package in to_remove {
                    handle::remove(&package)?;
                }
            }
        }
        Some(("search", matches)) => {
            let packages = packages
                .iter()
                .filter(|package| {
                    if matches.is_present("installed") && package.is_installed().is_none() {
                        return false;
                    }

                    if let Some(value) = matches.get_one::<String>("name") && !package.name.starts_with(value) {
                        return false;
                    }

                    return true;
            }).collect::<Vec<&Package>>();

            for package in packages {
                let mut str = format!("{} v{}", package.name, package.version);

                if package.is_installed().is_some() {
                    str += " [installed]";
                }

                println!("{}", str)
            }
        }
        _ => unreachable!("Exhausted list of sub commands"),
    };

    Ok(())
}
