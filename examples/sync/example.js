import Database from "libsql";
import reader from "readline-sync";

const url = process.env.LIBSQL_URL;
if (!url) {
  throw new Error("Environment variable LIBSQL_URL is not set.");
}
const authToken = process.env.LIBSQL_AUTH_TOKEN;

const options = { syncUrl: url, authToken: authToken };

// Sync also supports Turso Cloud encryption.
// 
// Documentation: https://docs.turso.tech/cloud/encryption
//
//
//   export LIBSQL_ENCRYPTION_KEY="encryption key in base64 format"
//
// The encryption key must be encoded in base64 format.

const encryptionKey = process.env.LIBSQL_ENCRYPTION_KEY;

if (encryptionKey) {
  options.remoteEncryptionKey = encryptionKey;
}

const db = new Database("hello.db", options);

db.sync();

db.exec("CREATE TABLE IF NOT EXISTS guest_book_entries (comment TEXT)");

db.sync();

const comment = reader.question("Enter your comment: ");

console.log(comment);

const stmt = db.prepare("INSERT INTO guest_book_entries (comment) VALUES (?)");
stmt.run(comment);
console.log("max write replication index: " + db.maxWriteReplicationIndex());

const replicated = db.sync();
console.log("frames synced: " + replicated.frames_synced);
console.log("frame no: " + replicated.frame_no);

console.log("Guest book entries:");
const rows = db.prepare("SELECT * FROM guest_book_entries").all();
for (const row of rows) {
    console.log(" - " + row.comment);
}
