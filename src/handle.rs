use crate::error::ExecuteError;
use crate::repo::Package;

pub fn install(package: &Package, packages: &Vec<Package>) -> Result<(), ExecuteError> {
    for ele in &package.depends {
        let depend = packages.iter().find(|package| &package.name == ele);

        match depend {
            Some(package) => install(&package, &packages)?,
            None => return Err(ExecuteError::NoDependFound),
        }
    }

    match package.install() {
        Ok(_) => println!("Installed {} v{}", package.name, package.version),
        Err(_) => {
            println!(
                "Failed to install {} v{}... Skipping!",
                package.name, package.version
            );
            return Err(ExecuteError::CompileFail);
        }
    };

    Ok(())
}
