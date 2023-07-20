# Experimental libSQL API for Node

## Getting Started

Install the package:

```sh
npm i libsql-experimental
```

Example application:

```javascript
import Database from 'libsql-experimental';

const db = new Database(':memory:');

db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
```

The packaging is based on the [neon-prebuild-example](https://github.com/dherman/neon-prebuild-example) project.
