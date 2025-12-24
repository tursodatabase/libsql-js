// Example: Connecting to an encrypted Turso Cloud database
//
// This example shows how to connect to a Turso Cloud database with
// remote encryption using libsql-js.
//
// Documentation: https://docs.turso.tech/cloud/encryption
//
// Usage:
//
//   export LIBSQL_URL="libsql://your-db.turso.io"
//   export LIBSQL_AUTH_TOKEN="your-token"
//   export LIBSQL_ENCRYPTION_KEY="encryption key in base64 format"
//   node cloud-encryption
//
// The encryption key must be encoded in base64 format.

import Database from "libsql";

const url = process.env.LIBSQL_URL;
const authToken = process.env.LIBSQL_AUTH_TOKEN;
const encryptionKey = process.env.LIBSQL_ENCRYPTION_KEY;

const opts = {
  authToken: authToken,
  remoteEncryptionKey: encryptionKey,
};

const db = new Database(url, opts);

db.exec("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
db.exec("INSERT OR REPLACE INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");
db.exec("INSERT OR REPLACE INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com')");

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);
console.log(`Name: ${row.name}, email: ${row.email}`);

db.close();