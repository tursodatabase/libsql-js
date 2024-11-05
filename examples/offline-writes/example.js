import Database from "libsql";
import reader from "readline-sync";

const dbPath = process.env.LIBSQL_DB_PATH;
if (!dbPath) {
  throw new Error("Environment variable LIBSQL_DB_PATH is not set.");
}
const syncUrl = process.env.LIBSQL_SYNC_URL;
if (!syncUrl) {
  throw new Error("Environment variable LIBSQL_SYNC_URL is not set.");
}
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const options = { syncUrl: syncUrl, authToken: authToken, offline: true };
const db = new Database(dbPath, options);

db.exec("CREATE TABLE IF NOT EXISTS guest_book_entries (text TEXT)");

const comment = reader.question("Enter your comment: ");

console.log(comment);

const stmt = db.prepare("INSERT INTO guest_book_entries (text) VALUES (?)");
stmt.run(comment);
console.log("max write replication index: " + db.maxWriteReplicationIndex());

const replicated = db.sync();
console.log("frames synced: " + replicated.frames_synced);
console.log("frame no: " + replicated.frame_no);

console.log("Guest book entries:");
const rows = db.prepare("SELECT * FROM guest_book_entries").all();
for (const row of rows) {
    console.log(" - " + row.text);
}
