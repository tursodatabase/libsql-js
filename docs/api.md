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

### authorizer(rules) ⇒ this

Configure authorization rules. The `rules` object is a map from table name to
`Authorization` object, which defines if access to table is allowed or denied.
If a table has no authorization rule, access to it is _denied_ by default.

Example:

```javascript
db.authorizer({
  "users": Authorization.ALLOW
});

// Access is allowed.
const stmt = db.prepare("SELECT * FROM users");

db.authorizer({
  "users": Authorization.DENY
});

// Access is denied.
const stmt = db.prepare("SELECT * FROM users");
```

**Note: This is an experimental API and, therefore, subject to change.**

### loadExtension(path, [entryPoint]) ⇒ this

Loads a SQLite3 extension

### exec(sql) ⇒ this

Executes a SQL statement.

| Param  | Type                | Description                          |
| ------ | ------------------- | ------------------------------------ |
| sql    | <code>string</code> | The SQL statement string to execute. |

### interrupt() ⇒ this

Cancel ongoing operations and make them return at earliest opportunity.

**Note:** This is an extension in libSQL and not available in `better-sqlite3`.

### close() ⇒ this

Closes the database connection.

# class Statement

## Methods

### run([...bindParameters]) ⇒ object

Executes the SQL statement and returns an info object.

| Param          | Type                          | Description                                      |
| -------------- | ----------------------------- | ------------------------------------------------ |
| bindParameters | <code>array of objects</code> | The bind parameters for executing the statement. |

The returned info object contains two properties: `changes` that describes the number of modified rows and `info.lastInsertRowid` that represents the `rowid` of the last inserted row.

### get([...bindParameters]) ⇒ row

Executes the SQL statement and returns the first row.

| Param          | Type                          | Description                                      |
| -------------- | ----------------------------- | ------------------------------------------------ |
| bindParameters | <code>array of objects</code> | The bind parameters for executing the statement. |

### all([...bindParameters]) ⇒ array of rows

Executes the SQL statement and returns an array of the resulting rows.

| Param          | Type                          | Description                                      |
| -------------- | ----------------------------- | ------------------------------------------------ |
| bindParameters | <code>array of objects</code> | The bind parameters for executing the statement. |

### iterate([...bindParameters]) ⇒ iterator

Executes the SQL statement and returns an iterator to the resulting rows.

| Param          | Type                          | Description                                      |
| -------------- | ----------------------------- | ------------------------------------------------ |
| bindParameters | <code>array of objects</code> | The bind parameters for executing the statement. |

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

### bind([...bindParameters]) ⇒ this

This function is currently not supported.
