//! # libsql-js
//!
//! A wrapper around the libSQL library for use in Node, Bun, and Deno.
//!
//! ## Design
//!
//! This JavaScript API is designed to be a drop-in replacement for `better-sqlite3`
//! with an opt-in async variant of the API.
//!
//! The API has two main classes: `Database` and `Statement`. The `Database` class
//! is a wrapper around libSQL `Database` and `Connection` structs whereas the
//! `Statement` class is a wrapper around libSQL `Statement` struct.
//!
//! As the `libsql` crate is async, the core of `libsql-js` is also implemented as such.
//! To support the synchronous semantics of `better-sqlite3`, the native API exposes
//! functions that are synchronous and block the event loop using Tokio's runtime. However,
//! the `promise` API module returns promises using `napi-rs` `Env::execute_tokio_future`.

#![deny(clippy::all)]
#![allow(non_snake_case)]
#![allow(deprecated)]

mod auth;

use napi::{
    bindgen_prelude::{Array, FromNapiValue, ToNapiValue},
    Env, JsUnknown, Result, ValueType,
};
use napi_derive::napi;
use once_cell::sync::OnceCell;
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::runtime::Runtime;
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

struct Error(libsql::Error);

impl From<Error> for napi::Error {
    fn from(error: Error) -> Self {
        use libsql::Error as E;
        match &error.0 {
            E::SqliteFailure(raw_code, msg) => {
                let code = map_sqlite_code(*raw_code);
                if *raw_code == libsql::ffi::SQLITE_AUTH {
                    let err_json = serde_json::json!({
                        "message": "Authorization denied by JS authorizer",
                        "libsqlError": true,
                        "code": code,
                        "rawCode": *raw_code
                    });
                    napi::Error::from_reason(err_json.to_string())
                } else {
                    throw_sqlite_error(msg.clone(), code, *raw_code)
                }
            }
            other => {
                let err_json = serde_json::json!({
                    "message": other.to_string(),
                    "libsqlError": true,
                    "code": "SQLITE_ERROR",
                    "rawCode": 1
                });
                napi::Error::from_reason(err_json.to_string())
            }
        }
    }
}

