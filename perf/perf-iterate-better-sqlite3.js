import { run, bench, group, baseline } from 'mitata';

import Database from 'better-sqlite3';

const db = new Database(':memory:');

db.exec(`CREATE TABLE users (
    field1 TEXT,
    field2 TEXT,
    field3 TEXT,
    field4 TEXT,
    field5 TEXT,
    field6 TEXT,
    field7 TEXT,
    field8 TEXT,
    field9 TEXT,
    field10 TEXT,
    field11 TEXT,
    field12 TEXT,
    field13 TEXT,
    field14 TEXT,
    field15 TEXT,
    field16 TEXT,
    field17 TEXT,
    field18 TEXT,
    field19 TEXT,
    field20 TEXT,
    field21 TEXT,
    field22 TEXT,
    field23 TEXT,
    field24 TEXT,
    field25 TEXT,
    field26 TEXT,
    field27 TEXT,
    field28 TEXT,
    field29 TEXT,
    field30 TEXT,
    field31 TEXT,
    field32 TEXT,
    field33 TEXT,
    field34 TEXT,
    field35 TEXT,
    field36 TEXT,
    field37 TEXT,
    field38 TEXT,
    field39 TEXT,
    field40 TEXT,
    field41 TEXT,
    field42 TEXT,
    field43 TEXT,
    field44 TEXT,
    field45 TEXT,
    field46 TEXT,
    field47 TEXT,
    field48 TEXT,
    field49 TEXT,
    field50 TEXT,
    field51 INTEGER,
    field52 INTEGER,
    field53 INTEGER,
    field54 INTEGER,
    field55 INTEGER,
    field56 INTEGER,
    field57 INTEGER,
    field58 INTEGER,
    field59 INTEGER,
    field60 INTEGER,
    field61 INTEGER,
    field62 INTEGER,
    field63 INTEGER,
    field64 INTEGER,
    field65 INTEGER,
    field66 INTEGER,
    field67 INTEGER,
    field68 INTEGER,
    field69 INTEGER,
    field70 INTEGER
)`);
for (let id = 0; id < 500; id++) {
    db.exec(`INSERT INTO users VALUES (
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        'some string here',
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id},
        ${id}
    )`);
}

const stmt = db.prepare("SELECT * FROM users WHERE field70 > ?");

group('Statement', () => {
  bench('iterate', () => {
      for (const row of stmt.iterate(10)) {
          if (row.field1 === 'Never appears') {
            break;
          }
      }
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
