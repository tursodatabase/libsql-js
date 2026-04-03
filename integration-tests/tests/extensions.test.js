import test from "ava";
import { Authorization, Action } from "libsql";

test.serial("Statement.run() returning duration", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT 1");
  const info = stmt.timing().run();
  t.not(info.duration, undefined);
  t.log(info.duration)
});

test.serial("Statement.get() returning duration", async (t) => {
  const db = t.context.db;

  const stmt = db.prepare("SELECT ?");
  const info = stmt.timing().get(1);
  t.not(info._metadata?.duration, undefined);
  t.log(info._metadata?.duration)
});

// ---- Legacy API (backward compatibility) ----

test.serial("Database.authorizer()/allow (legacy)", async (t) => {
  const db = t.context.db;

  db.authorizer({
    "users": Authorization.ALLOW
  });

  const stmt = db.prepare("SELECT * FROM users");
  const users = stmt.all();
  t.is(users.length, 2);
});

test.serial("Database.authorizer()/deny (legacy)", async (t) => {
  const db = t.context.db;

  db.authorizer({
    "users": Authorization.DENY
  });
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, {
    instanceOf: t.context.errorType,
    code: "SQLITE_AUTH"
  });
});

// ---- Rule-based API ----

test.serial("Rule-based: allow READ on table", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.READ, table: "users", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const stmt = db.prepare("SELECT * FROM users");
  const users = stmt.all();
  t.is(users.length, 2);
});

test.serial("Rule-based: deny all with default policy", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [],
    defaultPolicy: Authorization.DENY,
  });

  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, {
    instanceOf: t.context.errorType,
    code: "SQLITE_AUTH"
  });
});

test.serial("Rule-based: action-level deny PRAGMA", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.PRAGMA, policy: Authorization.DENY },
    ],
    defaultPolicy: Authorization.ALLOW,
  });

  await t.throwsAsync(async () => {
    return await db.prepare("PRAGMA table_info('users')");
  }, {
    instanceOf: t.context.errorType,
    code: "SQLITE_AUTH"
  });
});

test.serial("Rule-based: multiple actions in single rule", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: [Action.INSERT, Action.UPDATE, Action.DELETE], table: "users", policy: Authorization.DENY },
      { action: Action.SELECT, policy: Authorization.ALLOW },
      { action: Action.READ, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.ALLOW,
  });

  // SELECT should work
  const stmt = db.prepare("SELECT * FROM users");
  const users = stmt.all();
  t.is(users.length, 2);

  // INSERT should be denied
  await t.throwsAsync(async () => {
    return await db.prepare("INSERT INTO users (id, name, email) VALUES (3, 'Eve', 'eve@example.org')");
  }, {
    instanceOf: t.context.errorType,
    code: "SQLITE_AUTH"
  });
});

test.serial("Rule-based: glob pattern on table name", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS logs_access (id INTEGER PRIMARY KEY, msg TEXT)");
  db.exec("INSERT INTO logs_access (id, msg) VALUES (1, 'hello')");

  db.authorizer({
    rules: [
      { action: Action.READ, table: "logs_*", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const stmt = db.prepare("SELECT * FROM logs_access");
  const rows = stmt.all();
  t.is(rows.length, 1);

  // users table should be denied (doesn't match logs_*)
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, {
    instanceOf: t.context.errorType,
    code: "SQLITE_AUTH"
  });
});

test.serial("Rule-based: regex pattern on table name", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.READ, table: /^users$/, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const stmt = db.prepare("SELECT * FROM users");
  const users = stmt.all();
  t.is(users.length, 2);
});

test.serial("Rule-based: IGNORE returns NULL for READ columns", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.READ, table: "users", column: "email", policy: Authorization.IGNORE },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const stmt = db.prepare("SELECT id, name, email FROM users WHERE id = 1");
  const row = stmt.get();
  t.is(row.id, 1);
  t.is(row.name, "Alice");
  t.is(row.email, null);
});

