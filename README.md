# libSQL API for JavaScript/TypeScript

[![npm](https://badge.fury.io/js/libsql.svg)](https://badge.fury.io/js/libsql)

[libSQL](https://github.com/libsql/libsql) is an open source, open contribution fork of SQLite.
This source repository contains libSQL API bindings for Node, which aims to be compatible with [better-sqlite3](https://github.com/WiseLibs/better-sqlite3/), but with opt-in promise API.

**Please note you are currently encouraged to use the [libSQL SDK](https://github.com/libsql/libsql-client-ts) as it is more mature at this point and also supports serverless applications that only can use `fetch()`.**

## Features

* In-memory and local libSQL/SQLite databases
* Remote libSQL databases
* Embedded, in-app replica that syncs with a remote libSQL database
* Supports Bun, Deno, and Node on macOS, Linux, and Windows.

## Installing

You can install the package with:

**Node:**

```sh
npm i libsql
```

**Bun:**

```sh
bun add libsql
```

**Deno:**

Use the `npm:` prefix for package import:

```typescript
import Database from 'npm:libsql';
```

## Documentation

* [API reference](docs/api.md)

## Getting Started

To try out your first libsql program, type the following in `hello.js`:

```javascript
import Database from 'libsql';

const db = new Database(':memory:');

db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
```

and then run:

```shell
$ node hello.js
```

To use the promise API, import `libsql/promise`:

```javascript
import Database from 'libsql/promise';

const db = new Database(':memory:');

await db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
await db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");

const stmt = await db.prepare("SELECT * FROM users WHERE id = ?");
const row = stmt.get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
```

#### Connecting to a local database file

```javascript
import Database from 'libsql';

const db = new Database('hello.db');
````

#### Connecting to a Remote libSQL server

```javascript
import Database from 'libsql';

const url = process.env.LIBSQL_URL;
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const opts = {
  authToken: authToken,
};

const db = new Database(url, opts);
```

#### Creating an in-app replica and syncing it

```javascript
import libsql

const opts = { syncUrl: "<url>", authToken: "<optional auth token>" };
const db = new Database('hello.db', opts);
db.sync();
```

#### Creating a table

```javascript
db.exec("CREATE TABLE users (id INTEGER, email TEXT);")
```

#### Inserting rows into a table

```javascript
db.exec("INSERT INTO users VALUES (1, 'alice@example.org')")
```

#### Querying rows from a table

```javascript
const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);
```

## Developing

To build the `libsql` package, run:

```console
npm run build
```

You can then run the integration tests with:

```console
npm link
cd integration-tests
npm link libsql
npm test
```

## License

This project is licensed under the [MIT license].

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in libSQL by you, shall be licensed as MIT, without any additional
terms or conditions.

[MIT license]: https://github.com/libsql/libsql-node/blob/main/LICENSE
