// Run `npm run build`, then from this directory run:
// `npm install && node --expose-gc perf-libsql-batched-rows.js`
import { baseline, bench, group, run } from 'mitata';

// Import the parent checkout so this benchmark uses its locally built native module.
import libsql from '../promise.js';

const { connect } = libsql;

const BATCH_SIZE = 250;
const ROW_COUNTS = [1_000, 10_000, 100_000, 1_000_000];
const MAX_ROW_COUNT = ROW_COUNTS[ROW_COUNTS.length - 1];

const db = await connect(':memory:', {});
await db.exec(`
  CREATE TABLE benchmark_rows (value INTEGER PRIMARY KEY);
  WITH RECURSIVE numbers(value) AS (
    SELECT 1
    UNION ALL
    SELECT value + 1 FROM numbers WHERE value < ${MAX_ROW_COUNT}
  )
  INSERT INTO benchmark_rows SELECT value FROM numbers;
`);

const stmt = await db.prepare(`
  SELECT value
  FROM benchmark_rows
  WHERE value <= ?
  ORDER BY value
`);

for (const rowCount of ROW_COUNTS) {
  group(`${rowCount.toLocaleString('en-US')} rows`, () => {
    baseline('all()', async () => {
      validateRows(await stmt.all(rowCount), rowCount);
    });
    bench(`allBatched(${BATCH_SIZE})`, async () => {
      validateRows(await stmt.allBatched(BATCH_SIZE, rowCount), rowCount);
    });
  });
}

await run({
  units: false,
  silent: false,
  avg: true,
  json: false,
  colors: process.stdout.isTTY,
  min_max: true,
  percentiles: true,
});

db.close();

function validateRows(rows, expectedCount) {
  if (
    rows.length !== expectedCount ||
    rows[0]?.value !== 1 ||
    rows[rows.length - 1]?.value !== expectedCount
  ) {
    throw new Error(`Expected rows 1 through ${expectedCount}`);
  }
}
