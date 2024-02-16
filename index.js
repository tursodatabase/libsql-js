"use strict";

const { load, currentTarget } = require("@neon-rs/load");
const { familySync, GLIBC } = require("detect-libc");

// Static requires for bundlers.
if (0) {
  require("./.targets");
}

let target = currentTarget();

// Workaround for Bun, which reports a musl target, but really wants glibc...
if (familySync() == GLIBC) {
  switch (target) {
  case "linux-x64-musl":
    target = "linux-x64-gnu";
    break;
  case "linux-arm64-musl":
    target = "linux-arm64-gnu";
    break;
  }
}

const {
  databaseOpen,
  databaseOpenWithRpcSync,
  databaseInTransaction,
  databaseClose,
  databaseSyncSync,
  databaseExecSync,
  databasePrepareSync,
  databaseDefaultSafeIntegers,
  statementRaw,
  statementGet,
  statementRun,
  statementRowsSync,
  statementColumns,
  statementSafeIntegers,
  rowsNext,
} = load(__dirname) || require(`@libsql/${target}`);

const SqliteError = require("./sqlite-error");

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
    const encryptionCipher = opts?.encryptionCipher ?? "sqlcipher";
    if (opts && opts.syncUrl) {
      var authToken = "";
      if (opts.syncAuth) {
          console.warn("Warning: The `syncAuth` option is deprecated, please use `authToken` option instead.");
          authToken = opts.syncAuth;
      } else if (opts.authToken) {
          authToken = opts.authToken;
      }
      const encryptionKey = opts?.encryptionKey ?? "";
      this.db = databaseOpenWithRpcSync(path, opts.syncUrl, authToken, encryptionCipher, encryptionKey);
    } else {
      const authToken = opts?.authToken ?? "";
      const encryptionKey = opts?.encryptionKey ?? "";
      this.db = databaseOpen(path, authToken, encryptionCipher, encryptionKey);
    }
    // TODO: Use a libSQL API for this?
    this.memory = path === ":memory:";
    this.readonly = false;
    this.name = "";
    this.open = true;

    const db = this.db;
    Object.defineProperties(this, {
      inTransaction: {
        get() {
          return databaseInTransaction(db);
        }
      },
    });
  }

  sync() {
    databaseSyncSync.call(this.db);
  }

  /**
   * Prepares a SQL statement for execution.
   *
   * @param {string} sql - The SQL statement string to prepare.
   */
  prepare(sql) {
    try {
      const stmt = databasePrepareSync.call(this.db, sql);
      return new Statement(stmt);  
    } catch (err) {
      throw new SqliteError(err.message, err.code, err.rawCode);
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

    return (...bindParameters) => {
      // TODO: Use libsql transaction API.
      this.exec("BEGIN");
      try {
        const result = fn(...bindParameters);
        this.exec("COMMIT");
        return result;
      } catch (err) {
        this.exec("ROLLBACK");
        throw err;
      }
    };
  }

  pragma(source, options) {
    throw new Error("not implemented");
  }

  backup(filename, options) {
    throw new Error("not implemented");
  }

  serialize(options) {
    throw new Error("not implemented");
  }

  function(name, options, fn) {
    // Apply defaults
    if (options == null) options = {};
    if (typeof options === "function") {
      fn = options;
      options = {};
    }

    // Validate arguments
    if (typeof name !== "string")
      throw new TypeError("Expected first argument to be a string");
    if (typeof fn !== "function")
      throw new TypeError("Expected last argument to be a function");
    if (typeof options !== "object")
      throw new TypeError("Expected second argument to be an options object");
    if (!name)
      throw new TypeError(
        "User-defined function name cannot be an empty string"
      );

    throw new Error("not implemented");
  }

  aggregate(name, options) {
    // Validate arguments
    if (typeof name !== "string")
      throw new TypeError("Expected first argument to be a string");
    if (typeof options !== "object" || options === null)
      throw new TypeError("Expected second argument to be an options object");
    if (!name)
      throw new TypeError(
        "User-defined function name cannot be an empty string"
      );

    throw new Error("not implemented");
  }

  table(name, factory) {
    // Validate arguments
    if (typeof name !== "string")
      throw new TypeError("Expected first argument to be a string");
    if (!name)
      throw new TypeError(
        "Virtual table module name cannot be an empty string"
      );

    throw new Error("not implemented");
  }

  loadExtension(...args) {
    throw new Error("not implemented");
  }

  /**
   * Executes a SQL statement.
   *
   * @param {string} sql - The SQL statement string to execute.
   */
  exec(sql) {
    try {
      databaseExecSync.call(this.db, sql);
    } catch (err) {
      throw new SqliteError(err.message, err.code, err.rawCode);
    }
  }

  /**
   * Closes the database connection.
   */
  close() {
    databaseClose.call(this.db);
    this.open = false;
  }

  /**
   * Toggle 64-bit integer support.
   */
  defaultSafeIntegers(toggle) {
    databaseDefaultSafeIntegers.call(this.db, toggle ?? true);
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
    statementRaw.call(this.stmt, raw ?? true);
    return this;
  }

  /**
   * Executes the SQL statement and returns an info object.
   */
  run(...bindParameters) {
    try {
      if (bindParameters.length == 1 && typeof bindParameters[0] === "object") {
        return statementRun.call(this.stmt, bindParameters[0]);
      } else {
        return statementRun.call(this.stmt, bindParameters.flat());
      }
    } catch (err) {
      throw new SqliteError(err.message, err.code, err.rawCode);
    }
  }

  /**
   * Executes the SQL statement and returns the first row.
   *
   * @param bindParameters - The bind parameters for executing the statement.
   */
  get(...bindParameters) {
    if (bindParameters.length == 1 && typeof bindParameters[0] === "object") {
      return statementGet.call(this.stmt, bindParameters[0]);
    } else {
      return statementGet.call(this.stmt, bindParameters.flat());
    }
  }

  /**
   * Executes the SQL statement and returns an iterator to the resulting rows.
   *
   * @param bindParameters - The bind parameters for executing the statement.
   */
  iterate(...bindParameters) {
    var rows = undefined;
    if (bindParameters.length == 1 && typeof bindParameters[0] === "object") {
      rows = statementRowsSync.call(this.stmt, bindParameters[0]);
    } else {
      rows = statementRowsSync.call(this.stmt, bindParameters.flat());
    }
    const iter = {
      next() {
        const row = rowsNext.call(rows);
        if (!row) {
          return { done: true };
        }
        return { value: row, done: false };
      },
      [Symbol.iterator]() {
        return this;
      },
    };
    return iter;
  }

  /**
   * Executes the SQL statement and returns an array of the resulting rows.
   *
   * @param bindParameters - The bind parameters for executing the statement.
   */
  all(...bindParameters) {
    const result = [];
    for (const row of this.iterate(...bindParameters)) {
      result.push(row);
    }
    return result;
  }

  /**
   * Returns the columns in the result set returned by this prepared statement.
   */
  columns() {
    return statementColumns.call(this.stmt);
  }

  /**
   * Toggle 64-bit integer support.
   */
  safeIntegers(toggle) {
    statementSafeIntegers.call(this.stmt, toggle ?? true);
    return this;
  }
}

module.exports = Database;
module.exports.SqliteError = SqliteError;
