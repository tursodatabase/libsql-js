# class Database

The `Database` class represents a connection that can prepare and execute SQL statements.

## Methods

### new Database(path, [options]) ⇒ Database

Creates a new database connection.

| Param   | Type                | Description               |
| ------- | ------------------- | ------------------------- |
| path    | <code>string</code> | Path to the database file |
| options | <code>object</code> | Options.                  |

The `path` parameter points to the SQLite database file to open. If the file pointed to by `path` does not exists, it will be created.
To open an in-memory database, please pass `:memory:` as the `path` parameter.

You can use the `options` parameter to specify various options. Options supported by the parameter are:

- `syncUrl`: open the database as embedded replica synchronizing from the provided URL.
- `syncPeriod`: synchronize the database periodically every `syncPeriod` seconds.
- `authToken`: authentication token for the provider URL (optional).
- `timeout`: number of milliseconds to wait on locked database before returning `SQLITE_BUSY` error
- `defaultQueryTimeout`: default maximum number of milliseconds a query is allowed to run before being interrupted with `SQLITE_INTERRUPT` error

The function returns a `Database` object.

### prepare(sql) ⇒ Statement

Prepares a SQL statement for execution.

| Param  | Type                | Description                          |
| ------ | ------------------- | ------------------------------------ |
| sql    | <code>string</code> | The SQL statement string to prepare. |

The function returns a `Statement` object.

### transaction(function) ⇒ function

Returns a function that runs the given function in a transaction.

| Param    | Type                  | Description                           |
| -------- | --------------------- | ------------------------------------- |
| function | <code>function</code> | The function to run in a transaction. |

### pragma(string, [options]) ⇒ results

This function is currently not supported.

### backup(destination, [options]) ⇒ promise

This function is currently not supported.

### serialize([options]) ⇒ Buffer

This function is currently not supported.

### function(name, [options], function) ⇒ this

This function is currently not supported.

### aggregate(name, options) ⇒ this

This function is currently not supported.

### table(name, definition) ⇒ this

This function is currently not supported.

### authorizer(config) ⇒ this

Configure authorization rules for the database. Accepts three formats:

- **Legacy format** — a map from table name to `Authorization.ALLOW` or `Authorization.DENY`
- **Rule-based format** — an `AuthorizerConfig` object with ordered rules and pattern matching
- **`null`** — removes the authorizer entirely

#### Legacy format

A simple object mapping table names to `Authorization.ALLOW` (0) or `Authorization.DENY` (1).
Tables without an entry are denied by default.

```javascript
const { Authorization } = require('libsql');

db.authorizer({
  "users": Authorization.ALLOW,
  "secrets": Authorization.DENY,
});

// Access to "users" is allowed.
const stmt = db.prepare("SELECT * FROM users");

// Access to "secrets" throws SQLITE_AUTH.
const stmt = db.prepare("SELECT * FROM secrets"); // Error!
```

#### Rule-based format

An object with a `rules` array and an optional `defaultPolicy`. Rules are evaluated in order — **first match wins**. If no rule matches, `defaultPolicy` applies (defaults to `DENY`).

```javascript
const { Authorization, Action } = require('libsql');

db.authorizer({
  rules: [
    // Hide sensitive columns (returns NULL instead of the real value)
    { action: Action.READ, table: "users", column: "password_hash", policy: Authorization.IGNORE },
    { action: Action.READ, table: "users", column: "ssn", policy: Authorization.IGNORE },

    // Allow all reads
    { action: Action.READ, policy: Authorization.ALLOW },

    // Allow inserts on tables matching a glob pattern
    { action: Action.INSERT, table: { glob: "logs_*" }, policy: Authorization.ALLOW },

    // Deny DDL operations
    { action: [Action.CREATE_TABLE, Action.DROP_TABLE, Action.ALTER_TABLE], policy: Authorization.DENY },

    // Allow transactions and selects
    { action: Action.TRANSACTION, policy: Authorization.ALLOW },
    { action: Action.SELECT, policy: Authorization.ALLOW },
  ],
  defaultPolicy: Authorization.DENY,
});
```

#### AuthRule fields

| Field    | Type                                      | Description                                                          |
| -------- | ----------------------------------------- | -------------------------------------------------------------------- |
| action   | <code>number \| number[]</code>           | Action code(s) to match (from `Action`). Omit to match all actions.  |
| table    | <code>string \| { glob: string }</code>   | Table name pattern. Omit to match any table.                         |
| column   | <code>string \| { glob: string }</code>   | Column name pattern (relevant for READ/UPDATE). Omit to match any.   |
| entity   | <code>string \| { glob: string }</code>   | Entity name (index, trigger, view, pragma, function). Omit to match any. |
| policy   | <code>number</code>                       | `Authorization.ALLOW`, `Authorization.DENY`, or `Authorization.IGNORE`. |

#### Pattern matching