fn map_sqlite_code(code: i32) -> String {
    match code {
        libsql::ffi::SQLITE_OK => "SQLITE_OK".to_owned(),
        libsql::ffi::SQLITE_ERROR => "SQLITE_ERROR".to_owned(),
        libsql::ffi::SQLITE_INTERNAL => "SQLITE_INTERNAL".to_owned(),
        libsql::ffi::SQLITE_PERM => "SQLITE_PERM".to_owned(),
        libsql::ffi::SQLITE_ABORT => "SQLITE_ABORT".to_owned(),
        libsql::ffi::SQLITE_BUSY => "SQLITE_BUSY".to_owned(),
        libsql::ffi::SQLITE_LOCKED => "SQLITE_LOCKED".to_owned(),
        libsql::ffi::SQLITE_NOMEM => "SQLITE_NOMEM".to_owned(),
        libsql::ffi::SQLITE_READONLY => "SQLITE_READONLY".to_owned(),
        libsql::ffi::SQLITE_INTERRUPT => "SQLITE_INTERRUPT".to_owned(),
        libsql::ffi::SQLITE_IOERR => "SQLITE_IOERR".to_owned(),
        libsql::ffi::SQLITE_CORRUPT => "SQLITE_CORRUPT".to_owned(),
        libsql::ffi::SQLITE_NOTFOUND => "SQLITE_NOTFOUND".to_owned(),
        libsql::ffi::SQLITE_FULL => "SQLITE_FULL".to_owned(),
        libsql::ffi::SQLITE_CANTOPEN => "SQLITE_CANTOPEN".to_owned(),
        libsql::ffi::SQLITE_PROTOCOL => "SQLITE_PROTOCOL".to_owned(),
        libsql::ffi::SQLITE_EMPTY => "SQLITE_EMPTY".to_owned(),
        libsql::ffi::SQLITE_SCHEMA => "SQLITE_SCHEMA".to_owned(),
        libsql::ffi::SQLITE_TOOBIG => "SQLITE_TOOBIG".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT => "SQLITE_CONSTRAINT".to_owned(),
        libsql::ffi::SQLITE_MISMATCH => "SQLITE_MISMATCH".to_owned(),
        libsql::ffi::SQLITE_MISUSE => "SQLITE_MISUSE".to_owned(),
        libsql::ffi::SQLITE_NOLFS => "SQLITE_NOLFS".to_owned(),
        libsql::ffi::SQLITE_AUTH => "SQLITE_AUTH".to_owned(),
        libsql::ffi::SQLITE_FORMAT => "SQLITE_FORMAT".to_owned(),
        libsql::ffi::SQLITE_RANGE => "SQLITE_RANGE".to_owned(),
        libsql::ffi::SQLITE_NOTADB => "SQLITE_NOTADB".to_owned(),
        libsql::ffi::SQLITE_NOTICE => "SQLITE_NOTICE".to_owned(),
        libsql::ffi::SQLITE_WARNING => "SQLITE_WARNING".to_owned(),
        libsql::ffi::SQLITE_ROW => "SQLITE_ROW".to_owned(),
        libsql::ffi::SQLITE_DONE => "SQLITE_DONE".to_owned(),
        libsql::ffi::SQLITE_IOERR_READ => "SQLITE_IOERR_READ".to_owned(),
        libsql::ffi::SQLITE_IOERR_SHORT_READ => "SQLITE_IOERR_SHORT_READ".to_owned(),
        libsql::ffi::SQLITE_IOERR_WRITE => "SQLITE_IOERR_WRITE".to_owned(),
        libsql::ffi::SQLITE_IOERR_FSYNC => "SQLITE_IOERR_FSYNC".to_owned(),
        libsql::ffi::SQLITE_IOERR_DIR_FSYNC => "SQLITE_IOERR_DIR_FSYNC".to_owned(),
        libsql::ffi::SQLITE_IOERR_TRUNCATE => "SQLITE_IOERR_TRUNCATE".to_owned(),
        libsql::ffi::SQLITE_IOERR_FSTAT => "SQLITE_IOERR_FSTAT".to_owned(),
        libsql::ffi::SQLITE_IOERR_UNLOCK => "SQLITE_IOERR_UNLOCK".to_owned(),
        libsql::ffi::SQLITE_IOERR_RDLOCK => "SQLITE_IOERR_RDLOCK".to_owned(),
        libsql::ffi::SQLITE_IOERR_DELETE => "SQLITE_IOERR_DELETE".to_owned(),
        libsql::ffi::SQLITE_IOERR_BLOCKED => "SQLITE_IOERR_BLOCKED".to_owned(),
        libsql::ffi::SQLITE_IOERR_NOMEM => "SQLITE_IOERR_NOMEM".to_owned(),
        libsql::ffi::SQLITE_IOERR_ACCESS => "SQLITE_IOERR_ACCESS".to_owned(),
        libsql::ffi::SQLITE_IOERR_CHECKRESERVEDLOCK => "SQLITE_IOERR_CHECKRESERVEDLOCK".to_owned(),
        libsql::ffi::SQLITE_IOERR_LOCK => "SQLITE_IOERR_LOCK".to_owned(),
        libsql::ffi::SQLITE_IOERR_CLOSE => "SQLITE_IOERR_CLOSE".to_owned(),
        libsql::ffi::SQLITE_IOERR_DIR_CLOSE => "SQLITE_IOERR_DIR_CLOSE".to_owned(),
        libsql::ffi::SQLITE_IOERR_SHMOPEN => "SQLITE_IOERR_SHMOPEN".to_owned(),
        libsql::ffi::SQLITE_IOERR_SHMSIZE => "SQLITE_IOERR_SHMSIZE".to_owned(),
        libsql::ffi::SQLITE_IOERR_SHMLOCK => "SQLITE_IOERR_SHMLOCK".to_owned(),
        libsql::ffi::SQLITE_IOERR_SHMMAP => "SQLITE_IOERR_SHMMAP".to_owned(),
        libsql::ffi::SQLITE_IOERR_SEEK => "SQLITE_IOERR_SEEK".to_owned(),
        libsql::ffi::SQLITE_IOERR_DELETE_NOENT => "SQLITE_IOERR_DELETE_NOENT".to_owned(),
        libsql::ffi::SQLITE_IOERR_MMAP => "SQLITE_IOERR_MMAP".to_owned(),
        libsql::ffi::SQLITE_IOERR_GETTEMPPATH => "SQLITE_IOERR_GETTEMPPATH".to_owned(),
        libsql::ffi::SQLITE_IOERR_CONVPATH => "SQLITE_IOERR_CONVPATH".to_owned(),
        libsql::ffi::SQLITE_IOERR_VNODE => "SQLITE_IOERR_VNODE".to_owned(),
        libsql::ffi::SQLITE_IOERR_AUTH => "SQLITE_IOERR_AUTH".to_owned(),
        libsql::ffi::SQLITE_LOCKED_SHAREDCACHE => "SQLITE_LOCKED_SHAREDCACHE".to_owned(),
        libsql::ffi::SQLITE_BUSY_RECOVERY => "SQLITE_BUSY_RECOVERY".to_owned(),
        libsql::ffi::SQLITE_BUSY_SNAPSHOT => "SQLITE_BUSY_SNAPSHOT".to_owned(),
        libsql::ffi::SQLITE_CANTOPEN_NOTEMPDIR => "SQLITE_CANTOPEN_NOTEMPDIR".to_owned(),
        libsql::ffi::SQLITE_CANTOPEN_ISDIR => "SQLITE_CANTOPEN_ISDIR".to_owned(),
        libsql::ffi::SQLITE_CANTOPEN_FULLPATH => "SQLITE_CANTOPEN_FULLPATH".to_owned(),
        libsql::ffi::SQLITE_CANTOPEN_CONVPATH => "SQLITE_CANTOPEN_CONVPATH".to_owned(),
        libsql::ffi::SQLITE_CORRUPT_VTAB => "SQLITE_CORRUPT_VTAB".to_owned(),
        libsql::ffi::SQLITE_READONLY_RECOVERY => "SQLITE_READONLY_RECOVERY".to_owned(),
        libsql::ffi::SQLITE_READONLY_CANTLOCK => "SQLITE_READONLY_CANTLOCK".to_owned(),
        libsql::ffi::SQLITE_READONLY_ROLLBACK => "SQLITE_READONLY_ROLLBACK".to_owned(),
        libsql::ffi::SQLITE_READONLY_DBMOVED => "SQLITE_READONLY_DBMOVED".to_owned(),
        libsql::ffi::SQLITE_ABORT_ROLLBACK => "SQLITE_ABORT_ROLLBACK".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_CHECK => "SQLITE_CONSTRAINT_CHECK".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_COMMITHOOK => "SQLITE_CONSTRAINT_COMMITHOOK".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_FOREIGNKEY => "SQLITE_CONSTRAINT_FOREIGNKEY".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_FUNCTION => "SQLITE_CONSTRAINT_FUNCTION".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_NOTNULL => "SQLITE_CONSTRAINT_NOTNULL".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_PRIMARYKEY => "SQLITE_CONSTRAINT_PRIMARYKEY".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_TRIGGER => "SQLITE_CONSTRAINT_TRIGGER".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_UNIQUE => "SQLITE_CONSTRAINT_UNIQUE".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_VTAB => "SQLITE_CONSTRAINT_VTAB".to_owned(),
        libsql::ffi::SQLITE_CONSTRAINT_ROWID => "SQLITE_CONSTRAINT_ROWID".to_owned(),
        libsql::ffi::SQLITE_NOTICE_RECOVER_WAL => "SQLITE_NOTICE_RECOVER_WAL".to_owned(),
        libsql::ffi::SQLITE_NOTICE_RECOVER_ROLLBACK => "SQLITE_NOTICE_RECOVER_ROLLBACK".to_owned(),
        libsql::ffi::SQLITE_WARNING_AUTOINDEX => "SQLITE_WARNING_AUTOINDEX".to_owned(),
        libsql::ffi::SQLITE_AUTH_USER => "SQLITE_AUTH_USER".to_owned(),
        libsql::ffi::SQLITE_OK_LOAD_PERMANENTLY => "SQLITE_OK_LOAD_PERMANENTLY".to_owned(),
        _ => format!("UNKNOWN_SQLITE_ERROR_{}", code),
    }
}

