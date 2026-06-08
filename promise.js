"use strict";

const { Database: NativeDb, connect: nativeConnect } = require("./index.js");
const SqliteError = require("./sqlite-error.js");
const { Authorization, Action } = require("./auth");

/**
 * @import {Options as NativeOptions, Statement as NativeStatement} from './index.js'
 */

/**
 * better-sqlite3-compatible types for db.transaction()
 * @typedef {(...params: any[]) => unknown} VariableArgFunction
 */
/**
 * @template {VariableArgFunction} F
 * @typedef {F extends (...args: infer A) => unknown ? A : never} ArgumentTypes
 */

/**
 * A single row of a `ResultSet`, supporting both positional and named access.
 * @typedef {{ length: number, [index: number]: any, [name: string]: any }} Row
 */

/**
 * The result of executing a single statement in a {@link Database#batch} call.
 * @typedef {Object} ResultSet
 * @property {string[]} columns - The column names of the result.
 * @property {string[]} columnTypes - The declared column types of the result.
 * @property {Row[]} rows - The rows returned by the statement.
 * @property {number} rowsAffected - The number of rows changed by the statement.
 * @property {bigint | undefined} lastInsertRowid - The rowid of the last inserted row, if any.
 * @property {() => any} toJSON - Returns a JSON-serializable representation of the result set.
 */

/**
 * A statement to execute as part of a {@link Database#batch} call.
 * @typedef {string | { sql: string, args?: any[] | Record<string, any> }} BatchStatement
 */

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

/**
 * Builds a libSQL-style Row from a positional value array and column names.
 *
 * The returned object supports both positional (`row[0]`) and named
 * (`row.name`) access, matching the `@libsql/client` `Row` shape.
 *
 * @param {string[]} columnNames - The column names.
 * @param {any[]} values - The positional row values.
 * @returns {Row}
 */
function makeBatchRow(columnNames, values) {
  const row = [...values];
  for (let i = 0; i < columnNames.length; i++) {
    const name = columnNames[i];
    if (!name || name === "length" || /^\d+$/.test(name)) {
      continue;
    }
    Object.defineProperty(row, name, {
      value: values[i],
      enumerable: false,
      writable: false,
      configurable: true,
    });
  }
  return row;
}

/**
 * Constructs a single `ResultSet` for one statement in a batch.
 *
 * @param {string[]} columns
 * @param {string[]} columnTypes
 * @param {Row[]} rows
 * @param {number} rowsAffected
 * @param {bigint | undefined} lastInsertRowid
 * @returns {ResultSet}
 */
function makeResultSet(columns, columnTypes, rows, rowsAffected, lastInsertRowid) {
  return {
    columns,
    columnTypes,
    rows,
    rowsAffected,
    lastInsertRowid,
    toJSON() {
      return {
        columns: this.columns,
        columnTypes: this.columnTypes,
        rows: this.rows,
        rowsAffected: this.rowsAffected,
        lastInsertRowid: this.lastInsertRowid?.toString(),
      };
    },
  };
}

/**
 * Normalizes a batch transaction mode into a SQLite `BEGIN` mode.
 *
 * Accepts the `@libsql/client` modes (`"write"`, `"read"`, `"deferred"`) as
 * well as the native SQLite modes (`"deferred"`, `"immediate"`, `"exclusive"`).
 *
 * @param {string} mode
 * @returns {string}
 */
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
  if (isQueryOptions(bindParameters[bindParameters.length - 1])) {
    if (bindParameters.length === 1) {
      return { params: undefined, queryOptions: bindParameters[0] };
    }
    return {
      params: bindParameters.length === 2 ? bindParameters[0] : bindParameters.slice(0, -1),
      queryOptions: bindParameters[bindParameters.length - 1],
    };
  }
  return { params: bindParameters.length === 1 ? bindParameters[0] : bindParameters, queryOptions: undefined };
}

/**
 * Creates a new database connection.
 *
 * @param {string} path - Path to the database file.
 * @param {NativeOptions} opts - Options.
 */
const connect = async (path, opts) => {
  const db = await nativeConnect(path, opts);
  return new Database(db);
};

/**
 * Database represents a connection that can prepare and execute SQL statements.
 */
