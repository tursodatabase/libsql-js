/**
 * Authorization outcome.
 *
 * @readonly
 * @enum {number}
 * @property {number} ALLOW - Allow access to a resource.
 * @property {number} DENY - Deny access to a resource and throw an error.
 * @property {number} IGNORE - For READ: return NULL instead of the column value. For other actions: equivalent to DENY.
 */
const Authorization = {
  /**
   * Allow access to a resource.
   * @type {number}
   */
  ALLOW: 0,

  /**
   * Deny access to a resource and throw an error in `prepare()`.
   * @type {number}
   */
  DENY: 1,

  /**
   * For READ: return NULL instead of the actual column value.
   * For other actions: equivalent to DENY.
   * @type {number}
   */
  IGNORE: 2,
};

/**
 * SQLite authorizer action codes.
 *
 * @readonly
 * @enum {number}
 */
const Action = {
  CREATE_INDEX: 1,
  CREATE_TABLE: 2,
  CREATE_TEMP_INDEX: 3,
  CREATE_TEMP_TABLE: 4,
  CREATE_TEMP_TRIGGER: 5,
  CREATE_TEMP_VIEW: 6,
  CREATE_TRIGGER: 7,
  CREATE_VIEW: 8,
  DELETE: 9,
  DROP_INDEX: 10,
  DROP_TABLE: 11,
  DROP_TEMP_INDEX: 12,
  DROP_TEMP_TABLE: 13,
  DROP_TEMP_TRIGGER: 14,
  DROP_TEMP_VIEW: 15,
  DROP_TRIGGER: 16,
  DROP_VIEW: 17,
  INSERT: 18,
  PRAGMA: 19,
  READ: 20,
  SELECT: 21,
  TRANSACTION: 22,
  UPDATE: 23,
  ATTACH: 24,
  DETACH: 25,
  ALTER_TABLE: 26,
  REINDEX: 27,
  ANALYZE: 28,
  CREATE_VTABLE: 29,
  DROP_VTABLE: 30,
  FUNCTION: 31,
  SAVEPOINT: 32,
  RECURSIVE: 33,
};

module.exports = { Authorization, Action };
