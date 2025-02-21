use neon::types::buffer::TypedArray;
use neon::types::JsPromise;
use neon::{prelude::*, types::JsBigInt};
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::errors::throw_libsql_error;
use crate::runtime;

pub(crate) struct Statement {
    pub conn: Arc<Mutex<libsql::Connection>>,
    pub stmt: Arc<Mutex<libsql::Statement>>,
    pub raw: RefCell<bool>,
    pub safe_ints: RefCell<bool>,
}

impl Finalize for Statement {}

fn js_value_to_value(
    cx: &mut FunctionContext,
    v: Handle<'_, JsValue>,
) -> NeonResult<libsql::Value> {
    if v.is_a::<JsNull, _>(cx) || v.is_a::<JsUndefined, _>(cx) {
        Ok(libsql::Value::Null)
    } else if v.is_a::<JsBoolean, _>(cx) {
        todo!("bool");
    } else if v.is_a::<JsNumber, _>(cx) {
        let v = v.downcast_or_throw::<JsNumber, _>(cx)?;
        let v = v.value(cx);
        Ok(libsql::Value::Real(v))
    } else if v.is_a::<JsString, _>(cx) {
        let v = v.downcast_or_throw::<JsString, _>(cx)?;
        let v = v.value(cx);
        Ok(libsql::Value::Text(v))
    } else if v.is_a::<JsBigInt, _>(cx) {
        let v = v.downcast_or_throw::<JsBigInt, _>(cx)?;
        let v = v.to_i64(cx).or_throw(cx)?;
        Ok(libsql::Value::Integer(v))
    } else if v.is_a::<JsUint8Array, _>(cx) {
        let v = v.downcast_or_throw::<JsUint8Array, _>(cx)?;
        let v = v.as_slice(cx);
        Ok(libsql::Value::Blob(v.to_vec()))
    } else if v.is_a::<JsFloat32Array, _>(cx) {
        let v = v.downcast_or_throw::<JsFloat32Array, _>(cx)?;
        let v = v.buffer(cx);
        let v = v.as_slice(cx);
        Ok(libsql::Value::Blob(v.to_vec()))
    } else {
        cx.throw_error("SQLite3 can only bind numbers, strings, bigints, buffers, and null")
    }
}

impl Statement {
    pub fn js_raw(mut cx: FunctionContext) -> JsResult<JsNull> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let raw_stmt = stmt.stmt.blocking_lock();
        if raw_stmt.columns().is_empty() {
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

    pub fn js_is_reader(mut cx: FunctionContext) -> JsResult<JsBoolean> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let raw_stmt = stmt.stmt.blocking_lock();
        Ok(cx.boolean(!raw_stmt.columns().is_empty()))
    }

    pub fn js_run(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let raw_conn = stmt.conn.clone();
        let total_changes_before = raw_conn.blocking_lock().total_changes();
        let params = cx.argument::<JsValue>(0)?;
        let params = convert_params(&mut cx, &stmt, params)?;
        let mut raw_stmt = stmt.stmt.blocking_lock();
        raw_stmt.reset();
        let fut = raw_stmt.run(params);
        let rt = runtime(&mut cx)?;

        let initial = Instant::now();

        rt.block_on(fut)
            .or_else(|err| throw_libsql_error(&mut cx, err))?;

        let duration = Instant::now() - initial;

        let (changes, last_insert_rowid) = {
            let raw_conn = stmt.conn.clone();
            let raw_conn = raw_conn.blocking_lock();
            let changes = if raw_conn.total_changes() == total_changes_before {
                0
            } else {
                raw_conn.changes()
            };
            let last_insert_rowid = raw_conn.last_insert_rowid();
            (changes, last_insert_rowid)
        };

        let info = cx.empty_object();

        let changes = cx.number(changes as f64);
        info.set(&mut cx, "changes", changes)?;

        let duration = cx.number(duration.as_secs_f64() as f64);
        info.set(&mut cx, "duration", duration)?;

        let last_insert_row_id = cx.number(last_insert_rowid as f64);
        info.set(&mut cx, "lastInsertRowid", last_insert_row_id)?;

        Ok(info.upcast())
    }

