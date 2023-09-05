use neon::{prelude::*, types::JsBigInt};
use neon::types::JsPromise;
use std::cell::RefCell;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use std::sync::{Weak, Mutex};

fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME
        .get_or_try_init(Runtime::new)
        .or_else(|err| cx.throw_error(&err.to_string()))
}

struct Database {
    db: libsql::v2::Database,
    conn: RefCell<Option<Arc<libsql::v2::Connection>>>,
    stmts: Arc<Mutex<Vec<Arc<libsql::v2::Statement>>>>,
    default_safe_integers: RefCell<bool>,
}

unsafe impl Sync for Database {}
unsafe impl Send for Database {}

impl Finalize for Database {}

impl Database {
    fn new(
        db: libsql::v2::Database,
        conn: libsql::v2::Connection,
    ) -> Self {
        Database {
            db,
            conn: RefCell::new(Some(Arc::new(conn))),
            stmts: Arc::new(Mutex::new(vec![])),
            default_safe_integers: RefCell::new(false),
        }
    }

    fn js_open(mut cx: FunctionContext) -> JsResult<JsBox<Database>> {
        let db_path = cx.argument::<JsString>(0)?.value(&mut cx);
        let auth_token = cx.argument::<JsString>(1)?.value(&mut cx);
        let rt = runtime(&mut cx)?;
        let db = if is_remote_path(&db_path) {
            libsql::v2::Database::open_remote(db_path.clone(), auth_token)
        } else {
            libsql::v2::Database::open(db_path.clone())
        }.or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let fut = db.connect();
        let result = rt.block_on(fut);
        let conn = result.or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let db = Database::new(db, conn);
        Ok(cx.boxed(db))
    }

    fn js_open_with_rpc_sync(mut cx: FunctionContext) -> JsResult<JsBox<Database>> {
        let db_path = cx.argument::<JsString>(0)?.value(&mut cx);
        let sync_url = cx.argument::<JsString>(1)?.value(&mut cx);
        let sync_auth = cx.argument::<JsString>(2)?.value(&mut cx);
        let rt = runtime(&mut cx)?;
        let fut = libsql::v2::Database::open_with_sync(db_path, sync_url, sync_auth);
        let result = rt.block_on(fut);
        let db = result.or_else(|err| cx.throw_error(err.to_string()))?;
        let fut = db.connect();
        let result = rt.block_on(fut);
        let conn = result.or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let db = Database::new(db, conn);
        Ok(cx.boxed(db))
    }

    fn js_in_transaction(mut cx: FunctionContext) -> JsResult<JsValue> {
        let db = cx.argument::<JsBox<Database>>(0)?;
        let conn = db.conn.borrow();
        let conn = conn.as_ref().unwrap().clone();
        let result = !conn.is_autocommit();
        Ok(cx.boolean(result).upcast())
    }

    fn js_close(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        db.stmts.lock().unwrap().clear();
        db.conn.replace(None);
        Ok(cx.undefined())
    }

    fn js_sync(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let rt = runtime(&mut cx)?;
        rt.block_on(db.db.sync()).or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        Ok(cx.undefined())
    }

    fn js_exec_sync(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        let conn = db.conn.borrow();
        let conn = conn.as_ref().unwrap().clone();
        let fut = conn.execute(&sql, ());
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(fut);
        result.or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        Ok(cx.undefined())
    }

    fn js_exec_async(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let conn = db.conn.borrow();
        let conn = conn.as_ref().unwrap().clone();
        let rt = runtime(&mut cx)?;
        rt.spawn(async move {
            let fut = conn.execute(&sql, ());
            match fut.await {
                Ok(_) => {
                    deferred.settle_with(&channel, |mut cx| {
                        Ok(cx.undefined())
                    });
                }
                Err(err) => {
                    deferred.settle_with(&channel, |mut cx| {
                        cx.throw_error(from_libsql_error(err))?;
                        Ok(cx.undefined())
                    });
                },
            }
        });
        Ok(promise)
    }