pub fn throw_sqlite_error(message: String, code: String, raw_code: i32) -> napi::Error {
    let err_json = serde_json::json!({
        "message": message,
        "libsqlError": true,
        "code": code,
        "rawCode": raw_code
    });
    napi::Error::from_reason(err_json.to_string())
}

impl From<libsql::Error> for Error {
    fn from(error: libsql::Error) -> Self {
        Error(error)
    }
}

/// SQLite connection options.
#[napi(object)]
pub struct Options {
    // Timeout in seconds.
    pub timeout: Option<f64>,
    // Authentication token for remote databases.
    pub authToken: Option<String>,
    // URL for remote database sync.
    pub syncUrl: Option<String>,
    // Read your writes.
    pub readYourWrites: Option<bool>,
    // Sync interval in seconds.
    pub syncPeriod: Option<f64>,
    // Encryption cipher for local enryption at rest.
    pub encryptionCipher: Option<String>,
    // Encryption key for local encryption at rest.
    pub encryptionKey: Option<String>,
    // Encryption key for remote encryption at rest.
    pub remoteEncryptionKey: Option<String>,
}

/// Access mode.
///
/// The `better-sqlite3` API allows the caller to configure the format of
/// query results. This struct encapsulates the different access mode configs.
struct AccessMode {
    pub(crate) raw: AtomicBool,
    pub(crate) pluck: AtomicBool,
    pub(crate) safe_ints: AtomicBool,
    pub(crate) timing: AtomicBool,
}

/// SQLite database connection.
#[napi]
pub struct Database {
    // The libSQL database instance.
    db: libsql::Database,
    // The libSQL connection instance.
    conn: Option<Arc<libsql::Connection>>,
    // Whether to use safe integers by default.
    default_safe_integers: AtomicBool,
    // Whether to use memory-only mode.
    memory: bool,
}

impl Drop for Database {
    fn drop(&mut self) {
        self.conn = None;
    }
}

#[napi]
impl Database {
    /// Creates a new database instance.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the database file.
    /// * `opts` - The database options.
    #[napi(constructor)]
    pub fn new(path: String, opts: Option<Options>) -> Result<Self> {
        ensure_logger();
        let rt = runtime()?;
        let remote = is_remote_path(&path);
        let db = if remote {
            let auth_token = opts
                .as_ref()
                .and_then(|o| o.authToken.as_ref())
                .cloned()
                .unwrap_or_default();
            let mut builder = libsql::Builder::new_remote(path.clone(), auth_token);
            if let Some(encryption_key) = opts
                .as_ref()
                .and_then(|o| o.remoteEncryptionKey.as_ref())
                .cloned()
            {
                let encryption_context = libsql::EncryptionContext {
                    key: libsql::EncryptionKey::Base64Encoded(encryption_key),
                };
                builder = builder.remote_encryption(encryption_context);
            }
            rt.block_on(builder.build()).map_err(Error::from)?
        } else if let Some(options) = &opts {
            if let Some(sync_url) = &options.syncUrl {
                let auth_token = options.authToken.as_ref().cloned().unwrap_or_default();

                let encryption_cipher: String = opts
                    .as_ref()
                    .and_then(|o| o.encryptionCipher.as_ref())
                    .cloned()
                    .unwrap_or("aes256cbc".to_string());
                let cipher = libsql::Cipher::from_str(&encryption_cipher).map_err(|_| {
                    throw_sqlite_error(
                        "Invalid encryption cipher".to_string(),
                        "SQLITE_INVALID_ENCRYPTION_CIPHER".to_string(),
                        0,
                    )
                })?;
                let encryption_key = opts
                    .as_ref()
                    .and_then(|o| o.encryptionKey.as_ref())
                    .cloned()
                    .unwrap_or("".to_string());

                let mut builder =
                    libsql::Builder::new_remote_replica(path.clone(), sync_url.clone(), auth_token);

                let read_your_writes = options.readYourWrites.unwrap_or(true);
                builder = builder.read_your_writes(read_your_writes);

                if encryption_key.len() > 0 {
                    let encryption_config =
                        libsql::EncryptionConfig::new(cipher, encryption_key.into());
                    builder = builder.encryption_config(encryption_config);
                }

                if let Some(remote_encryption_key) = &options.remoteEncryptionKey {
                    let encryption_context = libsql::EncryptionContext {
                        key: libsql::EncryptionKey::Base64Encoded(
                            remote_encryption_key.to_string(),
                        ),
                    };
                    builder = builder.remote_encryption(encryption_context);
                }

                if let Some(period) = options.syncPeriod {
                    if period > 0.0 {
                        builder = builder.sync_interval(std::time::Duration::from_secs_f64(period));
                    }
                }

                rt.block_on(builder.build()).map_err(Error::from)?
            } else {
                let builder = libsql::Builder::new_local(&path);
                rt.block_on(builder.build()).map_err(Error::from)?
            }
        } else {
            let builder = libsql::Builder::new_local(&path);
            rt.block_on(builder.build()).map_err(Error::from)?
        };
        let conn = db.connect().map_err(Error::from)?;
        let default_safe_integers = AtomicBool::new(false);
        let memory = path == ":memory:";
        let timeout = match opts {
            Some(ref opts) => opts.timeout.unwrap_or(0.0),
            None => 0.0,
        };
        if timeout > 0.0 {
            conn.busy_timeout(Duration::from_millis(timeout as u64))
                .map_err(Error::from)?
        }
        Ok(Database {
            db,
            conn: Some(Arc::new(conn)),
            default_safe_integers,
            memory,
        })
    }

