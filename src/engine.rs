use async_std::{
    task,
    fs::{File, OpenOptions, DirBuilder},
    net::{TcpListener, TcpStream},
	io::{prelude::*, BufReader, ErrorKind},
	path::PathBuf,
};
use std::{collections::HashMap, io::Read};
use custom_codes::{FileOps, DbOps};
use tai64::TAI64N;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::{
	UserIdentifier, 
	Role, 
	AccessRights, 
	TuringFeedsError, 
	AutoGeneratedIdentifier, 
	UserDefinedName, 
	SeaHashCipher, 
	NoOfEntries, 
	CreateTaiTime, 
	ModifiedTaiTime,
	Result,
};

/// No need for rights as the user who decrypts the DB has total access

#[derive(Debug, Serialize, Deserialize)]
pub struct TuringFeeds {
	created: TAI64N,
	dbs: Option<HashMap<UserDefinedName, TuringFeedsDB>>,
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
		Self { created: TAI64N::now(), dbs: Option::default(), }
	}
	/// Recursively walk through the Directory
	/// Load all the Directories into memory
	/// Hash and Compare with Persisted Hash to check for corruption
	/// Throw errors if any otherwise 
	pub async fn init(self) -> Result<TuringFeeds> {
		let mut repo_path = PathBuf::new();
		repo_path.push("TuringFeeds");
		repo_path.push("REPO");
		repo_path.set_extension("log");

		let file = OpenOptions::new()
			.create(false)
			.read(true)
			.append(true)
			.open(repo_path).await?;

		let mut buffer = BufReader::new(file);
		let mut raw = String::new();

		buffer.read_line(&mut raw).await?;

		Ok(ron::de::from_str::<Self>(&raw)?)
	}
	/// Create a new repository/directory that contains the databases
	pub async fn create() -> Result<FileOps> {
		let mut repo_path = PathBuf::new();
		repo_path.push("TuringFeeds");
		
		match DirBuilder::new()
			.recursive(false)
			.create(repo_path)
			.await {
				Ok(_) => Ok(FileOps::CreateTrue),
				Err(error) => {
					if error.kind() == ErrorKind::PermissionDenied {
						Ok(FileOps::WriteDenied)
					}else if error.kind() == ErrorKind::AlreadyExists {
						Ok(FileOps::AlreadyExists)
					}else if error.kind() == ErrorKind::Interrupted {
						Ok(FileOps::Interrupted)
					}else {
						Err(TuringFeedsError::IoError(error))
					}
				}
			}
	}
	/// Create the Metadata file
	pub async fn metadata(self) -> Result<FileOps>{

		let mut repo_path = PathBuf::new();
		repo_path.push("TuringFeeds");
		repo_path.push("REPO");
		repo_path.set_extension("log");

		match OpenOptions::new()
		.create(true)
		.read(false)
		.append(true)
		.open(repo_path).await {
			Ok(mut file) => {
				let data = ron::ser::to_string(&self)?.as_bytes().to_owned();
				file.write_all(&data).await?;
				file.sync_all().await?;
				
				Ok(FileOps::CreateTrue)
			},
			Err(error) => {
				if error.kind() == ErrorKind::PermissionDenied {
					Ok(FileOps::WriteDenied)
				}else if error.kind() == ErrorKind::AlreadyExists {
					Ok(FileOps::AlreadyExists)
				}else if error.kind() == ErrorKind::Interrupted {
					Ok(FileOps::Interrupted)
				}else {
					Err(TuringFeedsError::IoError(error))
				} 
			}
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TuringFeedsDB {	
	identifier: UserDefinedName,
	time: TAI64N,
	document_list: Option<Vec<TFDocument>>,
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
	pub fn new() -> Self {
		Self {
			identifier: Default::default(),
			time: TAI64N::now(),
			document_list: Default::default(),
		}
	}
}

// Get structure from file instead of making it a `pub` type
#[derive(Debug, Serialize, Deserialize)]
enum Structure {
	Schemaless,
	Schema,
	Vector,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TFDocument {
	// Gives the document path
	identifier: UserDefinedName,
	primary_key: UserDefinedName,
	indexes: Vec<String>,
	hash: SeaHashCipher,
	size: NoOfEntries,
	create_time: CreateTaiTime,
	modified_time: ModifiedTaiTime,
	structure: Structure,
}

impl TFDocument {
	pub fn new() -> Self {
		Self {
			identifier: Uuid::new_v4().to_hyphenated().to_string(),
			primary_key: Default::default(),
			indexes: Vec::default(),
			hash: Default::default(),
			size: Default::default(),
			create_time: TAI64N::now(),
			modified_time: TAI64N::now(),
			structure: Structure::Schemaless,
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
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

struct TFTable {
	identifier: AutoGeneratedIdentifier,
	indexes: Vec<String>,
	primary_key: Option<String>,
	secrecy: TuringSecrecy,
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