    fn js_prepare_sync<'a>(mut cx: FunctionContext) -> JsResult<JsBox<Statement>> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        let conn = db.conn.borrow();
        let conn = conn.as_ref().unwrap();
        let fut = conn.prepare(&sql);
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(fut);
        let stmt = result.or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let stmt = Arc::new(stmt);
        {
            let mut stmts = db.stmts.lock().unwrap();
            stmts.push(stmt.clone());
        }
        let stmt = Statement {
            conn: Arc::downgrade(&conn),
            stmt: Arc::downgrade(&stmt),
            raw: RefCell::new(false),
            safe_ints: RefCell::new(*db.default_safe_integers.borrow()),
        };
        Ok(cx.boxed(stmt))
    }

    fn js_prepare_async<'a>(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let safe_ints = *db.default_safe_integers.borrow();
        let rt = runtime(&mut cx)?;
        let conn = db.conn.borrow().clone().unwrap();
        let stmts = db.stmts.clone();
        rt.spawn(async move {
            let fut = conn.prepare(&sql);
            match fut.await {
                Ok(stmt) => {
                    let stmt = Arc::new(stmt);
                    {
                        let mut stmts = stmts.lock().unwrap();
                        stmts.push(stmt.clone());
                    }
                    let stmt = Statement {
                        conn: Arc::downgrade(&conn),
                        stmt: Arc::downgrade(&stmt),
                        raw: RefCell::new(false),
                        safe_ints: RefCell::new(safe_ints),
                    };
                    deferred.settle_with(&channel, |mut cx| Ok(cx.boxed(stmt)));
                }
                Err(err) => {
                    deferred.settle_with(&channel, |mut cx| {
                        cx.throw_error(from_libsql_error(err))?;
                        Ok(cx.undefined())
                    });
                },
            }
        });
        Ok(promise)
    }

    fn js_default_safe_integers(mut cx: FunctionContext) -> JsResult<JsNull> {
        let db: Handle<'_, JsBox<Database>> = cx.this()?;
        let toggle = cx.argument::<JsBoolean>(0)?;
        let toggle = toggle.value(&mut cx);
        db.set_default_safe_integers(toggle);
        Ok(cx.null())
    }

    fn set_default_safe_integers(&self, toggle: bool) {
        self.default_safe_integers.replace(toggle);
    }

}

fn is_remote_path(path: &str) -> bool {
    path.starts_with("libsql://") || path.starts_with("http://") || path.starts_with("https://")
}

fn from_libsql_error(err: libsql::Error) -> String {
    match err {
        libsql::Error::PrepareFailed(_, _, err) => err,
        _ => err.to_string(),
    }
}

struct Statement {
    conn: Weak<libsql::v2::Connection>,
    stmt: Weak<libsql::v2::Statement>,
    raw: RefCell<bool>,
    safe_ints: RefCell<bool>,
}

unsafe impl<'a> Sync for Statement {}
unsafe impl<'a> Send for Statement {}

impl<'a> Finalize for Statement {}

fn js_value_to_value(
    cx: &mut FunctionContext,
    v: Handle<'_, JsValue>,
) -> NeonResult<libsql::Value> {
    if v.is_a::<JsNull, _>(cx) {
        Ok(libsql::Value::Null)
    } else if v.is_a::<JsUndefined, _>(cx) {
        Ok(libsql::Value::Null)
    } else if v.is_a::<JsArray, _>(cx) {
        todo!("array");
    } else if v.is_a::<JsBoolean, _>(cx) {
        todo!("bool");
    } else if v.is_a::<JsNumber, _>(cx) {
        let v = v.downcast_or_throw::<JsNumber, _>(cx)?;
        let v = v.value(cx);
        Ok(libsql::Value::Integer(v as i64))
    } else if v.is_a::<JsString, _>(cx) {
        let v = v.downcast_or_throw::<JsString, _>(cx)?;
        let v = v.value(cx);
        Ok(libsql::Value::Text(v))
    } else if v.is_a::<JsBigInt, _>(cx) {
        let v = v.downcast_or_throw::<JsBigInt, _>(cx)?;
        let v = v.to_i64(cx).or_throw(cx)?;
        Ok(libsql::Value::Integer(v))
    } else {
        todo!("unsupported type");
    }
}

