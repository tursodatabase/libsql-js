import { connect } from "libsql/promise";

const TIMEOUT = 500;
const db = await connect(":memory:", {
    defaultQueryTimeout: TIMEOUT,
});

await db.exec("CREATE TABLE t(x INTEGER)");
const insert = await db.prepare("INSERT INTO t values (?)");
for (let i = 0; i < 10000; i++) {
    await insert.run(i);
}

let okCount = 0;
let errCount = 0;
let maxDuration = 0;

async function query() {
    const stmt = await db.prepare("SELECT * FROM t ORDER BY x ASC");
    const res = await stmt.all();
    return res;
}

async function batch(n) {
    for (let i = 0; i < n; i++) {
        const start = performance.now();
        await query()
        .then(() => {
            const duration = performance.now() - start;
            if (duration > maxDuration) maxDuration = duration;
            okCount++;
        })
        .catch((e) => {
            const duration = performance.now() - start;
            if (duration > maxDuration) maxDuration = duration;
            errCount++;
            if (errCount <= 5) console.error("err:", e.code || e.name, e.message, duration.toFixed(1));
        });
    }
}

const startAll = performance.now();
const batches = [];
for (let i = 0; i < 100; i++) {
    batches.push(batch(10));
}
await Promise.all(batches);
const total = performance.now() - startAll;
console.log(`total wall: ${total.toFixed(1)}ms; ok=${okCount}, err=${errCount}, maxDur=${maxDuration.toFixed(1)}ms`);
