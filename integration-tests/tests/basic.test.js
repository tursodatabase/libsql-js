import test from "ava";

test("basic usage", async (t) => {
  for (const provider of ["sqlite", "libsql"]) {
    await testBasicUsage(provider, t);
  }
});

const testBasicUsage = async (provider, t) => {
  const db = await connect(provider);

  db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");

  db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");

  const userId = 1;

  const row = db.prepare("SELECT * FROM users WHERE id = ?").get(userId);

  t.is(row.name, "Alice");
};

test("errors", async (t) => {
  for (const provider of ["sqlite", "libsql"]) {
    await testErrors(provider, t);
  }
});

const testErrors = async (provider, t) => {
  const db = await connect(provider);

  const error = await t.throws(() => {
    db.exec("SELECT * FROM users")
  });
  t.is(error.message, 'no such table: users');
}

const connect = async (provider) => {
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