impl Statement {
    fn js_raw(mut cx: FunctionContext) -> JsResult<JsNull> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        if stmt.stmt.upgrade().unwrap().columns().len() == 0 {
            return cx.throw_error("The raw() method is only for statements that return data");
        }
        let raw = cx.argument::<JsBoolean>(0)?;
        let raw = raw.value(&mut cx);
        stmt.set_raw(raw);
        Ok(cx.null())
    }

    fn set_raw(&self, raw: bool) {
        self.raw.replace(raw);
    }

    fn js_run(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let params = cx.argument::<JsValue>(0)?;
        let params = convert_params(&mut cx, &stmt, params)?;
        let raw_stmt = stmt.stmt.upgrade().unwrap();
        raw_stmt.reset();
        let fut = raw_stmt.execute(&params);
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(fut);
        let changes = result.or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let raw_conn = stmt.conn.upgrade().unwrap();
        let last_insert_rowid = raw_conn.last_insert_rowid();
        let info = cx.empty_object();
        let changes = cx.number(changes as f64);
        info.set(&mut cx, "changes", changes)?;
        let last_insert_row_id = cx.number(last_insert_rowid as f64);
        info.set(&mut cx, "lastInsertRowid", last_insert_row_id)?;
        Ok(info.upcast())
    }

    fn js_get(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let params = cx.argument::<JsValue>(0)?;
        let params = convert_params(&mut cx, &stmt, params)?;
        let safe_ints = *stmt.safe_ints.borrow();
        let raw_stmt = stmt.stmt.upgrade().unwrap();
        let fut = raw_stmt.query(&params);
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(fut);
        let mut rows = result.or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let result = match rows
            .next()
            .or_else(|err| cx.throw_error(from_libsql_error(err)))?
        {
            Some(row) => {
                if *stmt.raw.borrow() {
                    let mut result = cx.empty_array();
                    convert_row_raw(&mut cx, safe_ints, &mut result, &rows, &row)?;
                    Ok(result.upcast())
                } else {
                    let mut result = cx.empty_object();
                    convert_row(&mut cx, safe_ints, &mut result, &rows, &row)?;
                    Ok(result.upcast())
                }
            }
            None => Ok(cx.undefined().upcast()),
        };
        raw_stmt.reset();
        result
    }

    fn js_rows_sync(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let params = cx.argument::<JsValue>(0)?;
        let params = convert_params(&mut cx, &stmt, params)?;
        let raw_stmt = stmt.stmt.upgrade().unwrap();
        raw_stmt.reset();
        let fut = raw_stmt.query(&params);
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(fut);
        let rows = result.or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let rows = Rows {
            rows: RefCell::new(rows),
            raw: *stmt.raw.borrow(),
            safe_ints: *stmt.safe_ints.borrow(),
        };
        Ok(cx.boxed(rows).upcast())
    }

    fn js_rows_async(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let params = cx.argument::<JsValue>(0)?;
        let params = convert_params(&mut cx, &stmt, params)?;
        let raw_stmt = stmt.stmt.upgrade().unwrap();
        raw_stmt.reset();
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let rt = runtime(&mut cx)?;
        let raw = *stmt.raw.borrow();
        let safe_ints = *stmt.safe_ints.borrow();
        let stmt = stmt.stmt.clone();
        rt.spawn(async move {
            let fut = raw_stmt.query(&params);
            match fut.await {
                Ok(rows) => {
                    deferred.settle_with(&channel, move |mut cx| {
                        let rows = Rows {
                            rows: RefCell::new(rows),
                            raw,
                            safe_ints,
                        };
                        Ok(cx.boxed(rows))
                    });
                }
                Err(err) => {
                    deferred.settle_with(&channel, |mut cx| {
                        cx.throw_error(from_libsql_error(err))?;
                        Ok(cx.undefined())
                    });
                },
            }
        });
        Ok(promise)
    }

    fn js_columns(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let result = cx.empty_array();
        let raw_stmt = stmt.stmt.upgrade().unwrap();
        for (i, col) in raw_stmt.columns().iter().enumerate() {
            let column = cx.empty_object();
            let column_name = cx.string(col.name());
            column.set(&mut cx, "name", column_name)?;
            let column_origin_name: Handle<'_, JsValue> = if let Some(origin_name) = col.origin_name() {
                cx.string(origin_name).upcast()
            } else {
                cx.null().upcast()
            };
            column.set(&mut cx, "column", column_origin_name)?;
            let column_table_name: Handle<'_, JsValue> = if let Some(table_name) = col.table_name() {
                cx.string(table_name).upcast()
            } else {
                cx.null().upcast()
            };
            column.set(&mut cx, "table", column_table_name)?;
            let column_database_name: Handle<'_, JsValue> = if let Some(database_name) = col.database_name() {
                cx.string(database_name).upcast()
            } else {
                cx.null().upcast()
            };
            column.set(&mut cx, "database", column_database_name)?;
            let column_decl_type: Handle<'_, JsValue> = if let Some(decl_type) = col.decl_type() {
                cx.string(decl_type).upcast()
            } else {
                cx.null().upcast()
            };
            column.set(&mut cx, "type", column_decl_type)?;
            result.set(&mut cx, i as u32, column)?;
        }
        Ok(result.upcast())
    }

    fn js_safe_integers(mut cx: FunctionContext) -> JsResult<JsNull> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let toggle = cx.argument::<JsBoolean>(0)?;
        let toggle = toggle.value(&mut cx);
        stmt.set_safe_integers(toggle);
        Ok(cx.null())
    }

    fn set_safe_integers(&self, toggle: bool) {
        self.safe_ints.replace(toggle);
    }
}

