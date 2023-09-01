import test from "ava";

test.beforeEach(async (t) => {
  const db = await connect();
  db.exec("DROP TABLE IF EXISTS users");
  db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
  db.exec(
    "INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')"
  );
  db.exec(
    "INSERT INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com')"
  );
  t.context = {
    db,
  };
});

test.after.always(async (t) => {
  if (t.context.db != undefined) {
    t.context.db.close();
  }
});

test.serial("Open in-memory database", async (t) => {
  const db = await connect(":memory:");
  t.is(db.memory, true);
});

test.serial("Statement.prepare() error", async (t) => {
  const db = t.context.db;

  const syntax_error = t.throws(() => {
    return db.prepare("SYNTAX ERROR");
  });
  t.is(syntax_error.message, 'near "SYNTAX": syntax error');
});

test.serial("Statement.run() [positional]", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("INSERT INTO users(name, email) VALUES (?, ?)");
  const info = stmt.run(["Carol", "carol@example.net"]);
  t.is(info.changes, 1);
  t.is(info.lastInsertRowid, 3);
});

test.serial("Statement.get() [positional]", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users WHERE id = ?");
  t.is(stmt.get(0), undefined);
  t.is(stmt.get([0]), undefined);
  t.is(stmt.get(1).name, "Alice");
  t.is(stmt.get(2).name, "Bob");
});

test.serial("Statement.get() [named]", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users WHERE id = :id");
  t.is(stmt.get({ id: 0 }), undefined);
  t.is(stmt.get({ id: 1 }).name, "Alice");
  t.is(stmt.get({ id: 2 }).name, "Bob");
});

test.serial("Statement.get() [raw]", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users WHERE id = ?");
  t.deepEqual(stmt.raw().get(1), [1, "Alice", "alice@example.org"]);
});

test.serial("Statement.iterate() [empty]", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users WHERE id = 0");
  t.is(stmt.iterate().next().done, true);
});

test.serial("Statement.iterate()", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users");
  const expected = [1, 2];
  var idx = 0;
  for (const row of stmt.iterate()) {
    t.is(row.id, expected[idx++]);
  }
});

test.serial("Statement.all()", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users");
  const expected = [
    { id: 1, name: "Alice", email: "alice@example.org" },
    { id: 2, name: "Bob", email: "bob@example.com" },
  ];
  t.deepEqual(stmt.all(), expected);
});

test.serial("Statement.all() [raw]", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users");
  const expected = [
    [1, "Alice", "alice@example.org"],
    [2, "Bob", "bob@example.com"],
  ];
  t.deepEqual(stmt.raw().all(), expected);
});

test.serial("Statement.columns()", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT * FROM users WHERE id = ?");
  t.deepEqual(stmt.columns(), [
    {
      column: "id",
      database: "main",
      name: "id",
      table: "users",
      type: "INTEGER",
    },
    {
      column: "name",
      database: "main",
      name: "name",
      table: "users",
      type: "TEXT",
    },
    {
      column: "email",
      database: "main",
      name: "email",
      table: "users",
      type: "TEXT",
    },
  ]);
});

test.serial("Database.transaction()", async (t) => {
  const db = t.context.db;

  const insert = db.prepare(
    "INSERT INTO users(name, email) VALUES (:name, :email)"
  );

  const insertMany = db.transaction((users) => {
    for (const user of users) insert.run(user);
  });

  insertMany([
    { name: "Joey", email: "joey@example.org" },
    { name: "Sally", email: "sally@example.org" },
    { name: "Junior", email: "junior@example.org" },
  ]);

  const stmt = db.prepare("SELECT * FROM users WHERE id = ?");
  t.is(stmt.get(3).name, "Joey");
  t.is(stmt.get(4).name, "Sally");
  t.is(stmt.get(5).name, "Junior");
});

test.serial("errors", async (t) => {
  const db = t.context.db;

  const syntax_error = await t.throws(() => {
    db.exec("SYNTAX ERROR");
  });
  t.is(syntax_error.message, 'near "SYNTAX": syntax error');
  const no_such_table_error = await t.throws(() => {
    db.exec("SELECT * FROM missing_table");
  });
  t.is(no_such_table_error.message, "no such table: missing_table");
});

const connect = async (path_opt) => {
  const path = path_opt ?? "hello.db";
  const provider = process.env.PROVIDER;
  if (provider === "libsql") {
    const database = process.env.LIBSQL_DATABASE ?? path;
    const x = await import("libsql-experimental");
    const options = {};
    const db = new x.default(database, options);
    return db;
  }
  if (provider == "sqlite") {
    const x = await import("better-sqlite3");
    const options = {};
    const db = x.default(path, options);
    return db;
  }
  throw new Error("Unknown provider: " + provider);
};