    /// Returns whether the database is in memory-only mode.
    #[napi(getter)]
    pub fn memory(&self) -> bool {
        self.memory
    }

    /// Returns whether the database is in a transaction.
    #[napi]
    pub fn in_transaction(&self) -> Result<bool> {
        let conn = match &self.conn {
            Some(conn) => conn.clone(),
            None => return Ok(false),
        };
        Ok(!conn.is_autocommit())
    }

    /// Prepares a statement for execution.
    ///
    /// # Arguments
    ///
    /// * `sql` - The SQL statement to prepare.
    ///
    /// # Returns
    ///
    /// A `Statement` instance.
    #[napi]
    pub async fn prepare(&self, sql: String) -> Result<Statement> {
        let conn = match &self.conn {
            Some(conn) => conn.clone(),
            None => {
                return Err(throw_sqlite_error(
                    "The database connection is not open".to_string(),
                    "SQLITE_NOTOPEN".to_string(),
                    0,
                ));
            }
        };
        let stmt = { conn.prepare(&sql).await.map_err(Error::from)? };
        let mode = AccessMode {
            safe_ints: self.default_safe_integers.load(Ordering::SeqCst).into(),
            raw: false.into(),
            pluck: false.into(),
            timing: false.into(),
        };
        Ok(Statement::new(conn, stmt, mode))
    }

    /// Sets the authorizer for the database.
    ///
    /// # Arguments
    ///
    /// * `env` - The environment.
    /// * `rules_obj` - The rules object.
    ///
    /// The `rules_obj` is a JavaScript object with the following properties:
    ///
    /// * `Authorization.ALLOW` - Allow access to the table.
    /// * `Authorization.DENY` - Deny access to the table.
    ///
    /// Example:
    ///
    /// ```javascript
    /// db.authorizer({
    ///     "users": Authorization.ALLOW
    /// });
    /// ```
    #[napi]
    pub fn authorizer(&self, env: Env, rules_obj: napi::JsObject) -> Result<()> {
        let conn = match &self.conn {
            Some(c) => c.clone(),
            None => {
                return Err(throw_database_closed_error(&env).into());
            }
        };
        let mut builder = crate::auth::AuthorizerBuilder::new();
        let prop_names = rules_obj.get_property_names()?;
        let len = prop_names.get_array_length()?;
        for idx in 0..len {
            let key_js: napi::JsString = prop_names.get_element::<napi::JsString>(idx)?;
            let key = key_js.into_utf8()?.into_owned()?;
            let value_js: napi::JsNumber = rules_obj.get_named_property(&key)?;
            let value = value_js.get_int32()?;
            match value {
                0 => {
                    // Authorization.ALLOW
                    builder.allow(&key);
                }
                1 => {
                    // Authorization.DENY
                    builder.deny(&key);
                }
                _ => {
                    let msg = format!(
                        "Invalid authorization rule value '{}' for table '{}'. Expected 0 (ALLOW) or 1 (DENY).",
                        value, key
                    );
                    return Err(napi::Error::from_reason(msg));
                }
            }
        }
        let authorizer = builder.build();
        let auth_arc = std::sync::Arc::new(authorizer);
        let closure = {
            let auth_arc = auth_arc.clone();
            move |ctx: &libsql::AuthContext| auth_arc.authorize(ctx)
        };
        conn.authorizer(Some(std::sync::Arc::new(closure)))
            .map_err(Error::from)?;
        Ok(())
    }

    /// Loads an extension into the database.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the extension file.
    /// * `entry_point` - The entry point of the extension.
    ///
    #[napi]
    pub fn loadExtension(&self, path: String, entry_point: Option<String>) -> Result<()> {
        let rt = runtime()?;
        let conn = match &self.conn {
            Some(conn) => conn.clone(),
            None => {
                return Err(throw_sqlite_error(
                    "The database connection is not open".to_string(),
                    "SQLITE_NOTOPEN".to_string(),
                    0,
                ));
            }
        };
        rt.block_on(async move {
            conn.load_extension_enable().map_err(Error::from)?;
            if let Err(err) = conn.load_extension(&path, entry_point.as_deref()) {
                let _ = conn.load_extension_disable();
                return Err(Error::from(err));
            }
            conn.load_extension_disable().map_err(Error::from)?;
            Ok(())
        })
        .map_err(|e| napi::Error::from(e))
    }

    /// Returns the maximum write replication index.
    ///
    /// # Returns
    ///
    /// The maximum write replication index.
    #[napi]
    pub fn max_write_replication_index(&self) -> Result<f64> {
        let result = self.db.max_write_replication_index();
        Ok(result.unwrap_or(0) as f64)
    }