    pub fn js_interrupt(mut cx: FunctionContext) -> JsResult<JsNull> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let mut raw_stmt = stmt.stmt.blocking_lock();
        raw_stmt.interrupt();
        Ok(cx.null())
    }

    pub fn js_get(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let params = cx.argument::<JsValue>(0)?;
        let params = convert_params(&mut cx, &stmt, params)?;
        let safe_ints = *stmt.safe_ints.borrow();
        let mut raw_stmt = stmt.stmt.blocking_lock();
        let fut = raw_stmt.query(params);
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(fut);
        let mut rows = result.or_else(|err| throw_libsql_error(&mut cx, err))?;

        let initial = Instant::now();

        let result = rt
            .block_on(rows.next())
            .or_else(|err| throw_libsql_error(&mut cx, err))?;

        let duration = Instant::now() - initial;

        let result = match result {
            Some(row) => {
                if *stmt.raw.borrow() {
                    let mut result = cx.empty_array();
                    convert_row_raw(&mut cx, safe_ints, &mut result, &rows, &row)?;
                    Ok(result.upcast())
                } else {
                    let mut result = cx.empty_object();
                    convert_row(&mut cx, safe_ints, &mut result, &rows, &row)?;

                    let metadata = cx.empty_object();
                    result.set(&mut cx, "_metadata", metadata)?;

                    let duration = cx.number(duration.as_secs_f64());
                    metadata.set(&mut cx, "duration", duration)?;

                    Ok(result.upcast())
                }
            }
            None => Ok(cx.undefined().upcast()),
        };
        raw_stmt.reset();
        result
    }

    pub fn js_rows_sync(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let params = cx.argument::<JsValue>(0)?;
        let params = convert_params(&mut cx, &stmt, params)?;
        let rt = runtime(&mut cx)?;
        let result = rt.block_on(async move {
            let mut raw_stmt = stmt.stmt.lock().await;
            raw_stmt.reset();
            raw_stmt.query(params).await
        });
        let rows = result.or_else(|err| throw_libsql_error(&mut cx, err))?;
        let rows = Rows {
            rows: RefCell::new(rows),
            raw: *stmt.raw.borrow(),
            safe_ints: *stmt.safe_ints.borrow(),
        };
        Ok(cx.boxed(rows).upcast())
    }

    pub fn js_rows_async(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let params = cx.argument::<JsValue>(0)?;
        let params = convert_params(&mut cx, &stmt, params)?;
        {
            let mut raw_stmt = stmt.stmt.blocking_lock();
            raw_stmt.reset();
        }
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let rt = runtime(&mut cx)?;
        let raw = *stmt.raw.borrow();
        let safe_ints = *stmt.safe_ints.borrow();
        let raw_stmt = stmt.stmt.clone();
        rt.spawn(async move {
            let result = {
                let mut raw_stmt = raw_stmt.lock().await;
                raw_stmt.query(params).await
            };
            match result {
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
                        throw_libsql_error(&mut cx, err)?;
                        Ok(cx.undefined())
                    });
                }
            }
        });
        Ok(promise)
    }

    pub fn js_columns(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt: Handle<'_, JsBox<Statement>> = cx.this()?;
        let result = cx.empty_array();
        let raw_stmt = stmt.stmt.blocking_lock();
        for (i, col) in raw_stmt.columns().iter().enumerate() {
            let column = cx.empty_object();
            let column_name = cx.string(col.name());
            column.set(&mut cx, "name", column_name)?;
            let column_origin_name: Handle<'_, JsValue> =
                if let Some(origin_name) = col.origin_name() {
                    cx.string(origin_name).upcast()
                } else {
                    cx.null().upcast()
                };
            column.set(&mut cx, "column", column_origin_name)?;
            let column_table_name: Handle<'_, JsValue> = if let Some(table_name) = col.table_name()
            {
                cx.string(table_name).upcast()
            } else {
                cx.null().upcast()
            };
            column.set(&mut cx, "table", column_table_name)?;
            let column_database_name: Handle<'_, JsValue> =
                if let Some(database_name) = col.database_name() {
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

    pub fn js_safe_integers(mut cx: FunctionContext) -> JsResult<JsNull> {
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

pub(crate) struct Rows {
    rows: RefCell<libsql::Rows>,
    raw: bool,
    safe_ints: bool,
}

impl Finalize for Rows {}

impl Rows {
    pub fn js_next(mut cx: FunctionContext) -> JsResult<JsNull> {
        let result_arr = cx.argument::<JsArray>(0)?;
        let rows: Handle<'_, JsBox<Rows>> = cx.this()?;
        let raw = rows.raw;
        let safe_ints = rows.safe_ints;
        let mut rows = rows.rows.borrow_mut();
        let rt = runtime(&mut cx)?;
        let count = result_arr.len(&mut cx);
        let res = cx.null();
        rt.block_on(async move {
            let mut keys = Vec::<Handle<JsString>>::with_capacity(rows.column_count() as usize);
            for idx in 0..rows.column_count() {
                let column_name = rows.column_name(idx).unwrap();
                keys.push(cx.string(column_name));
            }
            for idx in 0..count {
                match rows
                    .next()
                    .await
                    .or_else(|err| throw_libsql_error(&mut cx, err))?
                {
                    Some(row) => {
                        if raw {
                            let mut result = cx.empty_array();
                            convert_row_raw(&mut cx, safe_ints, &mut result, &rows, &row)?;
                            result_arr.set(&mut cx, idx, result)?;
                        } else {
                            let result = cx.empty_object();
                            for idx in 0..rows.column_count() {
                                let v = row
                                    .get_value(idx)
                                    .or_else(|err| throw_libsql_error(&mut cx, err))?;
                                let v: Handle<'_, JsValue> = match v {
                                    libsql::Value::Null => cx.null().upcast(),
                                    libsql::Value::Integer(v) => {
                                        if safe_ints {
                                            neon::types::JsBigInt::from_i64(&mut cx, v).upcast()
                                        } else {
                                            cx.number(v as f64).upcast()
                                        }
                                    }
                                    libsql::Value::Real(v) => cx.number(v).upcast(),
                                    libsql::Value::Text(v) => cx.string(v).upcast(),
                                    libsql::Value::Blob(v) => {
                                        JsArrayBuffer::from_slice(&mut cx, &v)?.upcast()
                                    }
                                };
                                result.set(&mut cx, keys[idx as usize], v)?;
                            }
                            result_arr.set(&mut cx, idx, result)?;
                        }
                    }
                    None => {
                        break;
                    }
                };
            }
            Ok(())
        })?;
        Ok(res)
    }
}

fn convert_params(
    cx: &mut FunctionContext,
    stmt: &Statement,
    v: Handle<'_, JsValue>,
) -> NeonResult<libsql::params::Params> {
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
) -> NeonResult<libsql::params::Params> {
    let mut params = vec![];
    for i in 0..v.len(cx) {
        let v = v.get(cx, i)?;
        let v = js_value_to_value(cx, v)?;
        params.push(v);
    }
    Ok(libsql::params::Params::Positional(params))
}

fn convert_params_object(
    cx: &mut FunctionContext,
    stmt: &Statement,
    v: Handle<'_, JsObject>,
) -> NeonResult<libsql::params::Params> {
    let mut params = vec![];
    let stmt = &stmt.stmt;
    let raw_stmt = stmt.blocking_lock();
    for idx in 0..raw_stmt.parameter_count() {
        let name = raw_stmt.parameter_name((idx + 1) as i32).unwrap();
        let name = name.to_string();
        let v = v.get(cx, &name[1..])?;
        let v = js_value_to_value(cx, v)?;
        params.push((name, v));
    }
    Ok(libsql::params::Params::Named(params))
}

fn convert_row(
    cx: &mut FunctionContext,
    safe_ints: bool,
    result: &mut JsObject,
    rows: &libsql::Rows,
    row: &libsql::Row,
) -> NeonResult<()> {
    for idx in 0..rows.column_count() {
        let v = row
            .get_value(idx)
            .or_else(|err| throw_libsql_error(cx, err))?;
        let column_name = rows.column_name(idx).unwrap();
        let v: Handle<'_, JsValue> = match v {
            libsql::Value::Null => cx.null().upcast(),
            libsql::Value::Integer(v) => {
                if safe_ints {
                    neon::types::JsBigInt::from_i64(cx, v).upcast()
                } else {
                    cx.number(v as f64).upcast()
                }
            }
            libsql::Value::Real(v) => cx.number(v).upcast(),
            libsql::Value::Text(v) => cx.string(v).upcast(),
            libsql::Value::Blob(v) => JsBuffer::from_slice(cx, &v)?.upcast(),
        };
        result.set(cx, column_name, v)?;
    }
    Ok(())
}

fn convert_row_raw(
    cx: &mut FunctionContext,
    safe_ints: bool,
    result: &mut JsArray,
    rows: &libsql::Rows,
    row: &libsql::Row,
) -> NeonResult<()> {
    for idx in 0..rows.column_count() {
        let v = row
            .get_value(idx)
            .or_else(|err| throw_libsql_error(cx, err))?;
        let v: Handle<'_, JsValue> = match v {
            libsql::Value::Null => cx.null().upcast(),
            libsql::Value::Integer(v) => {
                if safe_ints {
                    neon::types::JsBigInt::from_i64(cx, v).upcast()
                } else {
                    cx.number(v as f64).upcast()
                }
            }
            libsql::Value::Real(v) => cx.number(v).upcast(),
            libsql::Value::Text(v) => cx.string(v).upcast(),
            libsql::Value::Blob(v) => JsBuffer::from_slice(cx, &v)?.upcast(),
        };
        result.set(cx, idx as u32, v)?;
    }
    Ok(())
}
