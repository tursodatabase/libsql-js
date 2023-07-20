import test from "ava";

test.beforeEach(async (t) => {
  const db = await connect();
  db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
  db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");
  db.exec("INSERT INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com')");
	t.context = {
		db,
	};
});

test("Statement.get()", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users WHERE id = ?");
  t.is(stmt.get(1).name, "Alice");
  t.is(stmt.get(2).name, "Bob");
});

test("errors", async (t) => {
  const db = t.context.db;

  const error = await t.throws(() => {
    db.exec("SELECT * FROM missing_table")
  });
  t.is(error.message, 'no such table: missing_table');
});

const connect = async () => {
  const provider = process.env.PROVIDER;
  if (provider === "libsql") {
    const x = await import("libsql-experimental");
    const options = {};
    const db = new x.default(":memory:", options);
    return db;
  }
  if (provider == "sqlite") {
    const x = await import("better-sqlite3");
    const options = {};
    const db = x.default(":memory:", options);
    return db;
  }
  throw new Error("Unknown provider: " + provider);
};
