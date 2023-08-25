import Database from "libsql-experimental";

const url = process.env.LIBSQL_URL;
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const opts = {
  authToken: authToken,
};

const db = new Database(url, opts);

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
