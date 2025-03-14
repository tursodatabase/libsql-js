use libsql::replication::Replicated;
use neon::prelude::*;
use std::cell::RefCell;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::trace;

use crate::errors::{throw_database_closed_error, throw_libsql_error};
use crate::runtime;
use crate::Statement;

pub(crate) struct Database {
    db: Arc<Mutex<libsql::Database>>,
    conn: RefCell<Option<Arc<Mutex<libsql::Connection>>>>,
    default_safe_integers: RefCell<bool>,
}

impl Finalize for Database {}

impl Database {
    pub fn new(db: libsql::Database, conn: libsql::Connection) -> Self {
        Database {
            db: Arc::new(Mutex::new(db)),
            conn: RefCell::new(Some(Arc::new(Mutex::new(conn)))),
            default_safe_integers: RefCell::new(false),
        }
    }

    pub fn js_open(mut cx: FunctionContext) -> JsResult<JsBox<Database>> {
        let rt = runtime(&mut cx)?;
        let db_path = cx.argument::<JsString>(0)?.value(&mut cx);
        let auth_token = cx.argument::<JsString>(1)?.value(&mut cx);
        let encryption_cipher = cx.argument::<JsString>(2)?.value(&mut cx);
        let encryption_key = cx.argument::<JsString>(3)?.value(&mut cx);
        let busy_timeout = cx.argument::<JsNumber>(4)?.value(&mut cx);
        let db = if is_remote_path(&db_path) {
            let version = version("remote");
            trace!("Opening remote database: {}", db_path);
            libsql::Database::open_remote_internal(db_path.clone(), auth_token, version)
        } else {
            let cipher = libsql::Cipher::from_str(&encryption_cipher).or_else(|err| {
                throw_libsql_error(
                    &mut cx,
                    libsql::Error::SqliteFailure(err.extended_code, "".into()),
                )
            })?;
            let mut builder = libsql::Builder::new_local(&db_path);
            if !encryption_key.is_empty() {
                let encryption_config =
                    libsql::EncryptionConfig::new(cipher, encryption_key.into());
                builder = builder.encryption_config(encryption_config);
            }
            rt.block_on(builder.build())
        }
        .or_else(|err| throw_libsql_error(&mut cx, err))?;
        let conn = db
            .connect()
            .or_else(|err| throw_libsql_error(&mut cx, err))?;
        if busy_timeout > 0.0 {
            conn.busy_timeout(Duration::from_millis(busy_timeout as u64))
                .or_else(|err| throw_libsql_error(&mut cx, err))?;
        }
        let db = Database::new(db, conn);
        Ok(cx.boxed(db))
    }

    pub fn js_open_with_sync(mut cx: FunctionContext) -> JsResult<JsBox<Database>> {
        let db_path = cx.argument::<JsString>(0)?.value(&mut cx);
        let sync_url = cx.argument::<JsString>(1)?.value(&mut cx);
        let sync_auth = cx.argument::<JsString>(2)?.value(&mut cx);
        let encryption_cipher = cx.argument::<JsString>(3)?.value(&mut cx);
        let encryption_key = cx.argument::<JsString>(4)?.value(&mut cx);
        let sync_period = cx.argument::<JsNumber>(5)?.value(&mut cx);
        let read_your_writes = cx.argument::<JsBoolean>(6)?.value(&mut cx);
        let offline = cx.argument::<JsBoolean>(7)?.value(&mut cx);

        let cipher = libsql::Cipher::from_str(&encryption_cipher).or_else(|err| {
            throw_libsql_error(
                &mut cx,
                libsql::Error::SqliteFailure(err.extended_code, "".into()),
            )
        })?;
        let encryption_config = if encryption_key.is_empty() {
            None
        } else {
            Some(libsql::EncryptionConfig::new(cipher, encryption_key.into()))
        };

        let sync_period = if sync_period > 0.0 {
            Some(Duration::from_secs_f64(sync_period))
        } else {
            None
        };
        let version = version("rpc");

        trace!(
            "Opening local database with sync: database = {}, URL = {}",
            db_path,
            sync_url
        );
        let rt = runtime(&mut cx)?;
        let result = if offline {
            rt.block_on(libsql::Builder::new_synced_database(db_path, sync_url, sync_auth).build())
        } else {
            rt.block_on(async {
                let mut builder = libsql::Builder::new_remote_replica(db_path, sync_url, sync_auth);
                if let Some(encryption_config) = encryption_config {
                    builder = builder.encryption_config(encryption_config);
                }
                if let Some(sync_period) = sync_period {
                    builder = builder.sync_interval(sync_period);
                }
                builder.build().await
            })
        };
        let db = result.or_else(|err| cx.throw_error(err.to_string()))?;
        let conn = db
            .connect()
            .or_else(|err| throw_libsql_error(&mut cx, err))?;
        let db = Database::new(db, conn);
        Ok(cx.boxed(db))
    }

