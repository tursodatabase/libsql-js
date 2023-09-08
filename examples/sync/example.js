import Database from "libsql";

const url = process.env.LIBSQL_URL;
if (!url) {
  throw new Error("Environment variable LIBSQL_URL is not set.");
}
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const options = { syncUrl: url, authToken: authToken };
const db = new Database("hello.db", options);

db.sync();

var rows = undefined;

console.log("After sync:");

rows = db.prepare("SELECT * FROM users").all();
for (const row of rows) {
    console.log(row);
}

db.exec("INSERT INTO users VALUES (4, 'Pekka Enberg')");

console.log("After write:");

rows = db.prepare("SELECT * FROM users").all();
for (const row of rows) {
    console.log(row);
}
