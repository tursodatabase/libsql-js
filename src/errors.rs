use neon::{context::Context, object::Object, result::NeonResult, types::JsError};

pub fn throw_database_closed_error<'a, C: Context<'a>, T>(cx: &mut C) -> NeonResult<T> {
    let err = JsError::type_error(cx, "The database connection is not open")?;
    cx.throw(err)
}

pub fn throw_libsql_error<'a, C: Context<'a>, T>(cx: &mut C, err: libsql::Error) -> NeonResult<T> {
    match err {
        libsql::Error::SqliteFailure(code, err) => {
            let err = err.to_string();
            let err = JsError::error(cx, err).unwrap();
            let code_num = cx.number(code);
            err.set(cx, "rawCode", code_num).unwrap();
            let code = cx.string(convert_sqlite_code(code));
            err.set(cx, "code", code).unwrap();
            let val = cx.boolean(true);
            err.set(cx, "libsqlError", val).unwrap();
            cx.throw(err)?
        }
        _ => {
            let err = err.to_string();
            let err = JsError::error(cx, err).unwrap();
            let code = cx.string("");
            err.set(cx, "code", code).unwrap();
            cx.throw(err)?
        }
    }
}

fn convert_sqlite_code(code: i32) -> String {
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