test.serial("Rule-based: entity pattern for functions", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.FUNCTION, entity: /^(lower|upper|length)$/, policy: Authorization.ALLOW },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const stmt = db.prepare("SELECT upper(name) as uname FROM users WHERE id = 1");
  const row = stmt.get();
  t.is(row.uname, "ALICE");
});

test.serial("Rule-based: first match wins (order matters)", async (t) => {
  const db = t.context.db;

  // Specific deny for users table, then broad allow for all reads
  db.authorizer({
    rules: [
      { action: Action.READ, table: "users", policy: Authorization.DENY },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.ALLOW,
  });

  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, {
    instanceOf: t.context.errorType,
    code: "SQLITE_AUTH"
  });
});

test.serial("Rule-based: null removes authorizer", async (t) => {
  const db = t.context.db;

  // Set a restrictive authorizer
  db.authorizer({
    rules: [],
    defaultPolicy: Authorization.DENY,
  });

  // Should fail
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, {
    instanceOf: t.context.errorType,
    code: "SQLITE_AUTH"
  });

  // Remove authorizer
  db.authorizer(null);

  // Should succeed now
  const stmt = db.prepare("SELECT * FROM users");
  const users = stmt.all();
  t.is(users.length, 2);
});

test.serial("Rule-based: default policy allow", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [],
    defaultPolicy: Authorization.ALLOW,
  });

  const stmt = db.prepare("SELECT * FROM users");
  const users = stmt.all();
  t.is(users.length, 2);
});

// ---- Glob pattern tests ----

