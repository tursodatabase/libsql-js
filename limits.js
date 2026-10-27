/**
 * SQLite run-time limit categories, for use with `Database.limit()` and
 * `Database.setLimit()`.
 *
 * The values match SQLite's `SQLITE_LIMIT_*` constants. See
 * https://sqlite.org/c3ref/c_limit_attached.html for details.
 *
 * @readonly
 * @enum {number}
 */
const Limit = {
  /** The maximum size of any string or BLOB or table row, in bytes. */
  LENGTH: 0,
  /** The maximum length of an SQL statement, in bytes. */
  SQL_LENGTH: 1,
  /** The maximum number of columns in a table, index, result set, ORDER BY or GROUP BY. */
  COLUMN: 2,
  /** The maximum depth of the parse tree on any expression. */
  EXPR_DEPTH: 3,
  /** The maximum number of terms in a compound SELECT statement. */
  COMPOUND_SELECT: 4,
  /** The maximum number of instructions in a virtual machine program. */
  VDBE_OP: 5,
  /** The maximum number of arguments on a function. */
  FUNCTION_ARG: 6,
  /** The maximum number of attached databases. */
  ATTACHED: 7,
  /** The maximum length of the pattern argument to LIKE or GLOB. */
  LIKE_PATTERN_LENGTH: 8,
  /** The maximum index number of any parameter in an SQL statement. */
  VARIABLE_NUMBER: 9,
  /** The maximum depth of recursion for triggers. */
  TRIGGER_DEPTH: 10,
  /** The maximum number of auxiliary worker threads per prepared statement. */
  WORKER_THREADS: 11,
};

module.exports = { Limit };
