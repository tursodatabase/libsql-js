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

test.serial("Database.authorizer() [allow]", async (t) => {
  const db = t.context.db;
  db.authorizer(auth => auth("allow"));
  const stmt = db.prepare("SELECT * FROM users");
  const rows = stmt.all();
  t.is(rows.length, 2);
});

test.serial("Database.authorizer() [ignore]", async (t) => {
  const db = t.context.db;
  db.authorizer(auth => auth("ignore"));
  const stmt = db.prepare("SELECT * FROM users");
  const rows = stmt.all();
  t.is(rows.length, 0);
});

test.serial("Database.authorizer() [deny]", async (t) => {
  const db = t.context.db;
  db.authorizer(auth => auth("deny"));
  let error = null;
  try {
    await db.prepare("SELECT * FROM users");
  } catch (err) {
    error = err;
  }
  t.truthy(error, "Expected an error to be thrown");

  let parsed;
  try {
    parsed = JSON.parse(error.message);
  } catch {
    t.fail("Error message is not valid JSON: " + String(error && error.message));
  }
  t.is(parsed.code, "SQLITE_AUTH");
  t.is(parsed.libsqlError, true);
  t.is(parsed.message, "Authorization denied by JS authorizer");
  t.is(parsed.rawCode, 23);
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
