import Database from "libsql-experimental";

const url = process.env.LIBSQL_URL;
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const options = { syncUrl: url, syncAuth: authToken };
const db = new Database("hello.db", options);

db.sync();

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