class Database {
  /**
   * Creates a new database connection. If the database file pointed to by `path` does not exists, it will be created.
   *
   * @constructor
   * @param {NativeDb} db - Database object
   */
  constructor(db) {
    this.db = db;
    this.memory = this.db.memory

    /** @type boolean */
    this.inTransaction;

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
      return this.db.sync();
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
  async prepare(sql) {
    try {
      const stmt = await this.db.prepare(sql);
      return new Statement(stmt);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Returns a function that executes the given function in a transaction.
   *
   * @template {VariableArgFunction} F
   * @param {F} fn - The function to wrap in a transaction.
   * @returns {{
   *  (...bindParameters: ArgumentTypes<F>): Promise<ReturnType<F>>;
   *  default(...bindParameters: ArgumentTypes<F>): Promise<ReturnType<F>>;
   *  deferred(...bindParameters: ArgumentTypes<F>): Promise<ReturnType<F>>;
   *  immediate(...bindParameters: ArgumentTypes<F>): Promise<ReturnType<F>>;
   *  exclusive(...bindParameters: ArgumentTypes<F>): Promise<ReturnType<F>>;
   * }}
   */
  transaction(fn) {
    if (typeof fn !== "function")
      throw new TypeError("Expected first argument to be a function");

    const db = this;
    const wrapTxn = (mode) => {
      return async (...bindParameters) => {
        await db.exec("BEGIN " + mode);
        try {
          const result = await fn(...bindParameters);
          await db.exec("COMMIT");
          return result;
        } catch (err) {
          await db.exec("ROLLBACK");
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

  /**
   * Prepares the SQL and executes it as `Statement.run`, returning the run info.
   *
   * @param {string} sql - The SQL statement string.
   * @param {...any} bindParameters - Bind parameters, optionally followed by a query options object.
   */
  async run(sql, ...bindParameters) {
    const stmt = await this.prepare(sql);
    return await stmt.run(...bindParameters);
  }

  /**
   * Prepares the SQL and executes it as `Statement.get`, returning the first row.
   *
   * @param {string} sql - The SQL statement string.
   * @param {...any} bindParameters - Bind parameters, optionally followed by a query options object.
   */
  async get(sql, ...bindParameters) {
    const stmt = await this.prepare(sql);
    return await stmt.get(...bindParameters);
  }

  /**
   * Prepares the SQL and executes it as `Statement.all`, returning all rows.
   *
   * @param {string} sql - The SQL statement string.
   * @param {...any} bindParameters - Bind parameters, optionally followed by a query options object.
   */
  async all(sql, ...bindParameters) {
    const stmt = await this.prepare(sql);
    return await stmt.all(...bindParameters);
  }

  /**
   * Prepares the SQL and executes it as `Statement.iterate`, returning an async iterator over rows.
   *
   * @param {string} sql - The SQL statement string.
   * @param {...any} bindParameters - Bind parameters, optionally followed by a query options object.
   */
  async iterate(sql, ...bindParameters) {
    const stmt = await this.prepare(sql);
    return await stmt.iterate(...bindParameters);
  }

  /**
   * Execute a pragma statement
   * @param {string} source - The pragma statement to execute, without the `PRAGMA` prefix.
   * @param {object} [options] - Options object.
   * @param {boolean} [options.simple] - If true, return a single value for single-column results.
   */
  async pragma(source, options) {
    if (options == null) options = {};
    if (typeof source !== 'string') throw new TypeError('Expected first argument to be a string');
    if (typeof options !== 'object') throw new TypeError('Expected second argument to be an options object');
    const simple = options['simple'];
    const stmt = await this.prepare(`PRAGMA ${source}`, this, true);
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

  /**
   * Loads an extension into the database
   * @param {Parameters<NativeDb['loadExtension']>} args - Arguments to pass to the underlying loadExtension method
   */
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
  async exec(sql, queryOptions) {
    try {
      await this.db.exec(sql, queryOptions);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Executes a batch of SQL statements sequentially, returning one
   * {@link ResultSet} per input statement.
   *
   * When `mode` is provided and the connection is not already inside a
   * transaction, the batch is wrapped in a `BEGIN <mode>` / `COMMIT`
   * transaction that is rolled back if any statement fails.
   *
   * @param {BatchStatement[]} statements - The statements to execute.
   * @param {string} [mode] - Optional transaction mode (`"write"`, `"read"`,
   *   `"deferred"`, `"immediate"`, or `"exclusive"`). When omitted, the
   *   statements are executed without an enclosing transaction.
   * @returns {Promise<ResultSet[]>}
   */
  async batch(statements, mode) {
    if (!Array.isArray(statements)) {
      throw new TypeError("Expected first argument to be an array of statements");
    }

    const wrap = mode != null && !this.inTransaction;
    if (wrap) {
      await this.exec(`BEGIN ${normalizeBatchMode(mode)}`);
    }

    const results = [];
    try {
      // Seed the last inserted rowid so we can tell, per statement, whether it
      // performed an insert (SQLite only updates last_insert_rowid() on insert).
      const seed = await this.prepare("SELECT last_insert_rowid()");
      let prevRowid = await seed.pluck().get();

      for (const statement of statements) {
        const sql = typeof statement === "string" ? statement : statement.sql;
        const args = typeof statement === "string" ? undefined : statement.args;

        const stmt = await this.prepare(sql);
        const cols = stmt.columns();
        const columnNames = cols.map((c) => c.name);
        const columnTypes = cols.map((c) => c.type ?? "");

        if (columnNames.length > 0) {
          // Reader statement: collect the returned rows.
          stmt.raw(true);
          const raw = args !== undefined ? await stmt.all(args) : await stmt.all();
          const rows = raw.map((values) => makeBatchRow(columnNames, values));
          results.push(makeResultSet(columnNames, columnTypes, rows, 0, undefined));
        } else {
          // Mutating statement: report affected rows and any inserted rowid.
          const info = args !== undefined ? await stmt.run(args) : await stmt.run();
          const after = info.lastInsertRowid;
          const lastInsertRowid = after !== prevRowid ? BigInt(after) : undefined;
          prevRowid = after;
          results.push(makeResultSet(columnNames, columnTypes, [], info.changes, lastInsertRowid));
        }
      }

      if (wrap) {
        await this.exec("COMMIT");
      }
    } catch (err) {
      if (wrap) {
        try {
          await this.exec("ROLLBACK");
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
   * @param {boolean} [toggle] - Whether to use safe integers by default.
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
  /**
   * @param {NativeStatement} stmt
   */
  constructor(stmt) {
    this.stmt = stmt;
  }

  /**
   * Toggle raw mode.
   *
   * @param {boolean} [raw] - Enable or disable raw mode. If you don't pass the parameter, raw mode is enabled.
   */
  raw(raw) {
    this.stmt.raw(raw);
    return this;
  }

  /**
   * Toggle pluck mode.
   *
   * @param {boolean} [pluckMode] - Enable or disable pluck mode. If you don't pass the parameter, pluck mode is enabled.
   */
  pluck(pluckMode) {
    this.stmt.pluck(pluckMode);
    return this;
  }

  /**
   * Toggle query timing.
   *
   * @param {boolean} [timingMode] - Enable or disable query timing. If you don't pass the parameter, query timing is enabled.
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
  async run(...bindParameters) {
    try {
      const { params, queryOptions } = splitBindParameters(bindParameters);
      return await this.stmt.run(params, queryOptions);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Executes the SQL statement and returns the first row.
   *
   * @param bindParameters - The bind parameters for executing the statement.
   */
  async get(...bindParameters) {
    try {
      const { params, queryOptions } = splitBindParameters(bindParameters);
      return await this.stmt.get(params, queryOptions);
    } catch (err) {
      throw convertError(err);
    }
  }

  /**
   * Executes the SQL statement and returns an iterator to the resulting rows.
   *
   * @param bindParameters - The bind parameters for executing the statement.
   */
  async iterate(...bindParameters) {
    try {
      const { params, queryOptions } = splitBindParameters(bindParameters);
      const it = await this.stmt.iterate(params, queryOptions);
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
  async all(...bindParameters) {
    try {
      const result = [];
      const iterator = await this.iterate(...bindParameters);
      try {
        let next;
        while (!(next = await iterator.next()).done) {
          result.push(next.value);
        }
        return result;
      } finally {
        if (typeof iterator.return === "function") {
          await iterator.return();
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
      return it.next();
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
    [Symbol.asyncIterator]() {
      return this;
    }
  };
}

module.exports = {
  Action,
  Authorization,
  Database,
  SqliteError,
  Statement,
  connect,
};
