import test from "ava";

test.serial("Statement.run() returning duration", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT 1");
  const info = stmt.run();
  t.not(info.duration, undefined);
  t.log(info.duration)
});

test.serial("Statement.get() returning duration", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT ?");
  const info = stmt.get(1);
  t.not(info._metadata?.duration, undefined);
  t.log(info._metadata?.duration)
});

const connect = async (path_opt) => {
  const path = path_opt ?? "hello.db";
  const x = await import("libsql");
  const db = new x.default(process.env.LIBSQL_DATABASE ?? path, {});
  return [db, x.SqliteError, "libsql"];
};

test.beforeEach(async (t) => {
  const [db, errorType, provider] = await connect();
  db.exec(`
      DROP TABLE IF EXISTS users;
      CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)
  `);
  db.exec(
    "INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')"
  );
  db.exec(
    "INSERT INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com')"
  );
  t.context = {
    db,
    errorType,
    provider
  };
});
