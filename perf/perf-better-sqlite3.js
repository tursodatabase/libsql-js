import { run, bench, group, baseline } from 'mitata';

import Database from 'better-sqlite3';

const db = new Database(':memory:');

db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
db.exec("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.org')");

const stmt = db.prepare("SELECT * FROM users WHERE id = ?");

group('Statement', () => {
  bench('get()', () => {
    stmt.get(1);
  });
  bench('get() [raw]', () => {
    stmt.raw().get(1);
  });
  bench('get() [pluck]', () => {
    stmt.pluck().get(1);
  });
});

await run({
  units: false, // print small units cheatsheet
  silent: false, // enable/disable stdout output
  avg: true, // enable/disable avg column (default: true)
  json: false, // enable/disable json output (default: false)
  colors: true, // enable/disable colors (default: true)
  min_max: true, // enable/disable min/max column (default: true)
  percentiles: true, // enable/disable percentiles column (default: true)
});
