import Database from "libsql-experimental";

const url = process.env.LIBSQL_URL;
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const opts = {
  authToken: authToken,
};

const db = new Database(url, opts);

db.exec("CREATE TABLE IF NOT EXISTS users (id INT PRIMARY KEY, name TEXT, email TEXT)");
db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");
db.exec("INSERT INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com')");

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
