use neon::prelude::*;

struct Database {
    _db: libsql::Database,
    conn: libsql::Connection,
}

impl Finalize for Database {}

impl Database {
    fn new(db: libsql::Database, conn: libsql::Connection) -> Self {
        Database { _db: db, conn }
    }

    fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<Database>> {
        let dbpath = cx.argument::<JsString>(0)?.value(&mut cx);
        let db = libsql::Database::open(dbpath).unwrap();
        let conn = db.connect().unwrap();
        let db = Database::new(db, conn);
        Ok(cx.boxed(db))
    }

    fn js_exec(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let db = cx.this().downcast_or_throw::<JsBox<Database>, _>(&mut cx)?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        if let Err(err) = db.conn.execute(sql, ()) {
            let err = map_err(err);
            let err = cx.error(err)?;
            return cx.throw(err);
        }
        Ok(cx.undefined())
    }

    fn js_prepare(mut cx: FunctionContext) -> JsResult<JsBox<Statement>> {
        let db = cx.this().downcast_or_throw::<JsBox<Database>, _>(&mut cx)?;
        let sql = cx.argument::<JsString>(0)?.value(&mut cx);
        let stmt = db.conn.prepare(sql).unwrap();
        let stmt = Statement { stmt };
        Ok(cx.boxed(stmt))
    }
}

fn map_err(err: libsql::Error) -> String {
    match err {
        libsql::Error::PrepareFailed(_, err) => err,
        _ => {
            todo!();
        }
    }
}

struct Statement {
    stmt: libsql::Statement,
}

impl Finalize for Statement {}

fn js_value_to_value(cx: &mut FunctionContext, v: Handle<'_, JsValue>) -> libsql::Value {
    if v.is_a::<JsNull, _>(cx) {
        todo!("null");
    } else if v.is_a::<JsUndefined, _>(cx) {
        todo!("undefined");
    } else if v.is_a::<JsArray, _>(cx) {
        todo!("array");
    } else if v.is_a::<JsBoolean, _>(cx) {
        todo!("bool");
    } else if v.is_a::<JsNumber, _>(cx) {
        let v = v.downcast_or_throw::<JsNumber, _>(cx).unwrap();
        let v = v.value(cx);
        libsql::Value::Integer(v as i64)
    } else if v.is_a::<JsString, _>(cx) {
        let v = v.downcast_or_throw::<JsString, _>(cx).unwrap();
        let v = v.value(cx);
        libsql::Value::Text(v)
    } else {
        todo!("unsupported type");
    }
}

impl Statement {
    fn js_get(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt = cx
            .this()
            .downcast_or_throw::<JsBox<Statement>, _>(&mut cx)?;
        let mut params = vec![];
        for i in 0..cx.len() {
            let v = cx.argument::<JsValue>(i)?;
            let v = js_value_to_value(&mut cx, v);
            params.push(v);
        }
        let params = libsql::Params::Positional(params);
        stmt.stmt.reset();

        match stmt.stmt.execute(&params) {
            Some(rows) => {
                let row = rows.next().unwrap().unwrap();
                let mut result = cx.empty_object();
                convert_row(&mut cx, &mut result, &rows, &row);
                Ok(result.upcast())
            }
            None => Ok(cx.undefined().upcast()),
        }
    }

    fn js_rows(mut cx: FunctionContext) -> JsResult<JsValue> {
        let stmt = cx
            .this()
            .downcast_or_throw::<JsBox<Statement>, _>(&mut cx)?;
        let mut params = vec![];
        for i in 0..cx.len() {
            let v = cx.argument::<JsValue>(i)?;
            let v = js_value_to_value(&mut cx, v);
            params.push(v);
        }
        let params = libsql::Params::Positional(params);
        stmt.stmt.reset();
        match stmt.stmt.execute(&params) {
            Some(rows) => {
                let rows = Rows { rows };
                Ok(cx.boxed(rows).upcast())
            }
            None => Ok(cx.null().upcast()),
        }
    }
}

struct Rows {
    rows: libsql::Rows,
}

impl Finalize for Rows {}

impl Rows {
    fn js_next(mut cx: FunctionContext) -> JsResult<JsValue> {
        let rows = cx.this().downcast_or_throw::<JsBox<Rows>, _>(&mut cx)?;
        match rows.rows.next().unwrap() {
            Some(row) => {
                let mut result = cx.empty_object();
                convert_row(&mut cx, &mut result, &rows.rows, &row);
                Ok(result.upcast())
            }
            None => Ok(cx.undefined().upcast()),
        }
    }
}

fn convert_row(
    cx: &mut FunctionContext,
    result: &mut JsObject,
    rows: &libsql::rows::Rows,
    row: &libsql::rows::Row,
) {
    for idx in 0..rows.column_count() {
        let v = row.get_value(idx).unwrap();
        let column_name = rows.column_name(idx);
        let key = cx.string(column_name);
        let v: Handle<'_, JsValue> = match v {
            libsql::Value::Null => cx.null().upcast(),
            libsql::Value::Integer(v) => cx.number(v as f64).upcast(),
            libsql::Value::Float(v) => cx.number(v).upcast(),
            libsql::Value::Text(v) => cx.string(v).upcast(),
            libsql::Value::Blob(_v) => todo!("unsupported type"),
        };
        result.set(cx, key, v).unwrap();
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("databaseNew", Database::js_new)?;
    cx.export_function("databaseExec", Database::js_exec)?;
    cx.export_function("databasePrepare", Database::js_prepare)?;
    cx.export_function("statementGet", Statement::js_get)?;
    cx.export_function("statementRows", Statement::js_rows)?;
    cx.export_function("rowsNext", Rows::js_next)?;
    Ok(())
}