test.serial("Glob: ? matches exactly one character", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS log_a (id INTEGER PRIMARY KEY, msg TEXT)");
  db.exec("CREATE TABLE IF NOT EXISTS log_b (id INTEGER PRIMARY KEY, msg TEXT)");
  db.exec("INSERT INTO log_a (id, msg) VALUES (1, 'aaa')");
  db.exec("INSERT INTO log_b (id, msg) VALUES (1, 'bbb')");

  db.authorizer({
    rules: [
      { action: Action.READ, table: "log_?", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  // log_a and log_b should both match log_?
  const a = db.prepare("SELECT * FROM log_a").all();
  t.is(a.length, 1);
  const b = db.prepare("SELECT * FROM log_b").all();
  t.is(b.length, 1);

  // users should NOT match log_?
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });
});

test.serial("Glob: ? does not match zero or multiple characters", async (t) => {
  const db = t.context.db;

  // Create tables with varying suffix lengths
  db.exec("CREATE TABLE IF NOT EXISTS item_ (id INTEGER PRIMARY KEY)");
  db.exec("CREATE TABLE IF NOT EXISTS item_ab (id INTEGER PRIMARY KEY)");

  db.authorizer({
    rules: [
      { action: Action.READ, table: "item_?", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  // item_ (zero chars after _) should NOT match item_?
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM item_");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });

  // item_ab (two chars after _) should NOT match item_?
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM item_ab");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });
});

test.serial("Glob: * at start of pattern", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS audit_users (id INTEGER PRIMARY KEY, msg TEXT)");
  db.exec("INSERT INTO audit_users (id, msg) VALUES (1, 'x')");

  db.authorizer({
    rules: [
      { action: Action.READ, table: "*_users", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const rows = db.prepare("SELECT * FROM audit_users").all();
  t.is(rows.length, 1);
});

test.serial("Glob: * in middle of pattern", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS app_prod_logs (id INTEGER PRIMARY KEY, msg TEXT)");
  db.exec("INSERT INTO app_prod_logs (id, msg) VALUES (1, 'hello')");

  db.authorizer({
    rules: [
      { action: Action.READ, table: "app_*_logs", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const rows = db.prepare("SELECT * FROM app_prod_logs").all();
  t.is(rows.length, 1);

  // users doesn't match app_*_logs
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });
});

test.serial("Glob: multiple wildcards in one pattern", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS x_data_y (id INTEGER PRIMARY KEY)");
  db.exec("INSERT INTO x_data_y (id) VALUES (1)");

  db.authorizer({
    rules: [
      { action: Action.READ, table: "*_data_*", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const rows = db.prepare("SELECT * FROM x_data_y").all();
  t.is(rows.length, 1);
});

test.serial("Glob: on column name", async (t) => {
  const db = t.context.db;

  // IGNORE columns matching e* → email gets NULL, everything else readable
  db.authorizer({
    rules: [
      { action: Action.READ, table: "users", column: "e*", policy: Authorization.IGNORE },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const row = db.prepare("SELECT id, name, email FROM users WHERE id = 1").get();
  t.is(row.id, 1);
  t.is(row.name, "Alice");
  t.is(row.email, null); // email matches e*, gets IGNORE → NULL
});

test.serial("Glob: on entity name (pragma)", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.PRAGMA, entity: "table_*", policy: Authorization.ALLOW },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  // table_info matches table_*
  const info = db.prepare("PRAGMA table_info('users')").all();
  t.true(info.length > 0);
});

test.serial("Glob: exact string without wildcards is exact match", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.READ, table: "users", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const rows = db.prepare("SELECT * FROM users").all();
  t.is(rows.length, 2);
});

// ---- Regex pattern tests ----

test.serial("Regex: case-insensitive flag", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS Users_CI (id INTEGER PRIMARY KEY, val TEXT)");
  db.exec("INSERT INTO Users_CI (id, val) VALUES (1, 'test')");

  db.authorizer({
    rules: [
      { action: Action.READ, table: /^users_ci$/i, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const rows = db.prepare("SELECT * FROM Users_CI").all();
  t.is(rows.length, 1);
});

test.serial("Regex: partial match (no anchors)", async (t) => {
  const db = t.context.db;

  // /user/ without anchors should match "users" (partial match)
  db.authorizer({
    rules: [
      { action: Action.READ, table: /user/, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const rows = db.prepare("SELECT * FROM users").all();
  t.is(rows.length, 2);
});

test.serial("Regex: anchored pattern rejects partial matches", async (t) => {
  const db = t.context.db;

  // /^user$/ should NOT match "users" (has trailing s)
  db.authorizer({
    rules: [
      { action: Action.READ, table: /^user$/, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });
});

test.serial("Regex: alternation pattern", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS products (id INTEGER PRIMARY KEY, pname TEXT)");
  db.exec("INSERT INTO products (id, pname) VALUES (1, 'Widget')");

  db.authorizer({
    rules: [
      { action: Action.READ, table: /^(users|products)$/, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const u = db.prepare("SELECT * FROM users").all();
  t.is(u.length, 2);
  const p = db.prepare("SELECT * FROM products").all();
  t.is(p.length, 1);
});

test.serial("Regex: character class pattern", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS t1_data (id INTEGER PRIMARY KEY)");
  db.exec("CREATE TABLE IF NOT EXISTS t2_data (id INTEGER PRIMARY KEY)");
  db.exec("INSERT INTO t1_data (id) VALUES (1)");
  db.exec("INSERT INTO t2_data (id) VALUES (1)");

  db.authorizer({
    rules: [
      { action: Action.READ, table: /^t[0-9]_data$/, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const r1 = db.prepare("SELECT * FROM t1_data").all();
  t.is(r1.length, 1);
  const r2 = db.prepare("SELECT * FROM t2_data").all();
  t.is(r2.length, 1);

  // users shouldn't match
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });
});

test.serial("Regex: on column name with IGNORE", async (t) => {
  const db = t.context.db;

  // IGNORE any column ending in "il" → email gets NULL
  db.authorizer({
    rules: [
      { action: Action.READ, table: "users", column: /il$/, policy: Authorization.IGNORE },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const row = db.prepare("SELECT id, name, email FROM users WHERE id = 1").get();
  t.is(row.id, 1);
  t.is(row.name, "Alice");
  t.is(row.email, null); // "email" ends in "il"
});

test.serial("Regex: on entity name for allowed functions", async (t) => {
  const db = t.context.db;

  // Allow only functions starting with lowercase letters
  db.authorizer({
    rules: [
      { action: Action.FUNCTION, entity: /^[a-z]/, policy: Authorization.ALLOW },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const row = db.prepare("SELECT length(name) as len FROM users WHERE id = 1").get();
  t.is(row.len, 5); // "Alice" = 5 chars
});

test.serial("Regex: non-matching regex denies correctly", async (t) => {
  const db = t.context.db;

  // Only allow tables starting with "archive_"
  db.authorizer({
    rules: [
      { action: Action.READ, table: /^archive_/, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  // users doesn't start with archive_
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });
});

test.serial("Regex: complex pattern with quantifiers", async (t) => {
  const db = t.context.db;

  db.exec("CREATE TABLE IF NOT EXISTS logs_2024_01 (id INTEGER PRIMARY KEY, msg TEXT)");
  db.exec("INSERT INTO logs_2024_01 (id, msg) VALUES (1, 'jan')");

  // Match logs_YYYY_MM pattern
  db.authorizer({
    rules: [
      { action: Action.READ, table: /^logs_\d{4}_\d{2}$/, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const rows = db.prepare("SELECT * FROM logs_2024_01").all();
  t.is(rows.length, 1);

  // users doesn't match the date pattern
  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });
});

// ---- Combined glob/regex with multiple fields ----

test.serial("Glob table + regex column combo", async (t) => {
  const db = t.context.db;

  // For any table matching user*, IGNORE columns matching a secret-ish pattern
  db.authorizer({
    rules: [
      { action: Action.READ, table: "user*", column: /^(email|password|ssn)$/, policy: Authorization.IGNORE },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const row = db.prepare("SELECT id, name, email FROM users WHERE id = 2").get();
  t.is(row.id, 2);
  t.is(row.name, "Bob");
  t.is(row.email, null); // email matched the regex, users matched user*
});

test.serial("Regex table + glob column combo", async (t) => {
  const db = t.context.db;

  // For tables matching /^users$/, IGNORE columns matching e*
  db.authorizer({
    rules: [
      { action: Action.READ, table: /^users$/, column: "e*", policy: Authorization.IGNORE },
      { action: Action.READ, policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const row = db.prepare("SELECT id, name, email FROM users WHERE id = 1").get();
  t.is(row.id, 1);
  t.is(row.name, "Alice");
  t.is(row.email, null);
});

test.serial("Glob: wildcard-only pattern * matches everything", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.READ, table: "*", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  const rows = db.prepare("SELECT * FROM users").all();
  t.is(rows.length, 2);
});

test.serial("Glob: pattern with no match denies correctly", async (t) => {
  const db = t.context.db;

  db.authorizer({
    rules: [
      { action: Action.READ, table: "nonexistent_*", policy: Authorization.ALLOW },
      { action: Action.SELECT, policy: Authorization.ALLOW },
    ],
    defaultPolicy: Authorization.DENY,
  });

  await t.throwsAsync(async () => {
    return await db.prepare("SELECT * FROM users");
  }, { instanceOf: t.context.errorType, code: "SQLITE_AUTH" });
});

// ---- Setup ----

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
      DROP TABLE IF EXISTS logs_access;
      DROP TABLE IF EXISTS log_a;
      DROP TABLE IF EXISTS log_b;
      DROP TABLE IF EXISTS item_;
      DROP TABLE IF EXISTS item_ab;
      DROP TABLE IF EXISTS audit_users;
      DROP TABLE IF EXISTS app_prod_logs;
      DROP TABLE IF EXISTS x_data_y;
      DROP TABLE IF EXISTS Users_CI;
      DROP TABLE IF EXISTS products;
      DROP TABLE IF EXISTS t1_data;
      DROP TABLE IF EXISTS t2_data;
      DROP TABLE IF EXISTS logs_2024_01;
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
