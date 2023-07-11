use anyhow::Context;
use directories_next::ProjectDirs;
use sha2::Digest;
use sha2::Sha256;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::BufReader;
use std::io::Read;
use std::path::{Path, PathBuf};
use time::macros::format_description;
use time::OffsetDateTime;

/// Takes care of persistent storage to disk. Yes, this makes it impossible
/// to run in the browser, which only provides a String-String database. But
/// on Desktop this will improve the functionality immensely
/// if we want to run this on our server some day, it'll require some rework
/// and quite a few cfg!(), but it's worth it for the local functionality

#[derive(Debug, Clone)]
pub struct SavedDB {
    pub path: PathBuf,
    date: OffsetDateTime,
    server_str: String,
    date_str: String,
}

impl From<PathBuf> for SavedDB {
    fn from(path: PathBuf) -> Self {
        let default_name = "de99-1970-01-01-00-00-00UTC.sqlite";
        let default_filename = OsString::from(default_name);
        let filename = path
            .file_name()
            .unwrap_or(&default_filename)
            .to_str()
            .unwrap_or(default_name);
        let format = format_description!("[year]-[month]-[day]-[hour]-[minute]-[second]UTC");
        let splits: Vec<&str> = filename
            .strip_suffix(".sqlite")
            .unwrap_or(filename)
            .splitn(2, '-')
            .collect();
        let server_str = splits[0];
        let date_str = splits[1];
        let date = OffsetDateTime::parse(date_str, &format).unwrap_or(OffsetDateTime::UNIX_EPOCH); // TODO make that localized. Still save as UTC, but convert to local time after parsing

        Self {
            path: path.clone(),
            date,
            server_str: server_str.into(),
            date_str: date_str.into(),
        }
    }
}

impl Ord for SavedDB {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for SavedDB {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.path.partial_cmp(&other.path)
    }
}

impl Eq for SavedDB {}
impl PartialEq for SavedDB {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Display for SavedDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.server_str, self.date_str)
    }
}

/// returns a path to a not yet existing sqlite file. If the
/// function returns `Some(path)`, the parent directory is
/// guaranteed to exist.
pub fn get_new_db_filename(server: &str) -> Option<PathBuf> {
    if !ensure_storage_location_exists() {
        return None;
    }

    let dir = storage_dir()?;
    let format = format_description!("[year]-[month]-[day]-[hour]-[minute]-[second]UTC");
    let now = OffsetDateTime::now_utc();
    let time_str = now.format(&format).ok()?;
    let filename = format!("{server}-{time_str}.sqlite");
    Some(dir.join(filename))
}

/// get a list of all saved databases
pub fn get_list_of_saved_dbs() -> Vec<SavedDB> {
    let re = Vec::new();

    // only progress if the storage dir exists
    // there is no use calling ensure_storage_location_exists here,
    // because if we create it now it will be empty anyway
    let opt_dir = storage_dir();
    if opt_dir.is_none() {
        return re;
    }
    let dir = opt_dir.unwrap();

    // only progress if we can read the storage dir
    // TODO maybe we can tell the user what went wrong, if we can't read the directory?
    let res_files = fs::read_dir(dir);
    if res_files.is_err() {
        return re;
    }
    let files = res_files.unwrap();

    // return a list of all files that have the "sqlite" extension
    let mut db_files: Vec<SavedDB> = files
        .flatten()
        .map(|e| e.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension() == Some(OsStr::new("sqlite")))
        .map(SavedDB::from)
        .collect();
    db_files.sort();
    db_files
}

/// attempts to delete the given file
pub fn remove_db(filename: &Path) -> anyhow::Result<()> {
    fs::remove_file(filename).with_context(|| format!("Failed to delete {filename:?}"))
}

pub fn remove_all() {
    for saved_db in get_list_of_saved_dbs() {
        // TODO let the use know if something can't be deleted
        let _result = remove_db(saved_db.path.as_path());
    }
}

// read the file of the given path and return its SHA256 hash
pub fn hash(path: &Path) -> anyhow::Result<[u8; 32]> {
    // open file
    let file = File::open(path).with_context(|| format!("Failed to open {path:?}"))?;
    let mut buffered_file = BufReader::new(file);

    // read content
    let mut content: Vec<u8> = Vec::new();
    let _bytes_read = buffered_file
        .read_to_end(&mut content)
        .with_context(|| format!("Failed to read from {path:?}"))?;

    // hash content
    let hash = Sha256::digest(content);

    // rust shenanigans: return the data that is saved in a really weird format by the hashing lib
    let mut output = [0u8; 32];
    output.copy_from_slice(hash.as_slice());
    Ok(output)
}

// utility functions

fn my_project_dir() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "TurunMap")
}

fn storage_exists() -> bool {
    my_project_dir().is_some()
}

fn storage_dir() -> Option<PathBuf> {
    my_project_dir().map(|dir| dir.data_local_dir().into())
}

fn ensure_storage_location_exists() -> bool {
    if let Some(dir) = storage_dir() {
        fs::create_dir_all(dir).is_ok()
    } else {
        false
    }
}