struct Rows {
    rows: RefCell<libsql::v2::Rows>,
    raw: bool,
    safe_ints: bool,
}

impl Finalize for Rows {}

impl Rows {
    fn js_next(mut cx: FunctionContext) -> JsResult<JsValue> {
        let rows: Handle<'_, JsBox<Rows>> = cx.this()?;
        let raw = rows.raw;
        let safe_ints = rows.safe_ints;
        let mut rows = rows.rows.borrow_mut();
        match rows
            .next()
            .or_else(|err| cx.throw_error(from_libsql_error(err)))?
        {
            Some(row) => {
                if raw {
                    let mut result = cx.empty_array();
                    convert_row_raw(&mut cx, safe_ints, &mut result, &rows, &row)?;
                    Ok(result.upcast())
                } else {
                    let mut result = cx.empty_object();
                    convert_row(&mut cx, safe_ints, &mut result, &rows, &row)?;
                    Ok(result.upcast())
                }
            }
            None => Ok(cx.undefined().upcast()),
        }
    }
}

fn convert_params(cx: &mut FunctionContext, stmt: &Statement, v: Handle<'_, JsValue>) -> NeonResult<libsql::Params> {
    if v.is_a::<JsArray, _>(cx) {
        let v = v.downcast_or_throw::<JsArray, _>(cx)?;
        convert_params_array(cx, v)
    } else {
        let v = v.downcast_or_throw::<JsObject, _>(cx)?;
        convert_params_object(cx, stmt, v)
    }
}

fn convert_params_array(
    cx: &mut FunctionContext,
    v: Handle<'_, JsArray>,
) -> NeonResult<libsql::Params> {
    let mut params = vec![];
    for i in 0..v.len(cx) {
        let v = v.get(cx, i)?;
        let v = js_value_to_value(cx, v)?;
        params.push(v);
    }
    Ok(libsql::Params::Positional(params))
}

