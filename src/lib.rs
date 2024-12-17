mod database;
mod errors;
mod statement;

use crate::database::Database;
use crate::statement::{Rows, Statement};
use neon::prelude::*;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME
        .get_or_try_init(Runtime::new)
        .or_else(|err| cx.throw_error(&err.to_string()))
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    let _ = tracing_subscriber::fmt::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::ERROR.into())
                .from_env_lossy(),
        )
        .try_init();
    cx.export_function("databaseOpen", Database::js_open)?;
    cx.export_function("databaseOpenWithSync", Database::js_open_with_sync)?;
    cx.export_function("databaseInTransaction", Database::js_in_transaction)?;
    cx.export_function("databaseInterrupt", Database::js_interrupt)?;
    cx.export_function("databaseClose", Database::js_close)?;
    cx.export_function("databaseSyncSync", Database::js_sync_sync)?;
    cx.export_function("databaseSyncAsync", Database::js_sync_async)?;
    cx.export_function("databaseSyncUntilSync", Database::js_sync_until_sync)?;
    cx.export_function("databaseSyncUntilAsync", Database::js_sync_until_async)?;
    cx.export_function("databaseExecSync", Database::js_exec_sync)?;
    cx.export_function("databaseExecAsync", Database::js_exec_async)?;
    cx.export_function("databasePrepareSync", Database::js_prepare_sync)?;
    cx.export_function("databasePrepareAsync", Database::js_prepare_async)?;
    cx.export_function(
        "databaseDefaultSafeIntegers",
        Database::js_default_safe_integers,
    )?;
    cx.export_function("databaseLoadExtension", Database::js_load_extension)?;
    cx.export_function(
        "databaseMaxWriteReplicationIndex",
        Database::js_max_write_replication_index,
    )?;
    cx.export_function("statementRaw", Statement::js_raw)?;
    cx.export_function("statementIsReader", Statement::js_is_reader)?;
    cx.export_function("statementRun", Statement::js_run)?;
    cx.export_function("statementGet", Statement::js_get)?;
    cx.export_function("statementRowsSync", Statement::js_rows_sync)?;
    cx.export_function("statementRowsAsync", Statement::js_rows_async)?;
    cx.export_function("statementColumns", Statement::js_columns)?;
    cx.export_function("statementSafeIntegers", Statement::js_safe_integers)?;
    cx.export_function("rowsNext", Rows::js_next)?;
    Ok(())
}