    /// Executes a SQL statement.
    ///
    /// # Arguments
    ///
    /// * `env` - The environment.
    /// * `sql` - The SQL statement to execute.
    #[napi]
    pub async fn exec(&self, sql: String) -> Result<()> {
        let conn = match &self.conn {
            Some(conn) => conn.clone(),
            None => {
                return Err(throw_sqlite_error(
                    "The database connection is not open".to_string(),
                    "SQLITE_NOTOPEN".to_string(),
                    0,
                ));
            }
        };
        conn.execute_batch(&sql).await.map_err(Error::from)?;
        Ok(())
    }

    /// Syncs the database.
    ///
    /// # Returns
    ///
    /// A `SyncResult` instance.
    #[napi]
    pub async fn sync(&self) -> Result<SyncResult> {
        let result = self.db.sync().await.map_err(Error::from)?;
        Ok(SyncResult {
            frames_synced: result.frames_synced() as f64,
            replication_index: result.frame_no().unwrap_or(0) as f64,
        })
    }

    /// Interrupts any ongoing database operations.
    ///
    /// # Arguments
    ///
    /// * `env` - The environment.
    #[napi]
    pub fn interrupt(&self, env: Env) -> Result<()> {
        let conn = match &self.conn {
            Some(conn) => conn.clone(),
            None => return Err(throw_database_closed_error(&env).into()),
        };
        conn.interrupt().map_err(Error::from)?;
        Ok(())
    }

    /// Closes the database connection.
    #[napi]
    pub fn close(&mut self) -> Result<()> {
        self.conn = None;
        Ok(())
    }

    /// Sets the default safe integers mode.
    ///
    /// # Arguments
    ///
    /// * `toggle` - Whether to use safe integers by default.
    #[napi]
    pub fn defaultSafeIntegers(&self, toggle: Option<bool>) -> Result<()> {
        self.default_safe_integers
            .store(toggle.unwrap_or(true), Ordering::SeqCst);
        Ok(())
    }
}

/// Result of a database sync operation.
#[napi(object)]
pub struct SyncResult {
    /// The number of frames synced.
    pub frames_synced: f64,
    /// The replication index.
    pub replication_index: f64,
}

/// Prepares a statement in blocking mode.
#[napi]
pub fn database_prepare_sync(db: &Database, sql: String) -> Result<Statement> {
    let rt = runtime()?;
    rt.block_on(async move { db.prepare(sql).await })
}

/// Syncs the database in blocking mode.
#[napi]
pub fn database_sync_sync(db: &Database) -> Result<SyncResult> {
    let rt = runtime()?;
    rt.block_on(async move { db.sync().await })
}

/// Executes SQL in blocking mode.
#[napi]
pub fn database_exec_sync(db: &Database, sql: String) -> Result<()> {
    let rt = runtime()?;
    rt.block_on(async move { db.exec(sql).await })
}

fn is_remote_path(path: &str) -> bool {
    path.starts_with("libsql://") || path.starts_with("http://") || path.starts_with("https://")
}

fn throw_database_closed_error(env: &Env) -> napi::Error {
    let msg = "The database connection is not open";
    let err = napi::Error::new(napi::Status::InvalidArg, msg.to_string());
    env.throw_type_error(msg, None).unwrap();
    err
}

/// SQLite statement object.
#[napi]
pub struct Statement {
    // The libSQL connection instance.
    conn: Arc<libsql::Connection>,
    // The libSQL statement instance.
    stmt: Arc<libsql::Statement>,
    // The column names.
    column_names: Vec<std::ffi::CString>,
    // The access mode.
    mode: AccessMode,
}

#[napi]
impl Statement {
    /// Creates a new statement instance.
    ///
    /// # Arguments
    ///
    /// * `conn` - The connection instance.
    /// * `stmt` - The libSQL statement instance.
    /// * `mode` - The access mode.
    pub(crate) fn new(
        conn: Arc<libsql::Connection>,
        stmt: libsql::Statement,
        mode: AccessMode,
    ) -> Self {
        let column_names: Vec<std::ffi::CString> = stmt
            .columns()
            .iter()
            .map(|c| std::ffi::CString::new(c.name().to_string()).unwrap())
            .collect();
        let stmt = Arc::new(stmt);
        Self {
            conn,
            stmt,
            column_names,
            mode,
        }
    }

    /// Executes a SQL statement.
    ///
    /// # Arguments
    ///
    /// * `params` - The parameters to bind to the statement.
    #[napi]
    pub fn run(&self, params: Option<napi::JsUnknown>) -> Result<RunResult> {
        let rt = runtime()?;
        rt.block_on(async move {
            let total_changes_before = self.conn.total_changes();
            let start = std::time::Instant::now();

            let params = map_params(&self.stmt, params)?;
            self.stmt.run(params).await.map_err(Error::from)?;
            let changes = if self.conn.total_changes() == total_changes_before {
                0
            } else {
                self.conn.changes()
            };
            let last_insert_row_id = self.conn.last_insert_rowid();
            let duration = start.elapsed().as_secs_f64();
            self.stmt.reset();
            Ok(RunResult {
                changes: changes as f64,
                duration,
                lastInsertRowid: last_insert_row_id,
            })
        })
    }

