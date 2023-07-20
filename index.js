"use strict";

const { load, currentTarget } = require('@neon-rs/load');

// Static requires for bundlers.
if (0) { require('./.targets'); }

const { databaseNew, databaseExec, databasePrepare, statementGet, statementRows, rowsNext } = load(__dirname) || require(`@libsql/experimental-${currentTarget()}`);

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
    constructor(path) {
        this.db = databaseNew(path);
        this.memory = false;
        this.readonly = false;
        this.name = "";
        this.open = true;
        this.inTransaction = false;
    }

    /**
     * Prepares a SQL statement for execution.
     *
     * @param {string} sql - The SQL statement string to prepare.
     */
    prepare(sql) {
        const stmt = databasePrepare.call(this.db, sql);
        return new Statement(stmt);
    }

    transaction(fn) {
        if (typeof fn !== 'function') throw new TypeError('Expected first argument to be a function');
        throw new Error("not implemented")
    }

    pragma(source, options) {
        throw new Error("not implemented")
    }

    backup(filename, options) {
        throw new Error("not implemented")
    }

    serialize(options) {
        throw new Error("not implemented")
    }

    function(name, options, fn) {
	// Apply defaults
	if (options == null) options = {};
	if (typeof options === 'function') { fn = options; options = {}; }

	// Validate arguments
	if (typeof name !== 'string') throw new TypeError('Expected first argument to be a string');
	if (typeof fn !== 'function') throw new TypeError('Expected last argument to be a function');
	if (typeof options !== 'object') throw new TypeError('Expected second argument to be an options object');
	if (!name) throw new TypeError('User-defined function name cannot be an empty string');

        throw new Error("not implemented")
    }

    aggregate(name, options) {
	// Validate arguments
	if (typeof name !== 'string') throw new TypeError('Expected first argument to be a string');
	if (typeof options !== 'object' || options === null) throw new TypeError('Expected second argument to be an options object');
	if (!name) throw new TypeError('User-defined function name cannot be an empty string');

        throw new Error("not implemented")
    }

    table(name, factory) {
	// Validate arguments
	if (typeof name !== 'string') throw new TypeError('Expected first argument to be a string');
	if (!name) throw new TypeError('Virtual table module name cannot be an empty string');

        throw new Error("not implemented")
    }

    loadExtension(...args) {
        throw new Error("not implemented")
    }

    /**
     * Executes a SQL statement.
     *
     * @param {string} sql - The SQL statement string to execute.
     */
    exec(sql) {
        databaseExec.call(this.db, sql);
    }

    close() {
        throw new Error("not implemented")
    }

    defaultSafeIntegers(...args) {
        throw new Error("not implemented")
    }

    unsafeMode(...args) {
        throw new Error("not implemented")
    }
}

/**
 * Statement represents a prepared SQL statement that can be executed.
 */
class Statement {
    constructor(stmt) {
        this.stmt = stmt;
    }

    raw(raw) {
        this.raw = raw;
        return this;
    }

    /**
     * Executes the SQL statement and returns the first row.
     *
     * @param bindParameters - The bind parameters for executing the statement.
     */
    get(...bindParameters) {
        return statementGet.call(this.stmt, ...bindParameters);
    }

    /**
     * Executes the SQL statement and returns an iterator to the resulting rows.
     *
     * @param bindParameters - The bind parameters for executing the statement.
     */
    iterate(...bindParameters) {
        const rows = statementRows.call(this.stmt, ...bindParameters);
        const iter = {
            next() {
                if (!rows) {
                    return { done: true };
                }
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

    all(...bindParameters) {
       const result = [];
       for (const row of this.iterate(...bindParameters)) {
          result.push(row);
       }
       return result;
    }
}

module.exports = Database;
