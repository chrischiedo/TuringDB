use async_std::{
    fs::{DirBuilder, File, OpenOptions},
    io::{prelude::*, BufReader, ErrorKind, Seek, SeekFrom},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::RwLock,
    task,
};
use custom_codes::{DbOps, FileOps};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::Entry, HashMap},
    io::Read,
};
use tai64::TAI64N;

use crate::{
    AccessRights, AutoGeneratedIdentifier, CreateTaiTime, ModifiedTaiTime, NoOfEntries,
    RandIdentifier, RandIdentifierString, Result, Role, SeaHashCipher, TuringFeedsError,
    UserDefinedName, UserIdentifier,
};

/// No need for rights as the user who decrypts the DB has total access

#[derive(Debug)]
pub struct TuringFeeds {
    dbs: RwLock<HashMap<UserDefinedName, TuringFeedsDB>>,
    //hash: RepoBlake2hash,
    //secrecy: TuringSecrecy,
    //config: TuringConfig,
    //authstate: Assymetric Crypto
    //superuser: Only one
    // admins: vec![], -> (User, PriveledgeAccess)
    //users: vec![] -> ""
}

impl TuringFeeds {
    /// Initialize the structure with default values
    pub async fn new() -> Self {
        Self {
            dbs: RwLock::default(),
        }
    }
    /// Recursively walk through the Directory
    /// Load all the Directories into memory
    /// Hash and Compare with Persisted Hash to check for corruption
    /// Throw errors if any otherwise
    pub async fn init(&self) -> Result<&TuringFeeds> {
        let mut repo_path = PathBuf::new();

        repo_path.push("TuringFeedsRepo");
        repo_path.push("REPO");
        repo_path.set_extension("log");

        let mut contents = String::new();
        let mut file = OpenOptions::new()
            .create(false)
            .read(true)
            .write(true)
            .open(repo_path)
            .await?;

        file.read_to_string(&mut contents).await?;
        let data = ron::de::from_str::<HashMap<UserDefinedName, TuringFeedsDB>>(&contents)?;

        let mut mutate_self = self.dbs.write().await;
        *mutate_self = data;

        Ok(self)
    }
    /// Create a new repository/directory that contains the databases
    pub async fn create() -> Result<FileOps> {
        let mut repo_path = PathBuf::new();
        repo_path.push("TuringFeedsRepo");

        match DirBuilder::new().recursive(false).create(repo_path).await {
            Ok(_) => Ok(FileOps::CreateTrue),
            Err(error) => Err(TuringFeedsError::IoError(error)),
        }
    }
    /// Create the Metadata file or add data to the metadata file
    pub async fn commit(&self) -> Result<FileOps> {
        let mut repo_path = PathBuf::new();
        repo_path.push("TuringFeedsRepo");
        repo_path.push("REPO");
        repo_path.set_extension("log");

        match OpenOptions::new()
            .create(true)
            .read(false)
            .write(true)
            .open(repo_path)
            .await
        {
            Ok(mut file) => {
                let lock = self.dbs.read().await;
                let data = ron::ser::to_string(&*lock)?;
                file.write_all(&data.as_bytes().to_owned()).await?;
                file.sync_all().await?;

                Ok(FileOps::WriteTrue)
            }
            Err(error) => Err(TuringFeedsError::IoError(error)),
        }
    }
    /// Add or Modify a Database
    pub async fn memdb_add(&mut self, values: TuringFeedsDB) -> (DbOps, Option<&Self>) {
        match self.dbs.get_mut().entry(values.identifier.clone()) {
            Entry::Occupied(_) => (DbOps::AlreadyExists, None),
            Entry::Vacant(_) => {
                let mut lock = self.dbs.write().await;
                lock.insert(values.identifier.clone(), values);

                (DbOps::Inserted, Some(self))
            }
        }
    }
    /// Add or Modify a Database
    pub async fn memdb_update(&mut self, values: TuringFeedsDB) -> (DbOps, &Self) {
        match self.dbs.get_mut().entry(values.identifier.clone()) {
            Entry::Vacant(_) => (DbOps::KeyNotFound, self),
            Entry::Occupied(_) => {
                let mut lock = self.dbs.write().await;
                lock.insert(values.identifier.clone(), values);

                (DbOps::Modified, self)
            }
        }
    }
    /// Add a Database if it does not exist
    pub async fn memdb_rm(&self, key: &str) -> (DbOps, Option<TuringFeedsDB>) {
        let mut lock = self.dbs.write().await;
        match lock.remove(key) {
            Some(val) => (DbOps::Deleted, Some(val)),
            None => (DbOps::KeyNotFound, None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TuringFeedsDB {
    identifier: UserDefinedName,
    datetime: TAI64N,
    document_list: Option<HashMap<UserDefinedName, TFDocument>>,
    //rights: Option<HashMap<UserIdentifier, (Role, AccessRights)>>,
    //database_hash: Blake2hash,
    //secrecy: TuringSecrecy,
    //config: TuringConfig,
    //authstate: Assymetric Crypto
    //superuser: Only one
    // admins: vec![], -> (User, PriveledgeAccess)
    //users: vec![] -> """"
}

impl TuringFeedsDB {
    pub async fn new() -> Self {
        Self {
            identifier: String::default(),
            datetime: TAI64N::now(),
            document_list: Option::default(),
        }
    }
    pub async fn identifier(mut self, key: &str) -> Self {
        self.identifier = key.to_owned();

        self
    }
    pub async fn add(mut self, values: TFDocument) -> Self {
        if let Some(mut existing_map) = self.document_list {
            match existing_map.insert(values.identifier.clone(), values) {
                Some(_) => {
                    // If the value existed in the map
                    self.datetime = TAI64N::now();
                    self.document_list = Some(existing_map);

                    self
                }
                None => {
                    self.datetime = TAI64N::now();
                    self.document_list = Some(existing_map);

                    self
                }
            }
        } else {
            let mut new_map = HashMap::new();
            new_map.insert(values.identifier.clone(), values);
            self.datetime = TAI64N::now();
            self.document_list = Some(new_map);

            self
        }
    }
    pub async fn rm(mut self, key: &str) -> (DbOps, Self) {
        if let Some(mut existing_map) = self.document_list {
            match existing_map.remove(key) {
                Some(_) => {
                    // If the value existed in the map
                    self.datetime = TAI64N::now();
                    self.document_list = Some(existing_map);
                    (DbOps::Deleted, self)
                }
                None => {
                    // If the key does not exist in the map
                    self.document_list = Some(existing_map);
                    (DbOps::KeyNotFound, self)
                }
            }
        } else {
            // The Repository does not have any databases
            (DbOps::Empty, self)
        }
    }
}

// Get structure from file instead of making it a `pub` type
#[allow(unused_variables)]
#[derive(Debug, Serialize, Deserialize)]
enum Structure {
    Schemaless,
    Schema,
    Vector,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TFDocument {
    // Gives the document path
    identifier: RandIdentifierString,
    create_time: CreateTaiTime,
    modified_time: ModifiedTaiTime,
    //primary_key: Option<UserDefinedName>,
    //indexes: Vec<String>,
    //hash: SeaHashCipher,
    //structure: Structure,
}

impl TFDocument {
    pub async fn new() -> Self {
        let time_now = TAI64N::now();

        Self {
            identifier: RandIdentifier::build().await,
            //primary_key: Option::default(),
            //indexes: Vec::default(),
            //hash: Default::default(),
            create_time: time_now,
            modified_time: time_now,
        }
    }
    pub async fn id(mut self, value: &str) -> Self {
        self.identifier = value.to_owned();

        self
    }
    pub async fn modified_time(mut self) -> Self {
        self.modified_time = TAI64N::now();

        self
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum DocumentRights {
    /// Create Access
    C,
    /// Read Access
    R,
    /// Write Access
    W,
    /// Delete Access
    D,
    /// Forward
    F,
    /// Create Read Write Delete Access
    CRWD,
    /// Read Write Access
    RW,
}

enum TuringConfig {
    DefaultCOnfig,
    WriteACKs,
}
// Shows the level of security from the database level to a document level
enum TuringSecrecy {
    DatabaseMode,
    TableMode,
    DocumentMode,
    DefaultMode,
    InactiveMode,
}