    /// Executes a SQL statement and returns the first row.
    ///
    /// # Arguments
    ///
    /// * `env` - The environment.
    /// * `params` - The parameters to bind to the statement.
    #[napi]
    pub fn get(&self, env: Env, params: Option<napi::JsUnknown>) -> Result<napi::JsUnknown> {
        let rt = runtime()?;

        let safe_ints = self.mode.safe_ints.load(Ordering::SeqCst);
        let raw = self.mode.raw.load(Ordering::SeqCst);
        let pluck = self.mode.pluck.load(Ordering::SeqCst);
        let timed = self.mode.timing.load(Ordering::SeqCst);

        let start = if timed {
            Some(std::time::Instant::now())
        } else {
            None
        };
        rt.block_on(async move {
            let params = map_params(&self.stmt, params)?;
            let mut rows = self.stmt.query(params).await.map_err(Error::from)?;
            let row = rows.next().await.map_err(Error::from)?;
            let duration: Option<f64> = start.map(|start| start.elapsed().as_secs_f64());
            let result = Self::get_internal(
                &env,
                &row,
                &self.column_names,
                safe_ints,
                raw,
                pluck,
                duration,
            );
            self.stmt.reset();
            result
        })
    }

    fn get_internal(
        env: &Env,
        row: &Option<libsql::Row>,
        column_names: &[std::ffi::CString],
        safe_ints: bool,
        raw: bool,
        pluck: bool,
        duration: Option<f64>,
    ) -> Result<napi::JsUnknown> {
        match row {
            Some(row) => {
                if raw {
                    let js_array = map_row_raw(&env, &column_names, &row, safe_ints, pluck)?;
                    Ok(js_array.into_unknown())
                } else {
                    let mut js_object =
                        map_row_object(&env, &column_names, &row, safe_ints, pluck)?
                            .coerce_to_object()?;
                    if let Some(duration) = duration {
                        let mut metadata = env.create_object()?;
                        let js_duration = env.create_double(duration)?;
                        metadata.set_named_property("duration", js_duration)?;
                        js_object.set_named_property("_metadata", metadata)?;
                    }
                    Ok(js_object.into_unknown())
                }
            }
            None => {
                let undefined = env.get_undefined()?;
                Ok(undefined.into_unknown())
            }
        }
    }

    /// Create an iterator over the rows of a statement.
    ///
    /// # Arguments
    ///
    /// * `env` - The environment.
    /// * `params` - The parameters to bind to the statement.
    #[napi]
    pub fn iterate(&self, env: Env, params: Option<napi::JsUnknown>) -> Result<napi::JsObject> {
        let rt = runtime()?;
        let safe_ints = self.mode.safe_ints.load(Ordering::SeqCst);
        let raw = self.mode.raw.load(Ordering::SeqCst);
        let pluck = self.mode.pluck.load(Ordering::SeqCst);
        let stmt = self.stmt.clone();
        let params = {
            let stmt = stmt.clone();
            rt.block_on(async move {
                stmt.reset();
                map_params(&stmt, params).unwrap()
            })
        };
        let stmt = self.stmt.clone();
        let future = async move {
            let rows = stmt.query(params).await.map_err(Error::from)?;
            Ok::<_, napi::Error>(rows)
        };
        let column_names = self.column_names.clone();
        env.execute_tokio_future(future, move |&mut _env, result| {
            Ok(RowsIterator::new(
                Arc::new(tokio::sync::Mutex::new(result)),
                column_names,
                safe_ints,
                raw,
                pluck,
            ))
        })
    }

    #[napi]
    pub fn raw(&self, raw: Option<bool>) -> Result<&Self> {
        let returns_data = !self.stmt.columns().is_empty();
        if !returns_data {
            return Err(napi::Error::from_reason(
                "The raw() method is only for statements that return data",
            ));
        }
        self.mode.raw.store(raw.unwrap_or(true), Ordering::SeqCst);
        Ok(self)
    }

    #[napi]
    pub fn pluck(&self, pluck: Option<bool>) -> Result<&Self> {
        self.mode
            .pluck
            .store(pluck.unwrap_or(true), Ordering::SeqCst);
        Ok(self)
    }

    #[napi]
    pub fn timing(&self, timing: Option<bool>) -> Result<&Self> {
        self.mode
            .timing
            .store(timing.unwrap_or(true), Ordering::SeqCst);
        Ok(self)
    }

    #[napi]
    pub fn columns(&self, env: Env) -> Result<Array> {
        let columns = self.stmt.columns();
        let mut js_array = env.create_array(columns.len() as u32)?;
        for (i, col) in columns.iter().enumerate() {
            let mut js_obj = env.create_object()?;
            js_obj.set_named_property("name", env.create_string(col.name())?)?;
            // origin_name -> column
            if let Some(origin_name) = col.origin_name() {
                js_obj.set_named_property("column", env.create_string(origin_name)?)?;
            } else {
                js_obj.set_named_property("column", env.get_null()?)?;
            }
            // table_name -> table
            if let Some(table_name) = col.table_name() {
                js_obj.set_named_property("table", env.create_string(table_name)?)?;
            } else {
                js_obj.set_named_property("table", env.get_null()?)?;
            }
            // database_name -> database
            if let Some(database_name) = col.database_name() {
                js_obj.set_named_property("database", env.create_string(database_name)?)?;
            } else {
                js_obj.set_named_property("database", env.get_null()?)?;
            }
            // decl_type -> type
            if let Some(decl_type) = col.decl_type() {
                js_obj.set_named_property("type", env.create_string(decl_type)?)?;
            } else {
                js_obj.set_named_property("type", env.get_null()?)?;
            }
            js_array.set(i as u32, js_obj)?;
        }
        Ok(js_array)
    }

    #[napi]
    pub fn safeIntegers(&self, toggle: Option<bool>) -> Result<&Self> {
        self.mode
            .safe_ints
            .store(toggle.unwrap_or(true), Ordering::SeqCst);
        Ok(self)
    }

    #[napi]
    pub fn interrupt(&self) -> Result<()> {
        self.stmt.interrupt().map_err(Error::from)?;
        Ok(())
    }
}