    pub fn js_in_transaction(mut cx: FunctionContext) -> JsResult<JsValue> {
        let db = cx.argument::<JsBox<Database>>(0)?;
        let conn = db.conn.borrow();
        let conn = conn.as_ref().unwrap().clone();
        let result = !conn.blocking_lock().is_autocommit();
        Ok(cx.boolean(result).upcast())
    }

    pub fn js_interrupt(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let conn = db.conn.borrow();
        let conn = conn.as_ref().unwrap().clone();
        conn.blocking_lock().interrupt().or_else(|err| {
            throw_libsql_error(&mut cx, err)?;
            Ok(())
        })?;
        Ok(cx.undefined())
    }

    pub fn js_close(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        // the conn will be closed when the last statement in discarded. In most situation that
        // means immediately because you don't want to hold on a statement for longer that its
        // database is alive.
        trace!("Closing database");
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        db.conn.replace(None);
        Ok(cx.undefined())
    }

    pub fn js_max_write_replication_index(mut cx: FunctionContext) -> JsResult<JsValue> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let replication_index = db.db.blocking_lock().max_write_replication_index();
        Ok(if let Some(ri) = replication_index {
            cx.number(ri as f64).upcast()
        } else {
            cx.undefined().upcast()
        })
    }

    pub fn js_sync_sync(mut cx: FunctionContext) -> JsResult<JsObject> {
        trace!("Synchronizing database (sync)");
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let db = db.db.clone();
        let rt = runtime(&mut cx)?;
        let rep = rt
            .block_on(async move {
                let db = db.lock().await;
                db.sync().await
            })
            .or_else(|err| throw_libsql_error(&mut cx, err))?;

        let obj = convert_replicated_to_object(&mut cx, &rep)?;

        Ok(obj)
    }

    pub fn js_sync_async(mut cx: FunctionContext) -> JsResult<JsPromise> {
        trace!("Synchronizing database (async)");
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let db = db.db.clone();
        let rt = runtime(&mut cx)?;
        rt.spawn(async move {
            let result = db.lock().await.sync().await;
            match result {
                Ok(rep) => {
                    deferred.settle_with(&channel, move |mut cx| {
                        convert_replicated_to_object(&mut cx, &rep)
                    });
                }
                Err(err) => {
                    deferred.settle_with(&channel, |mut cx| {
                        throw_libsql_error(&mut cx, err)?;
                        Ok(cx.undefined())
                    });
                }
            }
        });
        Ok(promise)
    }

    pub fn js_sync_until_sync(mut cx: FunctionContext) -> JsResult<JsObject> {
        trace!("Synchronizing database until given replication index (sync)");
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let db = db.db.clone();
        let replication_index = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;
        let rt = runtime(&mut cx)?;
        let rep = rt
            .block_on(async move {
                let db = db.lock().await;
                db.sync_until(replication_index).await
            })
            .or_else(|err| throw_libsql_error(&mut cx, err))?;

        let obj = convert_replicated_to_object(&mut cx, &rep)?;

        Ok(obj)
    }

    pub fn js_sync_until_async(mut cx: FunctionContext) -> JsResult<JsPromise> {
        trace!("Synchronizing database until given replication index (async)");
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let replication_index = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let db = db.db.clone();
        let rt = runtime(&mut cx)?;
        rt.spawn(async move {
            let result = db.lock().await.sync_until(replication_index).await;
            match result {
                Ok(rep) => {
                    deferred.settle_with(&channel, move |mut cx| {
                        convert_replicated_to_object(&mut cx, &rep)
                    });
                }
                Err(err) => {
                    deferred.settle_with(&channel, |mut cx| {
                        throw_libsql_error(&mut cx, err)?;
                        Ok(cx.undefined())
                    });
                }
            }
        });
        Ok(promise)
    }

    pub fn js_exec_sync(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        trace!("Executing SQL statement (sync): {}", sql);
        let conn = match db.get_conn(&mut cx) {
            Some(conn) => conn,
            None => throw_database_closed_error(&mut cx)?,
        };
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(async { conn.lock().await.execute_batch(&sql).await });
        result.or_else(|err| throw_libsql_error(&mut cx, err))?;
        Ok(cx.undefined())
    }

    pub fn js_exec_async(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        trace!("Executing SQL statement (async): {}", sql);
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let conn = match db.get_conn(&mut cx) {
            Some(conn) => conn,
            None => {
                deferred.settle_with(&channel, |mut cx| {
                    throw_database_closed_error(&mut cx)?;
                    Ok(cx.undefined())
                });
                return Ok(promise);
            }
        };
        let rt = runtime(&mut cx)?;
        rt.spawn(async move {
            match conn.lock().await.execute_batch(&sql).await {
                Ok(_) => {
                    deferred.settle_with(&channel, |mut cx| Ok(cx.undefined()));
                }
                Err(err) => {
                    deferred.settle_with(&channel, |mut cx| {
                        throw_libsql_error(&mut cx, err)?;
                        Ok(cx.undefined())
                    });
                }
            }
        });
        Ok(promise)
    }

    pub fn js_prepare_sync(mut cx: FunctionContext) -> JsResult<JsBox<Statement>> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        trace!("Preparing SQL statement (sync): {}", sql);
        let conn = match db.get_conn(&mut cx) {
            Some(conn) => conn,
            None => throw_database_closed_error(&mut cx)?,
        };
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(async { conn.lock().await.prepare(&sql).await });
        let stmt = result.or_else(|err| throw_libsql_error(&mut cx, err))?;
        let stmt = Arc::new(Mutex::new(stmt));
        let stmt = Statement {
            conn: conn.clone(),
            stmt,
            raw: RefCell::new(false),
            safe_ints: RefCell::new(*db.default_safe_integers.borrow()),
        };
        Ok(cx.boxed(stmt))
    }

    pub fn js_prepare_async(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        trace!("Preparing SQL statement (async): {}", sql);
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let safe_ints = *db.default_safe_integers.borrow();
        let rt = runtime(&mut cx)?;
        let conn = match db.get_conn(&mut cx) {
            Some(conn) => conn,
            None => {
                deferred.settle_with(&channel, |mut cx| {
                    throw_database_closed_error(&mut cx)?;
                    Ok(cx.undefined())
                });
                return Ok(promise);
            }
        };
        rt.spawn(async move {
            match conn.lock().await.prepare(&sql).await {
                Ok(stmt) => {
                    let stmt = Arc::new(Mutex::new(stmt));
                    let stmt = Statement {
                        conn: conn.clone(),
                        stmt,
                        raw: RefCell::new(false),
                        safe_ints: RefCell::new(safe_ints),
                    };
                    deferred.settle_with(&channel, |mut cx| Ok(cx.boxed(stmt)));
                }
                Err(err) => {
                    deferred.settle_with(&channel, |mut cx| {
                        throw_libsql_error(&mut cx, err)?;
                        Ok(cx.undefined())
                    });
                }
            }
        });
        Ok(promise)
    }

    pub fn js_default_safe_integers(mut cx: FunctionContext) -> JsResult<JsNull> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let toggle = cx.argument::<JsBoolean>(0)?;
        let toggle = toggle.value(&mut cx);
        db.set_default_safe_integers(toggle);
        Ok(cx.null())
    }

    pub fn set_default_safe_integers(&self, toggle: bool) {
        self.default_safe_integers.replace(toggle);
    }

    pub fn js_load_extension(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let extension = cx.argument::<JsString>(0)?.value(&mut cx);
        let entry_point: Option<&str> = match cx.argument_opt(1) {
            Some(_arg) => todo!(),
            None => None,
        };
        trace!("Loading extension: {}", extension);
        let conn = match db.get_conn(&mut cx) {
            Some(conn) => conn,
            None => throw_database_closed_error(&mut cx)?,
        };
        let conn = conn.blocking_lock();
        if let Err(err) = conn.load_extension_enable() {
            throw_libsql_error(&mut cx, err)?;
        }
        if let Err(err) = conn.load_extension(&extension, entry_point) {
            let _ = conn.load_extension_disable();
            throw_libsql_error(&mut cx, err)?;
        }
        if let Err(err) = conn.load_extension_disable() {
            throw_libsql_error(&mut cx, err)?;
        }
        Ok(cx.undefined())
    }

    fn get_conn(&self, _cx: &mut FunctionContext) -> Option<Arc<Mutex<libsql::Connection>>> {
        let conn = self.conn.borrow();
        conn.as_ref().map(|conn| conn.clone())
    }
}

fn is_remote_path(path: &str) -> bool {
    path.starts_with("libsql://") || path.starts_with("http://") || path.starts_with("https://")
}

fn version(protocol: &str) -> String {
    let ver = env!("CARGO_PKG_VERSION");
    format!("libsql-js-{protocol}-{ver}")
}

fn convert_replicated_to_object<'a>(
    cx: &mut impl Context<'a>,
    rep: &Replicated,
) -> JsResult<'a, JsObject> {
    let obj = cx.empty_object();

    let frames_synced = cx.number(rep.frames_synced() as f64);
    obj.set(cx, "frames_synced", frames_synced)?;

    if let Some(v) = rep.frame_no() {
        let frame_no = cx.number(v as f64);
        obj.set(cx, "frame_no", frame_no)?;
    } else {
        let undef = cx.undefined();
        obj.set(cx, "frame_no", undef)?;
    }

    Ok(obj)
}
