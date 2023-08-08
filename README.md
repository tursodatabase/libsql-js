# libSQL API for Node

[![npm](https://badge.fury.io/js/libsql-experimental.svg)](https://badge.fury.io/js/libsql-experimental)

This source repository contains libSQL API bindings for Node, which aims to be compatible with [better-sqlite3](https://github.com/WiseLibs/better-sqlite3/).

## Installing

You can install the package with `npm`:

```sh
npm i libsql-experimental
```

## Documentation

* [API reference](docs/api.md)

## Getting Started

To try out your first libsql program, type the following in `hello.js`:

```javascript
import Database from 'libsql-experimental';

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

#### Connecting to a database

```javascript
import Database from 'libsql-experimental';

const db = new Database('hello.db');
````

#### Embedded replica

```javascript
import libsql_experimental as libsql

const opts = { syncUrl: "<url>" };
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

## License

This project is licensed under the [MIT license].

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in libSQL by you, shall be licensed as MIT, without any additional
terms or conditions.

[MIT license]: https://github.com/libsql/libsql-experimental-node/blob/main/LICENSE.md
