"use strict";

const { load, currentTarget } = require('@neon-rs/load');

// Static requires for bundlers.
if (0) { require('./.targets'); }

const { databaseNew, databaseExec, databasePrepare, statementGet } = load(__dirname) || require(`@libsql/experimental-${currentTarget()}`);

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

    /**
     * Executes a SQL statement.
     *
     * @param {string} sql - The SQL statement string to execute.
     */
    exec(sql) {
        databaseExec.call(this.db, sql);
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
     * Executes the SQL statement and returns the first row.
     *
     * @param bindParameters - The bind parameters for executing the statement.
     */
    get(...bindParameters) {
        return statementGet.call(this.stmt, ...bindParameters);
    }
}

module.exports = Database;