fn convert_params_object(
    cx: &mut FunctionContext,
    stmt: &Statement,
    v: Handle<'_, JsObject>,
) -> NeonResult<libsql::Params> {
    let mut params = vec![];
    let raw_stmt = stmt.stmt.upgrade().unwrap();
    for idx in 0..raw_stmt.parameter_count() {
        let name = raw_stmt.parameter_name((idx + 1) as i32).unwrap();
        let name = name.to_string();
        let v = v.get(cx, &name[1..])?;
        let v = js_value_to_value(cx, v)?;
        params.push((name, v));
    }
    Ok(libsql::Params::Named(params))
}

fn convert_row(
    cx: &mut FunctionContext,
    safe_ints: bool,
    result: &mut JsObject,
    rows: &libsql::v2::Rows,
    row: &libsql::v2::Row,
) -> NeonResult<()> {
    for idx in 0..rows.column_count() {
        let v = row
            .get_value(idx)
            .or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let column_name = rows.column_name(idx).unwrap();
        let key = cx.string(column_name);
        let v: Handle<'_, JsValue> = match v {
            libsql::Value::Null => cx.null().upcast(),
            libsql::Value::Integer(v) => {
                if safe_ints {
                    neon::types::JsBigInt::from_i64(cx, v).upcast()
                } else {
                    cx.number(v as f64).upcast()
                }
            },
            libsql::Value::Real(v) => cx.number(v).upcast(),
            libsql::Value::Text(v) => cx.string(v).upcast(),
            libsql::Value::Blob(_v) => todo!("unsupported type"),
        };
        result.set(cx, key, v)?;
    }
    Ok(())
}

fn convert_row_raw(
    cx: &mut FunctionContext,
    safe_ints: bool,
    result: &mut JsArray,
    rows: &libsql::v2::Rows,
    row: &libsql::v2::Row,
) -> NeonResult<()> {
    for idx in 0..rows.column_count() {
        let v = row
            .get_value(idx)
            .or_else(|err| cx.throw_error(from_libsql_error(err)))?;
        let v: Handle<'_, JsValue> = match v {
            libsql::Value::Null => cx.null().upcast(),
            libsql::Value::Integer(v) => {
                if safe_ints {
                    neon::types::JsBigInt::from_i64(cx, v).upcast()
                } else {
                    cx.number(v as f64).upcast()
                }
            },
            libsql::Value::Real(v) => cx.number(v).upcast(),
            libsql::Value::Text(v) => cx.string(v).upcast(),
            libsql::Value::Blob(_v) => todo!("unsupported blob type"),
        };
        result.set(cx, idx as u32, v)?;
    }
    Ok(())
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("databaseOpen", Database::js_open)?;
    cx.export_function("databaseOpenWithRpcSync", Database::js_open_with_rpc_sync)?;
    cx.export_function("databaseInTransaction", Database::js_in_transaction)?;
    cx.export_function("databaseClose", Database::js_close)?;
    cx.export_function("databaseSync", Database::js_sync)?;
    cx.export_function("databaseExecSync", Database::js_exec_sync)?;
    cx.export_function("databaseExecAsync", Database::js_exec_async)?;
    cx.export_function("databasePrepareSync", Database::js_prepare_sync)?;
    cx.export_function("databasePrepareAsync", Database::js_prepare_async)?;
    cx.export_function("databaseDefaultSafeIntegers", Database::js_default_safe_integers)?;
    cx.export_function("statementRaw", Statement::js_raw)?;
    cx.export_function("statementRun", Statement::js_run)?;
    cx.export_function("statementGet", Statement::js_get)?;
    cx.export_function("statementRowsSync", Statement::js_rows_sync)?;
    cx.export_function("statementRowsAsync", Statement::js_rows_async)?;
    cx.export_function("statementColumns", Statement::js_columns)?;
    cx.export_function("statementSafeIntegers", Statement::js_safe_integers)?;
    cx.export_function("rowsNext", Rows::js_next)?;
    Ok(())
}
