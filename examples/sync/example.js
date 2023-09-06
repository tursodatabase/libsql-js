import Database from "libsql";

const url = process.env.LIBSQL_URL;
if (!url) {
  throw new Error("Environment variable LIBSQL_URL is not set.");
}
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const options = { syncUrl: url, authToken: authToken };
const db = new Database("hello.db", options);

db.sync();

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
