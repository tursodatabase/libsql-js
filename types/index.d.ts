export = Database;
/**
 * Database represents a connection that can prepare and execute SQL statements.
 */
declare class Database {
    /**
     * Creates a new database connection. If the database file pointed to by `path` does not exists, it will be created.
     *
     * @constructor
     * @param {string} path - Path to the database file.
     */
    constructor(path: string, opts: any);
    db: any;
    memory: boolean;
    readonly: boolean;
    name: string;
    open: boolean;
    sync(): any;
    /**
     * Prepares a SQL statement for execution.
     *
     * @param {string} sql - The SQL statement string to prepare.
     */
    prepare(sql: string): Statement;
    /**
     * Returns a function that executes the given function in a transaction.
     *
     * @param {function} fn - The function to wrap in a transaction.
     */
    transaction(fn: Function): (...bindParameters: any[]) => any;
    pragma(source: any, options: any): any;
    backup(filename: any, options: any): void;
    serialize(options: any): void;
    function(name: any, options: any, fn: any): void;
    aggregate(name: any, options: any): void;
    table(name: any, factory: any): void;
    loadExtension(...args: any[]): void;
    /**
     * Executes a SQL statement.
     *
     * @param {string} sql - The SQL statement string to execute.
     */
    exec(sql: string): void;
    /**
     * Closes the database connection.
     */
    close(): void;
    /**
     * Toggle 64-bit integer support.
     */
    defaultSafeIntegers(toggle: any): this;
    unsafeMode(...args: any[]): void;
}
declare namespace Database {
    export { SqliteError };
}
/**
 * Statement represents a prepared SQL statement that can be executed.
 */
declare class Statement {
    constructor(stmt: any);
    stmt: any;
    /**
     * Toggle raw mode.
     *
     * @param raw Enable or disable raw mode. If you don't pass the parameter, raw mode is enabled.
     */
    raw(raw: any): this;
    get reader(): any;
    /**
     * Executes the SQL statement and returns an info object.
     */
    run(...bindParameters: any[]): any;
    /**
     * Executes the SQL statement and returns the first row.
     *
     * @param bindParameters - The bind parameters for executing the statement.
     */
    get(...bindParameters: any[]): any;
    /**
     * Executes the SQL statement and returns an iterator to the resulting rows.
     *
     * @param bindParameters - The bind parameters for executing the statement.
     */
    iterate(...bindParameters: any[]): {
        [x: number]: () => any;
        nextRows: any[];
        nextRowIndex: number;
        next(): {
            done: boolean;
            value?: undefined;
        } | {
            value: any;
            done: boolean;
        };
    };
    /**
     * Executes the SQL statement and returns an array of the resulting rows.
     *
     * @param bindParameters - The bind parameters for executing the statement.
     */
    all(...bindParameters: any[]): any[];
    /**
     * Returns the columns in the result set returned by this prepared statement.
     */
    columns(): any;
    /**
     * Toggle 64-bit integer support.
     */
    safeIntegers(toggle: any): this;
}
import SqliteError = require("./sqlite-error");
//# sourceMappingURL=index.d.ts.map