use clap::ArgMatches;

use crate::error::{ExecuteError, UpdateError};
use crate::repo::{InstallFlags, Package, Repo};

pub fn install(
    package: &Package,
    packages: &Vec<Package>,
    matches: &ArgMatches,
) -> Result<(), ExecuteError> {
    for ele in &package.depends {
        let depend = packages.iter().find(|package| &package.name == ele);

        match depend {
            // We just want to call this method recursively until all dependencies are installed.
            // We probably want to manually handle the error in here, considering they're children, and not the entire
            // build process should have to be stopped just because this build fails.
            Some(package) => install(&package, &packages, matches)?,
            // I'm not sure what kind of behaviour we should be expecting here.
            // Should we expect the whole package to be skipped? Or should we just ignore this dependency?
            // I suggest we completely skip the package for now, because there is simply something wrong with the package if
            // the dependency is not present, and if it actually does depend on the package, there's something wrong with
            // the user's repositories setup on their local system.
            None => return Err(ExecuteError::NoDependFound),
        }
    }

    let flags: InstallFlags = matches.into();

    match package.install(flags) {
        Ok(_) => println!("Installed {} v{}", package.name, package.version),
        Err(e) => {
            println!(
                "Failed to install {} v{}... Skipping!",
                package.name, package.version
            );

            // Here we want to print the error for easier debugging.
            // Should we only print this if a certain environment variable is set? (e.g DEBUG).
            println!("{:?}", e);

            return Err(ExecuteError::CompileFail);
        }
    };

    Ok(())
}

pub fn update(repository: &Repo) -> Result<(), UpdateError> {
    repository.update_repository(&mut |package, data| {
        println!(
            "Found new version {} for {}! Updating...Updating from {}...",
            package.version, package.name, data.version
        );

        // we want to update the package contents now
        match package.update() {
            Ok(_) => {
                println!("Updated {} to v{}", package.name, package.version);
            }
            Err(e) => {
                println!(
                    "Failed to update {} to v{}, because {:?}",
                    package.name, package.version, e
                );

                println!("... Skipping!");
            }
        };

        Ok(())
    })
}

pub fn remove(package: &Package) -> Result<(), ExecuteError> {
    match package.uninstall() {
        Ok(_) => println!("Removed {} v{}", package.name, package.version),
        Err(e) => {
            println!(
                "Failed to remove {} v{}... Skipping!",
                package.name, package.version
            );

            // Here we want to print the error for easier debugging.
            // Should we only print this if a certain environment variable is set? (e.g DEBUG).
            println!("{:?}", e);

            return Err(ExecuteError::UninstallFail);
        }
    }

    Ok(())
}
