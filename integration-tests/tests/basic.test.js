import test from "ava";

test.beforeEach(async (t) => {
  const db = await connect();
	t.context = {
		db,
	};
});

test("basic usage", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");

  db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");

  const userId = 1;

  const row = db.prepare("SELECT * FROM users WHERE id = ?").get(userId);

  t.is(row.name, "Alice");
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
