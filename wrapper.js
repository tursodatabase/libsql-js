"use strict";

const { Database: NativeDb, databasePrepareSync, databaseSyncSync, databaseExecSync, statementIterateSync, iteratorNextSync } = require("./index.js");
const SqliteError = require("./sqlite-error.js");
const Authorization = require("./auth");

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

  authorizer(rules) {
    try {
      this.db.authorizer(rules);
    } catch (err) {
      throw convertError(err);
    }
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
  exec(sql) {
    try {
      databaseExecSync(this.db, sql);
    } catch (err) {
      throw convertError(err);
    }
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

  authorizer(hook) {
    this.db.authorizer(hook);
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
    throw new Error("not implemented");
  }

  /**
   * Executes the SQL statement and returns an info object.
   */
  run(...bindParameters) {
    try {
      return this.stmt.run(...bindParameters);
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
      return this.stmt.get(...bindParameters);
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
      const it = statementIterateSync(this.stmt, ...bindParameters);
      return {
        next: () => iteratorNextSync(it),
        [Symbol.iterator]() {
          return {
            next: () => iteratorNextSync(it),
          }
        },
      };
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
      let next;
      while (!(next = iterator.next()).done) {
        result.push(next.value);
      }
      return result;
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

module.exports = Database;
module.exports.SqliteError = SqliteError;
module.exports.Authorization = Authorization;
