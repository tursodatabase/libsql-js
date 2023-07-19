import libsql from 'libsql-experimental';

const db = new libsql.Database(':memory:');

db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");

db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");

const userId = 1;

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(userId);

console.log(row.name);
