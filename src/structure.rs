use std::{fmt::Display, fs, path::PathBuf};

use crate::error::FileStructureError;

type FileResult<T> = Result<T, FileStructureError>;

pub trait FileStructure: Sized {
    fn create_all(&self) -> FileResult<()>;
    fn delete_all(&self) -> FileResult<()>;
    fn symlink_out_scope(&self) -> FileResult<()>;
    fn remove_symlinks(&self) -> FileResult<()>;
    fn move_all(&self, target: &PathBuf) -> FileResult<()>;
}

#[derive(Debug, Clone)]
pub struct InstallFileStructure {
    id: String,
    parent: PathBuf,
    children: Vec<String>,
}

impl InstallFileStructure {
    pub fn new(id: &str) -> Self {
        let id = id.to_owned();
        let parent = PathBuf::from(format!("/var/db/installed/{}/files", id));

        Self {
            id,
            parent,
            children: vec!["usr/bin", "usr/lib", "usr/lib64", "usr/sbin", "usr/linuxrc"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<String>>(),
        }
    }

    pub fn get_path_bufs(&self) -> Vec<PathBuf> {
        let mut bufs = Vec::<PathBuf>::new();
        let parent = &self.parent;

        for child in &self.children {
            bufs.push(parent.join(child));
        }

        bufs.push(parent.to_path_buf());
        bufs.push(parent.to_path_buf().parent().unwrap().to_path_buf());

        return bufs;
    }

    pub fn get_children(&self) -> Vec<(PathBuf, String)> {
        let mut children = Vec::<(PathBuf, String)>::new();
        let parent = &self.parent;

        for child in &self.children {
            let path = parent.join(child);
            children.push((path, child.to_owned()));
        }

        return children;
    }
}

impl FileStructure for InstallFileStructure {
    fn create_all(&self) -> FileResult<()> {
        for path in self.get_path_bufs() {
            if path.exists() {
                continue;
            }

            fs::create_dir_all(path)
                .map_err(|err| FileStructureError::FileCreateError(err.to_string()))?;
        }

        Ok(())
    }

    fn delete_all(&self) -> FileResult<()> {
        for path in self.get_path_bufs() {
            if !path.exists() {
                continue;
            }

            fs::remove_dir_all(path)
                .map_err(|err| FileStructureError::FileDeleteError(err.to_string()))?;
        }

        Ok(())
    }

    fn move_all(&self, target: &PathBuf) -> FileResult<()> {
        for (path, id) in self.get_children() {
            if !path.exists() {
                continue;
            }

            fs::copy(path, target.join(id))
                .map_err(|err| FileStructureError::FileCopyError(err.to_string()))?;
        }

        Ok(())
    }

    fn symlink_out_scope(&self) -> FileResult<()> {
        for (path, id) in self.get_children() {
            if !path.exists() {
                println!("{:?} does not exist unlink", path);
                continue;
            }

            let target_path = PathBuf::from(id).join(&self.id);

            do_recursive::<FileStructureError>(&path, &|path| {
                let child = path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .split("/")
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<String>>();

                let last = child.get(child.len() - 2);
                let mut target_path = PathBuf::from("/").join(target_path.clone());

                if let Some(last) = last {
                    target_path = target_path.join(&last);
                }

                if path.is_file() {
                    symlink(&path, &target_path)
                        .map_err(|err| FileStructureError::SymLinkError(err.to_string()))?;
                }

                Ok(())
            })
            .map_err(|error| FileStructureError::SymLinkError(error.to_string()))?;
        }

        Ok(())
    }

    fn remove_symlinks(&self) -> FileResult<()> {
        for (path, id) in self.get_children() {
            if !path.exists() {
                println!("{:?} does not exist unlink", path);
                continue;
            }

            let target_path = PathBuf::from(id).join(&self.id);

            do_recursive::<FileStructureError>(&path, &|path| {
                let child = path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .split("/")
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<String>>();

                let last = child.get(child.len() - 2);
                let mut target_path = PathBuf::from("/").join(target_path.clone());

                if let Some(last) = last {
                    target_path = target_path.join(&last);
                }

                if path.is_file() {
                    let _ = fs::remove_file(target_path);
                }

                Ok(())
            })?
        }

        Ok(())
    }
}

impl Display for FileStructureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn do_recursive<T>(
    dir: &PathBuf,
    callback: &dyn Fn(&PathBuf) -> Result<(), T>,
) -> Result<(), T> {
    for entry in dir.read_dir() {
        for entry in entry {
            let entry = entry.unwrap();
            let path = entry.path();

            match (path.is_file(), path.is_dir()) {
                (true, false) => callback(&path)?,
                (false, true) => do_recursive(dir, callback)?,
                (_, _) => {
                    println!("what? {:?}", path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(all(unix))]
fn symlink(path: &PathBuf, target: &PathBuf) -> std::io::Result<()> {
    std::os::unix::fs::symlink(&path, &target)
}

// this is just here to remove the stupid compile-time error on windows!
//#[cfg(target_os = "windows")]
//fn symlink(_: &PathBuf, _: &PathBuf) -> std::io::Result<()> {
//    Ok(())
//}