#[napi]
pub fn statement_iterate_sync(
    stmt: &Statement,
    _env: Env,
    params: Option<napi::JsUnknown>,
) -> Result<RowsIterator> {
    let rt = runtime()?;
    let safe_ints = stmt.mode.safe_ints.load(Ordering::SeqCst);
    let raw = stmt.mode.raw.load(Ordering::SeqCst);
    let pluck = stmt.mode.pluck.load(Ordering::SeqCst);
    let stmt = stmt.stmt.clone();
    let (rows, column_names) = rt.block_on(async move {
        stmt.reset();
        let params = map_params(&stmt, params)?;
        let rows = stmt.query(params).await.map_err(Error::from)?;
        let mut column_names = Vec::new();
        for i in 0..rows.column_count() {
            column_names
                .push(std::ffi::CString::new(rows.column_name(i).unwrap().to_string()).unwrap());
        }
        Ok::<_, napi::Error>((rows, column_names))
    })?;
    Ok(RowsIterator::new(
        Arc::new(tokio::sync::Mutex::new(rows)),
        column_names,
        safe_ints,
        raw,
        pluck,
    ))
}

/// SQLite `run()` result object
#[napi(object)]
pub struct RunResult {
    pub changes: f64,
    pub duration: f64,
    pub lastInsertRowid: i64,
}

fn map_params(
    stmt: &libsql::Statement,
    params: Option<napi::JsUnknown>,
) -> Result<libsql::params::Params> {
    if let Some(params) = params {
        match params.get_type()? {
            ValueType::Object => {
                let object = params.coerce_to_object()?;
                if object.is_array()? {
                    map_params_array(object)
                } else {
                    map_params_object(stmt, object)
                }
            }
            _ => map_params_single(params),
        }
    } else {
        Ok(libsql::params::Params::None)
    }
}

fn map_params_single(param: napi::JsUnknown) -> Result<libsql::params::Params> {
    Ok(libsql::params::Params::Positional(vec![map_value(param)?]))
}

fn map_params_array(object: napi::JsObject) -> Result<libsql::params::Params> {
    let mut params = vec![];
    let length = object.get_array_length()?;
    for i in 0..length {
        let element = object.get_element::<napi::JsUnknown>(i)?;
        let value = map_value(element)?;
        params.push(value);
    }
    Ok(libsql::params::Params::Positional(params))
}

fn map_params_object(
    stmt: &libsql::Statement,
    object: napi::JsObject,
) -> Result<libsql::params::Params> {
    let mut params = vec![];
    for idx in 0..stmt.parameter_count() {
        let name = stmt.parameter_name((idx + 1) as i32).unwrap();
        let name = name.to_string();
        // Remove the leading ':' or '@' or '$' from parameter name
        let key = &name[1..];
        if let Ok(value) = object.get_named_property::<napi::JsUnknown>(key) {
            let value = map_value(value)?;
            params.push((name, value));
        }
    }
    Ok(libsql::params::Params::Named(params))
}

/// Maps a JavaScript value to libSQL value types.
fn map_value(value: JsUnknown) -> Result<libsql::Value> {
    let value_type = value.get_type()?;

    match value_type {
        ValueType::Null | ValueType::Undefined => Ok(libsql::Value::Null),

        ValueType::Boolean => {
            let js_bool = value.coerce_to_bool()?;
            let b = js_bool.get_value()?;
            Ok(libsql::Value::Integer(if b { 1 } else { 0 }))
        }

        ValueType::Number => {
            let js_num = value.coerce_to_number()?;
            let n = js_num.get_double()?;
            Ok(libsql::Value::Real(n))
        }

        ValueType::BigInt => {
            let js_bigint = napi::JsBigInt::from_unknown(value)?;
            let (v, lossless) = js_bigint.get_i64()?;
            if !lossless {
                return Err(napi::Error::from_reason(
                    "BigInt value is out of range for SQLite INTEGER (i64)",
                ));
            }
            Ok(libsql::Value::Integer(v))
        }

        ValueType::String => {
            let js_str = value.coerce_to_string()?;
            let utf8 = js_str.into_utf8()?;
            // into_utf8 returns a Utf8 object that derefs to str
            Ok(libsql::Value::Text(utf8.as_str()?.to_owned()))
        }

        ValueType::Object => {
            let obj = value.coerce_to_object()?;

            // Check if it's a buffer
            if obj.is_buffer()? {
                let buf = napi::JsBuffer::try_from(obj.into_unknown())?;
                let data = buf.into_value()?.to_vec();
                return Ok(libsql::Value::Blob(data));
            }

            if obj.is_typedarray()? {
                let js_typed = napi::JsTypedArray::try_from(obj.into_unknown())?;
                let typed_array_value = js_typed.into_value()?;

                let buffer_data = typed_array_value.arraybuffer.into_value()?;
                let start = typed_array_value.byte_offset;
                let end = start + typed_array_value.length;

                if end > buffer_data.len() {
                    return Err(napi::Error::from_reason("TypedArray length out of bounds"));
                }

                let slice = &buffer_data[start..end];
                return Ok(libsql::Value::Blob(slice.to_vec()));
            }
            Err(napi::Error::from_reason(
                "SQLite3 can only bind numbers, strings, bigints, buffers, and null",
            ))
        }

        _ => Err(napi::Error::from_reason(
            "SQLite3 can only bind numbers, strings, bigints, buffers, and null",
        )),
    }
}

/// A raw iterator over rows. The JavaScript layer wraps this in a iterable.
#[napi]
pub struct RowsIterator {
    rows: Arc<tokio::sync::Mutex<libsql::Rows>>,
    column_names: Vec<std::ffi::CString>,
    safe_ints: bool,
    raw: bool,
    pluck: bool,
}

