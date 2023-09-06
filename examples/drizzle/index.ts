import { sqliteTable, text, integer } from 'drizzle-orm/sqlite-core';
import { drizzle } from 'drizzle-orm/better-sqlite3';
import { migrate } from "drizzle-orm/better-sqlite3/migrator";
import Database from 'libsql';

const users = sqliteTable('users', {
  id: integer('id').primaryKey(),  // 'id' is the column name
  fullName: text('full_name'),
})

const sqlite = new Database('drizzle.db');

const db = drizzle(sqlite);

const allUsers = db.select().from(users).all();

console.log(allUsers);
