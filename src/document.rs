use directories;
use open;
use std::error::Error;
use std::fmt::Display;
use std::fs::{create_dir_all, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Document {
    pathbuf: PathBuf,
    file: Option<std::fs::File>,
}

#[derive(Debug, Clone, Copy)]
pub enum OpenMode {
    Read,
    Replace,
    Append,
    ReadReplace,
    ReadAppend,
}

impl OpenMode {
    pub fn readable(&self) -> bool {
        match self {
            Self::Read | Self::ReadReplace | Self::ReadAppend => true,
            _ => false,
        }
    }
    pub fn writable(&self) -> bool {
        match self {
            Self::Replace | Self::Append | Self::ReadAppend | Self::ReadReplace => true,
            _ => false,
        }
    }
    pub fn appendable(&self) -> bool {
        match self {
            Self::Append | Self::ReadAppend => true,
            _ => false,
        }
    }
}

pub enum Folder<'a> {
    User(User<'a>),
}

fn join_all(path: &Path, join_vec: Vec<&str>) -> PathBuf {
    let mut pathbuf = path.to_path_buf();
    for joinable in join_vec {
        pathbuf.push(joinable);
    }
    pathbuf
}

impl<'a> Folder<'a> {
    fn into_pathbuf_result(&self, filename: &str) -> Result<PathBuf, DocumentError> {
        match self {
            Folder::User(subdir) => match subdir {
                User::Pictures(join_vec) => {
                    if let Some(dir) = directories::UserDirs::new() {
                        if let Some(path) = dir.picture_dir() {
                            let mut pathbuf = join_all(path, join_vec.clone());
                            pathbuf = pathbuf.join(filename);
                            Ok(pathbuf)
                        } else {
                            Err(DocumentError::PicturesDirNotFound)?
                        }
                    } else {
                        Err(DocumentError::UserDirsNotFound)?
                    }
                }
                User::Downloads(join_vec) => {
                    if let Some(dir) = directories::UserDirs::new() {
                        if let Some(path) = dir.download_dir() {
                            let mut pathbuf = join_all(path, join_vec.clone());
                            pathbuf = pathbuf.join(filename);
                            Ok(pathbuf)
                        } else {
                            Err(DocumentError::DownloadsDirNotFound)?
                        }
                    } else {
                        Err(DocumentError::UserDirsNotFound)?
                    }
                }
            },
        }
    }
}

pub enum User<'a> {
    Pictures(Vec<&'a str>),
    Downloads(Vec<&'a str>),
}
#[derive(Debug)]
pub enum DocumentError {
    UserDirsNotFound,
    PicturesDirNotFound,
    DownloadsDirNotFound,
    FileNotFound,
    CouldNotCreateFile,
    CouldNotCreateParentFolder,
    CouldNotLaunchFile,
}

impl Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(match self {
            Self::UserDirsNotFound => "UserDirsNotFound",
            Self::PicturesDirNotFound => "PicturesDirNotFound",
            Self::DownloadsDirNotFound => "DownloadsDirNotFound",
            Self::FileNotFound => "FileNotFound",
            Self::CouldNotCreateFile => "CouldNotCreateFile",
            Self::CouldNotCreateParentFolder => "CouldNotFindParentFolder",
            Self::CouldNotLaunchFile => "CouldNotLaunchFile",
        })
    }
}

impl Error for DocumentError {
    fn description(&self) -> &str {
        match self {
            Self::UserDirsNotFound => "Could not find user directory",
            Self::PicturesDirNotFound => "Could not find pictures directory",
            Self::DownloadsDirNotFound => "Could not find downloads directory",
            Self::FileNotFound => "Could not find requested file",
            Self::CouldNotCreateFile => "Could not create file",
            Self::CouldNotCreateParentFolder => "Could not find parent folder of this file",
            Self::CouldNotLaunchFile => {
                "Unable to use https://crates.io/crates/open to launch file"
            }
        }
    }
}

impl Document {
    pub fn new(location: Folder, filename: &str) -> Result<Self, Box<dyn Error>> {
        let pathbuf = location.into_pathbuf_result(filename)?;
        Ok(Self {
            pathbuf,
            file: None,
        })
    }
    pub fn open(&mut self, permissions: OpenMode) -> Result<&mut Self, Box<dyn Error>> {
        if let Ok(file) = OpenOptions::new()
            .read(permissions.readable())
            .write(permissions.writable())
            .append(permissions.appendable())
            .open(self.pathbuf.clone())
        {
            self.file = Some(file);
            Ok(self)
        } else {
            Err(Box::new(DocumentError::FileNotFound))
        }
    }
    pub fn create_and_open(&mut self, permissions: OpenMode) -> Result<&mut Self, Box<dyn Error>> {
        if let Some(parent_folder) = self.pathbuf.clone().parent() {
            if let Err(_) = create_dir_all(parent_folder) {
                Err(DocumentError::CouldNotCreateParentFolder)?
            }
        }
        let mut suffix = 0;
        let filename = self
            .pathbuf
            .clone()
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string()
            .split(".")
            .collect::<Vec<&str>>()[0]
            .to_string();
        let extension = self
            .pathbuf
            .clone()
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string();
        while self.pathbuf.exists() {
            suffix += 1;
            let new_filename =
                filename.clone() + suffix.to_string().as_str() + "." + extension.as_str();
            self.pathbuf = self
                .pathbuf
                .clone()
                .parent()
                .unwrap_or(&Path::new(""))
                .join(new_filename);
        }
        if let Ok(file) = OpenOptions::new()
            .read(permissions.readable())
            .write(permissions.writable())
            .append(permissions.appendable())
            .create_new(true)
            .open(self.pathbuf.clone())
        {
            self.file = Some(file);
            Ok(self)
        } else {
            Err(Box::new(DocumentError::CouldNotCreateFile))
        }
    }
    pub fn launch_with_default_app(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(_) = self.file {
            self.file = None;
        }
        if let Err(_) = open::that_detached(self.path()) {
            Err(Box::new(DocumentError::CouldNotLaunchFile))
        } else {
            Ok(())
        }
    }
    pub fn name(&self) -> String {
        self.pathbuf
            .clone()
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("")
            .to_string()
    }
    pub fn path(&self) -> String {
        self.pathbuf.as_os_str().to_str().unwrap_or("").to_string()
    }
    pub fn file(&self) -> Option<&std::fs::File> {
        self.file.as_ref()
    }
}

pub fn with<Closure>(
    file: Result<Document, Box<dyn Error>>,
    closure: Closure,
) -> Result<(), Box<dyn Error>>
where
    Closure: Fn(Document) -> Result<(), Box<dyn Error>>,
{
    closure(file?)
}
