use directories::{BaseDirs, ProjectDirs, UserDirs};
use std::{fs, io::Error, path::Path};

pub struct File {}

enum OpenMode {
    Read,
    Write,
    ReadWrite,
}

enum Directory {
    User(User),
}

enum User {
    Pictures(Vec<&str>),
    Downloads(Vec<&str>),
}

impl File {
    fn open(location: Directory, permissions: OpenMode) -> Result<fs::File, Box<dyn Error>> {
        let path: Option<Path> = match location {
            Directory::User(sub_dir) => match sub_dir {
                User::Pictures(join_subdirs) => {
                    let p = Path::new(&UserDirs::new()?.picture_dir()?);
                    join_subdirs.iter().map(|&subdir| p.join(subdir));
                    Some(p)
                }
                User::Downloads(join_subdirs) => {
                    let p = Path::new(&UserDirs::new()?.download_dir()?);
                    join_subdirs.iter().map(|&subdir| p.join(subdir));
                    Some(p)
                }
            },
        };
        if let Some(path) = path {
        } else {
            Err(Error::new(std::io::ErrorKind::NotFound, None))
        }
    }
}
