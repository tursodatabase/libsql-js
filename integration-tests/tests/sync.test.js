import test from "ava";

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

test.after.always(async (t) => {
  if (t.context.db != undefined) {
    t.context.db.close();
  }
});

test.serial("Open in-memory database", async (t) => {
  const [db] = await connect(":memory:");
  t.is(db.memory, true);
});

test.serial("Statement.prepare() error", async (t) => {
  const db = t.context.db;

  t.throws(() => {
    return db.prepare("SYNTAX ERROR");
  }, {
    instanceOf: t.context.errorType,
    message: 'near "SYNTAX": syntax error'
  });
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

  var stmt = 0;

  stmt = db.prepare("SELECT * FROM users WHERE id = ?");
  t.is(stmt.get(0), undefined);
  t.is(stmt.get([0]), undefined);
  t.is(stmt.get(1).name, "Alice");
  t.is(stmt.get(2).name, "Bob");

  stmt = db.prepare("SELECT * FROM users WHERE id = ?1");
  t.is(stmt.get({1: 0}), undefined);
  t.is(stmt.get({1: 1}).name, "Alice");
  t.is(stmt.get({1: 2}).name, "Bob");
});

test.serial("Statement.get() [named]", async (t) => {
  const db = t.context.db;

  var stmt = undefined;

  stmt = db.prepare("SELECT :b, :a");
  t.deepEqual(stmt.raw().get({ a: 'a', b: 'b' }), ['b', 'a']);

  stmt = db.prepare("SELECT * FROM users WHERE id = :id");
  t.is(stmt.get({ id: 0 }), undefined);
  t.is(stmt.get({ id: 1 }).name, "Alice");
  t.is(stmt.get({ id: 2 }).name, "Bob");

  stmt = db.prepare("SELECT * FROM users WHERE id = @id");
  t.is(stmt.get({ id: 0 }), undefined);
  t.is(stmt.get({ id: 1 }).name, "Alice");
  t.is(stmt.get({ id: 2 }).name, "Bob");

  stmt = db.prepare("SELECT * FROM users WHERE id = $id");
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
  t.is(stmt.iterate([]).next().done, true);
  t.is(stmt.iterate({}).next().done, true);
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

test.serial("Statement.all() [default safe integers]", async (t) => {
  const db = t.context.db;
  db.defaultSafeIntegers();
  const stmt = db.prepare("SELECT * FROM users");
  const expected = [
    [1n, "Alice", "alice@example.org"],
    [2n, "Bob", "bob@example.com"],
  ];
  t.deepEqual(stmt.raw().all(), expected);
});

test.serial("Statement.all() [statement safe integers]", async (t) => {
  const db = t.context.db;
  const stmt = db.prepare("SELECT * FROM users");
  stmt.safeIntegers();
  const expected = [
    [1n, "Alice", "alice@example.org"],
    [2n, "Bob", "bob@example.com"],
  ];
  t.deepEqual(stmt.raw().all(), expected);
});

test.serial("Statement.raw() [failure]", async (t) => {
  const db = t.context.db;
  const stmt = db.prepare("INSERT INTO users (id, name, email) VALUES (?, ?, ?)");
  await t.throws(() => {
    stmt.raw()
  }, {
    message: 'The raw() method is only for statements that return data'
  });
});

test.serial("Statement.columns()", async (t) => {
  const db = t.context.db;

  var stmt = undefined;

  stmt = db.prepare("SELECT 1");
  t.deepEqual(stmt.columns(), [
    {
      column: null,
      database: null,
      name: '1',
      table: null,
      type: null,
    },
  ]);

  stmt = db.prepare("SELECT * FROM users WHERE id = ?");
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
    t.is(db.inTransaction, true);
    for (const user of users) insert.run(user);
  });

  t.is(db.inTransaction, false);
  insertMany([
    { name: "Joey", email: "joey@example.org" },
    { name: "Sally", email: "sally@example.org" },
    { name: "Junior", email: "junior@example.org" },
  ]);
  t.is(db.inTransaction, false);

  const stmt = db.prepare("SELECT * FROM users WHERE id = ?");
  t.is(stmt.get(3).name, "Joey");
  t.is(stmt.get(4).name, "Sally");
  t.is(stmt.get(5).name, "Junior");
});

test.serial("values", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT ?").raw();
  t.deepEqual(stmt.get(1), [1]);
  t.deepEqual(stmt.get(Number.MIN_VALUE), [Number.MIN_VALUE]);
  t.deepEqual(stmt.get(Number.MAX_VALUE), [Number.MAX_VALUE]);
  t.deepEqual(stmt.get(Number.MAX_SAFE_INTEGER), [Number.MAX_SAFE_INTEGER]);
  t.deepEqual(stmt.get(9007199254740991n), [9007199254740991]);
});

test.serial("Database.pragma()", async (t) => {
  const db = t.context.db;
  db.pragma("cache_size = 2000");
  t.deepEqual(db.pragma("cache_size"), [{ "cache_size": 2000 }]);
});

test.serial("errors", async (t) => {
  const db = t.context.db;

  const syntaxError = await t.throws(() => {
    db.exec("SYNTAX ERROR");
  }, {
    instanceOf: t.context.errorType,
    message: 'near "SYNTAX": syntax error',
    code: 'SQLITE_ERROR'
  });
  const noTableError = await t.throws(() => {
    db.exec("SELECT * FROM missing_table");
  }, {
    instanceOf: t.context.errorType,
    message: "no such table: missing_table",
    code: 'SQLITE_ERROR'
  });

  if (t.context.provider === 'libsql') {
    t.is(noTableError.rawCode, 1)
    t.is(syntaxError.rawCode, 1)
  }
});

const connect = async (path_opt) => {
  const path = path_opt ?? "hello.db";
  const provider = process.env.PROVIDER;
  if (provider === "libsql") {
    const database = process.env.LIBSQL_DATABASE ?? path;
    const x = await import("libsql");
    const options = {};
    const db = new x.default(database, options);
    return [db, x.SqliteError, provider];
  }
  if (provider == "sqlite") {
    const x = await import("better-sqlite3");
    const options = {};
    const db = x.default(path, options);
    return [db, x.SqliteError, provider];
  }
  throw new Error("Unknown provider: " + provider);
};