#[napi]
impl RowsIterator {
    pub fn new(
        rows: Arc<tokio::sync::Mutex<libsql::Rows>>,
        column_names: Vec<std::ffi::CString>,
        safe_ints: bool,
        raw: bool,
        pluck: bool,
    ) -> Self {
        Self {
            rows,
            column_names,
            safe_ints,
            raw,
            pluck,
        }
    }

    #[napi]
    pub async fn next(&self) -> Result<Record> {
        let mut rows = self.rows.lock().await;
        let row = rows.next().await.map_err(Error::from)?;
        Ok(Record {
            row,
            column_names: self.column_names.clone(),
            safe_ints: self.safe_ints,
            raw: self.raw,
            pluck: self.pluck,
        })
    }
}

/// Retrieve next row from an iterator synchronously. Needed for better-sqlite3 API compatibility.
#[napi]
pub fn iterator_next_sync(iter: &RowsIterator) -> Result<Record> {
    let rt = runtime()?;
    rt.block_on(async move { iter.next().await })
}

#[napi]
pub struct Record {
    row: Option<libsql::Row>,
    column_names: Vec<std::ffi::CString>,
    safe_ints: bool,
    raw: bool,
    pluck: bool,
}

#[napi]
impl Record {
    #[napi(getter)]
    pub fn value(&self, env: Env) -> napi::Result<napi::JsUnknown> {
        if let Some(row) = &self.row {
            Ok(map_row(
                &env,
                &self.column_names,
                &row,
                self.safe_ints,
                self.raw,
                self.pluck,
            )?)
        } else {
            Ok(env.get_null()?.into_unknown())
        }
    }

    #[napi(getter)]
    pub fn done(&self) -> bool {
        self.row.is_none()
    }
}

fn runtime() -> Result<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    let rt = RUNTIME.get_or_try_init(Runtime::new).unwrap();
    Ok(rt)
}

fn map_row(
    env: &Env,
    column_names: &[std::ffi::CString],
    row: &libsql::Row,
    safe_ints: bool,
    raw: bool,
    pluck: bool,
) -> Result<napi::JsUnknown> {
    let result = if raw {
        map_row_raw(env, column_names, row, safe_ints, pluck)?
    } else {
        map_row_object(env, column_names, row, safe_ints, pluck)?.into_unknown()
    };
    Ok(result)
}

fn convert_value_to_js(
    env: &Env,
    value: &libsql::Value,
    safe_ints: bool,
) -> Result<napi::JsUnknown> {
    match value {
        libsql::Value::Null => Ok(env.get_null()?.into_unknown()),
        libsql::Value::Integer(v) => {
            if safe_ints {
                Ok(env.create_bigint_from_i64(*v)?.into_unknown()?)
            } else {
                Ok(env.create_double(*v as f64)?.into_unknown())
            }
        }
        libsql::Value::Real(v) => Ok(env.create_double(*v)?.into_unknown()),
        libsql::Value::Text(v) => Ok(env.create_string(v)?.into_unknown()),
        libsql::Value::Blob(v) => Ok(env.create_buffer_with_data(v.clone())?.into_unknown()),
    }
}

fn map_row_object(
    env: &Env,
    column_names: &[std::ffi::CString],
    row: &libsql::Row,
    safe_ints: bool,
    pluck: bool,
) -> Result<napi::JsUnknown> {
    let column_count = column_names.len();

    let result = if pluck {
        if column_count > 0 {
            let value = match row.get_value(0) {
                Ok(v) => v,
                Err(e) => return Err(napi::Error::from_reason(e.to_string())),
            };
            convert_value_to_js(env, &value, safe_ints)?
        } else {
            env.get_null()?.into_unknown()
        }
    } else {
        let result = env.create_object()?;
        let result = unsafe { napi::JsObject::to_napi_value(env.raw(), result)? };
        // If not plucking, get all columns
        for idx in 0..column_count {
            let value = match row.get_value(idx as i32) {
                Ok(v) => v,
                Err(e) => return Err(napi::Error::from_reason(e.to_string())),
            };

            let column_name = &column_names[idx];
            let js_value = convert_value_to_js(env, &value, safe_ints)?;
            unsafe {
                napi::sys::napi_set_named_property(
                    env.raw(),
                    result,
                    column_name.as_ptr(),
                    napi::JsUnknown::to_napi_value(env.raw(), js_value)?,
                );
            }
        }
        let result: napi::JsObject = unsafe { napi::JsObject::from_napi_value(env.raw(), result)? };
        result.into_unknown()
    };
    Ok(result)
}

fn map_row_raw(
    env: &Env,
    column_names: &[std::ffi::CString],
    row: &libsql::Row,
    safe_ints: bool,
    pluck: bool,
) -> Result<napi::JsUnknown> {
    if pluck {
        let value = match row.get_value(0) {
            Ok(v) => convert_value_to_js(env, &v, safe_ints)?,
            Err(_) => env.get_null()?.into_unknown(),
        };
        return Ok(value);
    }
    let column_count = column_names.len();
    let mut arr = env.create_array(column_count as u32)?;
    for idx in 0..column_count {
        let value = match row.get_value(idx as i32) {
            Ok(v) => v,
            Err(e) => return Err(napi::Error::from_reason(e.to_string())),
        };
        let js_value = convert_value_to_js(env, &value, safe_ints)?;
        arr.set(idx as u32, js_value)?;
    }
    Ok(arr.coerce_to_object()?.into_unknown())
}

static LOGGER_INIT: OnceCell<()> = OnceCell::new();

fn ensure_logger() {
    LOGGER_INIT.get_or_init(|| {
        let _ = tracing_subscriber::fmt::fmt()
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::ERROR.into())
                    .from_env_lossy(),
            )
            .try_init();
    });
}
