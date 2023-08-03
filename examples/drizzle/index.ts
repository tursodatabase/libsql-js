import { sqliteTable, text, integer } from 'drizzle-orm/sqlite-core';
import { drizzle } from 'drizzle-orm/better-sqlite3';
import Database from 'libsql-experimental';

const users = sqliteTable('users', {
  id: integer('id').primaryKey(),  // 'id' is the column name
  fullName: text('full_name'),
})

const opts = {
  syncUrl: 'http://localhost:8081'
};
const sqlite = new Database('sqlite.db', opts);

sqlite.sync();

const db = drizzle(sqlite);

const allUsers = db.select().from(users).all();

console.log(allUsers);