Pattern fields (`table`, `column`, `entity`) accept either:

- A **plain string** for exact matching: `"users"`
- An **object with a `glob` key** for glob matching: `{ glob: "logs_*" }`

Glob patterns support `*` (match any number of characters) and `?` (match exactly one character).

```javascript
// Exact match
{ action: Action.READ, table: "users", policy: Authorization.ALLOW }

// Glob: all tables starting with "logs_"
{ action: Action.READ, table: { glob: "logs_*" }, policy: Authorization.ALLOW }

// Glob: single-character wildcard
{ action: Action.READ, table: { glob: "t?_data" }, policy: Authorization.ALLOW }

// Glob: match all tables
{ action: Action.READ, table: { glob: "*" }, policy: Authorization.ALLOW }
```

#### Authorization values

| Value                      | Effect                                                                 |
| -------------------------- | ---------------------------------------------------------------------- |
| `Authorization.ALLOW` (0)  | Permit the operation.                                                  |
| `Authorization.DENY` (1)   | Reject the entire SQL statement with a `SQLITE_AUTH` error.            |
| `Authorization.IGNORE` (2) | For READ: return NULL instead of the column value. Otherwise: deny.    |

#### Removing the authorizer

Pass `null` to remove the authorizer and allow all operations:

```javascript
db.authorizer(null);
```

### loadExtension(path, [entryPoint]) ⇒ this

Loads a SQLite3 extension

### exec(sql[, queryOptions]) ⇒ this

Executes a SQL statement.

| Param  | Type                | Description                          |
| ------ | ------------------- | ------------------------------------ |
| sql    | <code>string</code> | The SQL statement string to execute. |
| queryOptions | <code>object</code> | Optional per-query overrides (for example, `{ queryTimeout: 100 }`). |

### interrupt() ⇒ this

Cancel ongoing operations and make them return at earliest opportunity.

**Note:** This is an extension in libSQL and not available in `better-sqlite3`.

### close() ⇒ this

Closes the database connection.

# class Statement

## Methods

### run([...bindParameters][, queryOptions]) ⇒ object

Executes the SQL statement and returns an info object.

| Param          | Type                          | Description                                      |
| -------------- | ----------------------------- | ------------------------------------------------ |
| bindParameters | <code>array of objects</code> | The bind parameters for executing the statement. |
| queryOptions   | <code>object</code>           | Optional per-query overrides (for example, `{ queryTimeout: 100 }`). |

The returned info object contains two properties: `changes` that describes the number of modified rows and `info.lastInsertRowid` that represents the `rowid` of the last inserted row.

### get([...bindParameters][, queryOptions]) ⇒ row

Executes the SQL statement and returns the first row.

| Param          | Type                          | Description                                      |
| -------------- | ----------------------------- | ------------------------------------------------ |
| bindParameters | <code>array of objects</code> | The bind parameters for executing the statement. |
| queryOptions   | <code>object</code>           | Optional per-query overrides (for example, `{ queryTimeout: 100 }`). |

### all([...bindParameters][, queryOptions]) ⇒ array of rows

Executes the SQL statement and returns an array of the resulting rows.

| Param          | Type                          | Description                                      |
| -------------- | ----------------------------- | ------------------------------------------------ |
| bindParameters | <code>array of objects</code> | The bind parameters for executing the statement. |
| queryOptions   | <code>object</code>           | Optional per-query overrides (for example, `{ queryTimeout: 100 }`). |

### iterate([...bindParameters][, queryOptions]) ⇒ iterator

Executes the SQL statement and returns an iterator to the resulting rows.

| Param          | Type                          | Description                                      |
| -------------- | ----------------------------- | ------------------------------------------------ |
| bindParameters | <code>array of objects</code> | The bind parameters for executing the statement. |
| queryOptions   | <code>object</code>           | Optional per-query overrides (for example, `{ queryTimeout: 100 }`). |

### pluck([toggleState]) ⇒ this

This function is currently not supported.

### expand([toggleState]) ⇒ this

This function is currently not supported.

### raw([rawMode]) ⇒ this

Toggle raw mode.

| Param   | Type                 | Description                                                                       |
| ------- | -------------------- | --------------------------------------------------------------------------------- |
| rawMode | <code>boolean</code> | Enable or disable raw mode. If you don't pass the parameter, raw mode is enabled. |

This function enables or disables raw mode. Prepared statements return objects by default, but if raw mode is enabled, the functions return arrays instead.

### timed([toggle]) ⇒ this

Toggle query duration timing.

### columns() ⇒ array of objects

Returns the columns in the result set returned by this prepared statement.

### reader ⇒ boolean

Returns `true` if the statement returns data (i.e., it is a `SELECT` statement or an `INSERT`/`UPDATE`/`DELETE` with a `RETURNING` clause), `false` otherwise.

### bind([...bindParameters]) ⇒ this

This function is currently not supported.
