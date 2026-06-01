import test from "ava";
import crypto from "crypto";
import fs from "fs";

// Reproducer for the connection-exhaustion regression reported between
// 0.6.0-pre.33 (opens 1500 connections cleanly) and 0.6.0-pre.36 (fails
// around 400 opens). Each test opens a large number of connections to the
// same on-disk database and keeps them all open at once, then closes them.
//
// The count can be overridden with the CONNECTION_COUNT env var.
const CONNECTION_COUNT = Number(process.env.CONNECTION_COUNT ?? 1500);

test.serial("Open many connections [async]", async (t) => {
  const libsql = await import("libsql/promise");
  const path = genDatabaseFilename();

  const connections = [];
  try {
    for (let i = 0; i < CONNECTION_COUNT; i++) {
      const db = await libsql.connect(path);
      // Touch the connection so we know it is actually usable.
      await db.exec("SELECT 1");
      connections.push(db);
    }
    t.is(connections.length, CONNECTION_COUNT);
  } finally {
    for (const db of connections) {
      db.close();
    }
    cleanup(path);
  }
});

test.serial("Open many connections [sync]", async (t) => {
  const x = await import("libsql");
  const Database = x.default;
  const path = genDatabaseFilename();

  const connections = [];
  try {
    for (let i = 0; i < CONNECTION_COUNT; i++) {
      const db = new Database(path);
      // Touch the connection so we know it is actually usable.
      db.exec("SELECT 1");
      connections.push(db);
    }
    t.is(connections.length, CONNECTION_COUNT);
  } finally {
    for (const db of connections) {
      db.close();
    }
    cleanup(path);
  }
});

/// Generate a unique database filename
const genDatabaseFilename = () => {
  return `test-${crypto.randomBytes(8).toString("hex")}.db`;
};

const cleanup = (path) => {
  for (const suffix of ["", "-wal", "-shm"]) {
    try {
      fs.unlinkSync(path + suffix);
    } catch {
      // ignore
    }
  }
};
