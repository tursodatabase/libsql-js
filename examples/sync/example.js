import Database from 'libsql-experimental';

const options = { syncUrl: "http://localhost:8080" };
const db = new Database('hello.db', options);

db.sync();

const row = db.prepare("SELECT * FROM users WHERE id = ?").get(1);

console.log(`Name: ${row.name}, email: ${row.email}`);
