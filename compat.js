"use strict";

const { Database: NativeDb, databasePrepareSync, databaseSyncSync, databaseExecSync, statementRunSync, statementGetSync, statementIterateSync, iteratorNextSync } = require("./index.js");
const SqliteError = require("./sqlite-error.js");
const { Authorization, Action } = require("./auth");

function convertError(err) {
  // Handle errors from Rust with JSON-encoded message
  if (typeof err.message === 'string') {
    try {
      const data = JSON.parse(err.message);
      if (data && data.libsqlError) {
        if (data.code === "SQLITE_AUTH") {
          // For SQLITE_AUTH, preserve the JSON string for the test
          return new SqliteError(err.message, data.code, data.rawCode);
        } else if (data.code === "SQLITE_NOTOPEN") {
          // Convert SQLITE_NOTOPEN to TypeError with expected message
          return new TypeError("The database connection is not open");
        } else {
          // For all other errors, use the plain message string
          return new SqliteError(data.message, data.code, data.rawCode);
        }
      }
    } catch (_) {
      // Not JSON, ignore
    }
  }
  return err;
}

function isQueryOptions(value) {
  return value != null
    && typeof value === "object"
    && !Array.isArray(value)
    && Object.prototype.hasOwnProperty.call(value, "queryTimeout");
}

function splitBindParameters(bindParameters) {
  if (bindParameters.length === 0) {
    return { params: undefined, queryOptions: undefined };
  }
  if (bindParameters.length > 1 && isQueryOptions(bindParameters[bindParameters.length - 1])) {
    return {
      params: bindParameters.length === 2 ? bindParameters[0] : bindParameters.slice(0, -1),
      queryOptions: bindParameters[bindParameters.length - 1],
    };
  }
  return { params: bindParameters.length === 1 ? bindParameters[0] : bindParameters, queryOptions: undefined };
}

/**
 * Database represents a connection that can prepare and execute SQL statements.
 */
class Database {
  /**
   * Creates a new database connection. If the database file pointed to by `path` does not exists, it will be created.
   *
   * @constructor
   * @param {string} path - Path to the database file.
   */
  constructor(path, opts) {
    this.db = new NativeDb(path, opts);
    this.memory = this.db.memory
    const db = this.db;
    Object.defineProperties(this, {
      inTransaction: {
        get() {
          return db.inTransaction();
        }
      },
    });
  }

  sync() {
    try {
      const result = databaseSyncSync(this.db);
      return {
        frames_synced: result.frames_synced,
        replication_index: result.replication_index
      };
    } catch (err) {
      throw convertError(err);
    }
  }

  syncUntil(replicationIndex) {
    throw new Error("not implemented");
  }

