"use strict";

const { load, currentTarget } = require('@neon-rs/load');

// Static requires for bundlers.
if (0) { require('./.targets'); }

const { databaseNew, databaseExec, databasePrepare, statementGet } = load(__dirname) || require(`@libsql/experimental-${currentTarget()}`);

class Database {
    constructor(url) {
        this.db = databaseNew(url);
    }

    exec(sql) {
        databaseExec.call(this.db, sql);
    }

    prepare(sql) {
        const stmt = databasePrepare.call(this.db, sql);
        return new Statement(stmt);
    }
}

class Statement {
    constructor(stmt) {
        this.stmt = stmt;
    }

    get(...bindParameters) {
        return statementGet.call(this.stmt, ...bindParameters);
    }
}

module.exports = Database;
