# Experimental libSQL API for Node

This source repository contains libSQL API bindings for Node, which aim to be compatible with [better-sqlite3](https://github.com/WiseLibs/better-sqlite3/).

## Installing

You can install the package with `npm`:

```sh
npm i libsql-experimental
```

### Usage

```javascript
import Database from 'libsql-experimental';

const db = new Database(':memory:');

db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
```
