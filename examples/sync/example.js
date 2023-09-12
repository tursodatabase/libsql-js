import Database from "libsql";
import reader from "readline-sync";

const url = process.env.LIBSQL_URL;
if (!url) {
  throw new Error("Environment variable LIBSQL_URL is not set.");
}
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const options = { syncUrl: url, authToken: authToken };
const db = new Database("hello.db", options);

db.sync();

db.exec("CREATE TABLE IF NOT EXISTS guest_book_entries (comment TEXT)");

db.sync();

const comment = reader.question("Enter your comment: ");

console.log(comment);

const stmt = db.prepare("INSERT INTO guest_book_entries (comment) VALUES (?)");
stmt.run(comment);

db.sync();

console.log("Guest book entries:");
const rows = db.prepare("SELECT * FROM guest_book_entries").all();
for (const row of rows) {
    console.log(" - " + row.comment);
}