  /**
   * Prepares a SQL statement for execution.
   *
   * @param {string} sql - The SQL statement string to prepare.
   */
  prepare(sql) {
    try {
      const stmt = databasePrepareSync(this.db, sql);
      return new Statement(stmt);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Returns a function that executes the given function in a transaction.
   *
   * @param {function} fn - The function to wrap in a transaction.
   */
  transaction(fn) {
    if (typeof fn !== "function")
      throw new TypeError("Expected first argument to be a function");

    const db = this;
    const wrapTxn = (mode) => {
      return (...bindParameters) => {
        db.exec("BEGIN " + mode);
        try {
          const result = fn(...bindParameters);
          db.exec("COMMIT");
          return result;
        } catch (err) {
          db.exec("ROLLBACK");
          throw err;
        }
      };
    };
    const properties = {
      default: { value: wrapTxn("") },
      deferred: { value: wrapTxn("DEFERRED") },
      immediate: { value: wrapTxn("IMMEDIATE") },
      exclusive: { value: wrapTxn("EXCLUSIVE") },
      database: { value: this, enumerable: true },
    };
    Object.defineProperties(properties.default.value, properties);
    Object.defineProperties(properties.deferred.value, properties);
    Object.defineProperties(properties.immediate.value, properties);
    Object.defineProperties(properties.exclusive.value, properties);
    return properties.default.value;
  }

  pragma(source, options) {
    if (options == null) options = {};
    if (typeof source !== 'string') throw new TypeError('Expected first argument to be a string');
    if (typeof options !== 'object') throw new TypeError('Expected second argument to be an options object');
    const simple = options['simple'];
    const stmt = this.prepare(`PRAGMA ${source}`, this, true);
    return simple ? stmt.pluck().get() : stmt.all();
  }

  backup(filename, options) {
    throw new Error("not implemented");
  }

  serialize(options) {
    throw new Error("not implemented");
  }

  function(name, options, fn) {
    throw new Error("not implemented");
  }

  aggregate(name, options) {
    throw new Error("not implemented");
  }

  table(name, factory) {
    throw new Error("not implemented");
  }

  loadExtension(...args) {
    try {
      this.db.loadExtension(...args);
    } catch (err) {
      throw convertError(err);
    }
  }

  maxWriteReplicationIndex() {
    try {
      return this.db.maxWriteReplicationIndex();
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Executes a SQL statement.
   *
   * @param {string} sql - The SQL statement string to execute.
   */
  exec(sql, queryOptions) {
    try {
      databaseExecSync(this.db, sql, queryOptions);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Executes a batch of SQL statements sequentially, returning one
   * result object per input statement.
   *
   * When `mode` is provided and the connection is not already inside a
   * transaction, the batch is wrapped in a `BEGIN <mode>` / `COMMIT`
   * transaction that is rolled back if any statement fails.
   *
   * @param {Array<string | { sql: string, args?: any[] | Record<string, any> }>} statements - The statements to execute.
   * @param {string | { mode?: string, raw?: boolean }} [options] - Optional
   *   transaction mode or batch options. When `mode` is provided and the
   *   connection is not already inside a transaction, the statements run inside
   *   a transaction. When `raw` is true, reader rows are returned as arrays.
   *
   * Batch mutation result sets intentionally expose `rowsAffected` only. Unlike
   * `Statement.run()`, they do not include `lastInsertRowid`.
   * @returns {Array<{ columns: string[], columnTypes: string[], rows: Array<Record<string, any> | any[]>, rowsAffected: number }>}
   */
  batch(statements, options) {
    if (!Array.isArray(statements)) {
      throw new TypeError("Expected first argument to be an array of statements");
    }

    const { mode, raw } = normalizeBatchOptions(options);
    const wrap = mode != null && !this.inTransaction;
    if (wrap) {
      this.exec(`BEGIN ${normalizeBatchMode(mode)}`);
    }

    const results = [];
    try {
      for (const statement of statements) {
        const sql = typeof statement === "string" ? statement : statement.sql;
        const args = typeof statement === "string" ? undefined : statement.args;

        const stmt = this.prepare(sql);
        const cols = stmt.columns();
        const columnNames = cols.map((c) => c.name);
        const columnTypes = cols.map((c) => c.type ?? "");

        if (columnNames.length > 0) {
          // Reader statement: collect the returned rows.
          if (raw) {
            stmt.raw(true);
          }
          const rows = args !== undefined ? stmt.all(args) : stmt.all();
          results.push(makeResultSet(columnNames, columnTypes, rows, 0));
        } else {
          // Mutating statement: report affected rows only; batch results do not
          // expose Statement.run()'s lastInsertRowid by design.
          const info = args !== undefined ? stmt.run(args) : stmt.run();
          results.push(makeResultSet(columnNames, columnTypes, [], info.changes));
        }
      }

      if (wrap) {
        this.exec("COMMIT");
      }
    } catch (err) {
      if (wrap) {
        try {
          this.exec("ROLLBACK");
        } catch (_) {
          // ignore rollback failures and surface the original error
        }
      }
      throw convertError(err);
    }
    return results;
  }

  /**
   * Interrupts the database connection.
   */
  interrupt() {
    this.db.interrupt();
  }

  /**
   * Closes the database connection.
   */
  close() {
    this.db.close();
  }

  authorizer(config) {
    try {
      this.db.authorizer(config);
    } catch (err) {
      throw convertError(err);
    }
    return this;
  }

  /**
   * Toggle 64-bit integer support.
   */
  defaultSafeIntegers(toggle) {
    this.db.defaultSafeIntegers(toggle);
    return this;
  }

  unsafeMode(...args) {
    throw new Error("not implemented");
  }
}

/**
 * Statement represents a prepared SQL statement that can be executed.
 */
class Statement {
  constructor(stmt) {
    this.stmt = stmt;
  }

  /**
   * Toggle raw mode.
   *
   * @param raw Enable or disable raw mode. If you don't pass the parameter, raw mode is enabled.
   */
  raw(raw) {
    this.stmt.raw(raw);
    return this;
  }

  /**
   * Toggle pluck mode.
   *
   * @param pluckMode Enable or disable pluck mode. If you don't pass the parameter, pluck mode is enabled.
   */
  pluck(pluckMode) {
    this.stmt.pluck(pluckMode);
    return this;
  }

  /**
   * Toggle query timing.
   *
   * @param timing Enable or disable query timing. If you don't pass the parameter, query timing is enabled.
   */
  timing(timingMode) {
    this.stmt.timing(timingMode);
    return this;
  }

  get reader() {
    return this.stmt.columns().length > 0;
  }

  /**
   * Executes the SQL statement and returns an info object.
   */
  run(...bindParameters) {
    try {
      const { params, queryOptions } = splitBindParameters(bindParameters);
      return statementRunSync(this.stmt, params, queryOptions);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Executes the SQL statement and returns the first row.
   *
   * @param bindParameters - The bind parameters for executing the statement.
   */
  get(...bindParameters) {
    try {
      const { params, queryOptions } = splitBindParameters(bindParameters);
      return statementGetSync(this.stmt, params, queryOptions);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Executes the SQL statement and returns an iterator to the resulting rows.
   *
   * @param bindParameters - The bind parameters for executing the statement.
   */
  iterate(...bindParameters) {
    try {
      const { params, queryOptions } = splitBindParameters(bindParameters);
      const it = statementIterateSync(this.stmt, params, queryOptions);
      return wrappedIter(it);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Executes the SQL statement and returns an array of the resulting rows.
   *
   * @param bindParameters - The bind parameters for executing the statement.
   */
  all(...bindParameters) {
    try {
      const result = [];
      const iterator = this.iterate(...bindParameters);
      try {
        let next;
        while (!(next = iterator.next()).done) {
          result.push(next.value);
        }
        return result;
      } finally {
        if (typeof iterator.return === "function") {
          iterator.return();
        }
      }
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Interrupts the statement.
   */
  interrupt() {
    this.stmt.interrupt();
    return this;
  }

  /**
   * Returns the columns in the result set returned by this prepared statement.
   */
  columns() {
    return this.stmt.columns();
  }

  /**
   * Toggle 64-bit integer support.
   */
  safeIntegers(toggle) {
    this.stmt.safeIntegers(toggle);
    return this;
  }
}

function wrappedIter(it) {
  return {
    next() {
      return iteratorNextSync(it);
    },
    return(value) {
      if (typeof it.close === "function") {
        it.close();
      }
      return {
        done: true,
        value,
      };
    },
    [Symbol.iterator]() {
      return this;
    },
  };
}

module.exports = Database;
module.exports.SqliteError = SqliteError;
module.exports.Authorization = Authorization;
module.exports.Action = Action;

function normalizeBatchMode(mode) {
  switch (String(mode).toLowerCase()) {
    case "write":
      return "IMMEDIATE";
    case "read":
      return "DEFERRED";
    case "deferred":
      return "DEFERRED";
    case "immediate":
      return "IMMEDIATE";
    case "exclusive":
      return "EXCLUSIVE";
    default:
      return String(mode).toUpperCase();
  }
}

function normalizeBatchOptions(options) {
  if (options != null && typeof options === "object") {
    return {
      mode: options.mode,
      raw: options.raw === true,
    };
  }
  return {
    mode: options,
    raw: false,
  };
}

function makeResultSet(columns, columnTypes, rows, rowsAffected) {
  return {
    columns,
    columnTypes,
    rows,
    rowsAffected,
  };
}
