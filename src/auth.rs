use libsql::{
    ffi::SQLITE_ALTER_TABLE, ffi::SQLITE_ANALYZE, ffi::SQLITE_ATTACH, ffi::SQLITE_COPY,
    ffi::SQLITE_CREATE_INDEX, ffi::SQLITE_CREATE_TABLE, ffi::SQLITE_CREATE_TEMP_INDEX,
    ffi::SQLITE_CREATE_TEMP_TABLE, ffi::SQLITE_CREATE_TEMP_TRIGGER, ffi::SQLITE_CREATE_TEMP_VIEW,
    ffi::SQLITE_CREATE_TRIGGER, ffi::SQLITE_CREATE_VIEW, ffi::SQLITE_CREATE_VTABLE,
    ffi::SQLITE_DELETE, ffi::SQLITE_DETACH, ffi::SQLITE_DROP_INDEX, ffi::SQLITE_DROP_TABLE,
    ffi::SQLITE_DROP_TEMP_INDEX, ffi::SQLITE_DROP_TEMP_TABLE, ffi::SQLITE_DROP_TEMP_TRIGGER,
    ffi::SQLITE_DROP_TEMP_VIEW, ffi::SQLITE_DROP_TRIGGER, ffi::SQLITE_DROP_VIEW,
    ffi::SQLITE_DROP_VTABLE, ffi::SQLITE_FUNCTION, ffi::SQLITE_INSERT, ffi::SQLITE_PRAGMA,
    ffi::SQLITE_READ, ffi::SQLITE_RECURSIVE, ffi::SQLITE_REINDEX, ffi::SQLITE_SAVEPOINT,
    ffi::SQLITE_SELECT, ffi::SQLITE_TRANSACTION, ffi::SQLITE_UPDATE, AuthAction,
};

use std::collections::HashSet;
use tracing::trace;

/// How a pattern matches against a string identifier.
pub enum PatternMatcher {
    /// Case-sensitive exact match.
    Exact(String),
    /// Glob pattern (supports `*` and `?` wildcards).
    Glob(String),
}

impl PatternMatcher {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            PatternMatcher::Exact(s) => s == value,
            PatternMatcher::Glob(pattern) => glob_match::glob_match(pattern, value),
        }
    }
}

/// Action info extraction
pub struct ActionInfo<'a> {
    pub code: i32,
    pub table_name: Option<&'a str>,
    pub column_name: Option<&'a str>,
    pub entity_name: Option<&'a str>,
}

pub fn extract_action_info<'a>(action: &'a libsql::AuthAction) -> ActionInfo<'a> {
    match action {
        AuthAction::Unknown { .. } => ActionInfo {
            code: SQLITE_COPY,
            table_name: None,
            column_name: None,
            entity_name: None,
        },
        AuthAction::CreateIndex {
            index_name,
            table_name,
        } => ActionInfo {
            code: SQLITE_CREATE_INDEX,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(index_name),
        },
        AuthAction::CreateTable { table_name } => ActionInfo {
            code: SQLITE_CREATE_TABLE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: None,
        },
        AuthAction::CreateTempIndex {
            index_name,
            table_name,
        } => ActionInfo {
            code: SQLITE_CREATE_TEMP_INDEX,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(index_name),
        },
        AuthAction::CreateTempTable { table_name } => ActionInfo {
            code: SQLITE_CREATE_TEMP_TABLE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: None,
        },
        AuthAction::CreateTempTrigger {
            trigger_name,
            table_name,
        } => ActionInfo {
            code: SQLITE_CREATE_TEMP_TRIGGER,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(trigger_name),
        },
        AuthAction::CreateTempView { view_name } => ActionInfo {
            code: SQLITE_CREATE_TEMP_VIEW,
            table_name: None,
            column_name: None,
            entity_name: Some(view_name),
        },
        AuthAction::CreateTrigger {
            trigger_name,
            table_name,
        } => ActionInfo {
            code: SQLITE_CREATE_TRIGGER,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(trigger_name),
        },
        AuthAction::CreateView { view_name } => ActionInfo {
            code: SQLITE_CREATE_VIEW,
            table_name: None,
            column_name: None,
            entity_name: Some(view_name),
        },
        AuthAction::Delete { table_name } => ActionInfo {
            code: SQLITE_DELETE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: None,
        },
        AuthAction::DropIndex {
            index_name,
            table_name,
        } => ActionInfo {
            code: SQLITE_DROP_INDEX,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(index_name),
        },
        AuthAction::DropTable { table_name } => ActionInfo {
            code: SQLITE_DROP_TABLE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: None,
        },
        AuthAction::DropTempIndex {
            index_name,
            table_name,
        } => ActionInfo {
            code: SQLITE_DROP_TEMP_INDEX,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(index_name),
        },
        AuthAction::DropTempTable { table_name } => ActionInfo {
            code: SQLITE_DROP_TEMP_TABLE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: None,
        },
        AuthAction::DropTempTrigger {
            trigger_name,
            table_name,
        } => ActionInfo {
            code: SQLITE_DROP_TEMP_TRIGGER,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(trigger_name),
        },
        AuthAction::DropTempView { view_name } => ActionInfo {
            code: SQLITE_DROP_TEMP_VIEW,
            table_name: None,
            column_name: None,
            entity_name: Some(view_name),
        },
        AuthAction::DropTrigger {
            trigger_name,
            table_name,
        } => ActionInfo {
            code: SQLITE_DROP_TRIGGER,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(trigger_name),
        },
        AuthAction::DropView { view_name } => ActionInfo {
            code: SQLITE_DROP_VIEW,
            table_name: None,
            column_name: None,
            entity_name: Some(view_name),
        },
        AuthAction::Insert { table_name } => ActionInfo {
            code: SQLITE_INSERT,
            table_name: Some(table_name),
            column_name: None,
            entity_name: None,
        },
        AuthAction::Pragma { pragma_name, .. } => ActionInfo {
            code: SQLITE_PRAGMA,
            table_name: None,
            column_name: None,
            entity_name: Some(pragma_name),
        },
        AuthAction::Read {
            table_name,
            column_name,
        } => ActionInfo {
            code: SQLITE_READ,
            table_name: Some(table_name),
            column_name: Some(column_name),
            entity_name: None,
        },
        AuthAction::Select => ActionInfo {
            code: SQLITE_SELECT,
            table_name: None,
            column_name: None,
            entity_name: None,
        },
        AuthAction::Transaction { .. } => ActionInfo {
            code: SQLITE_TRANSACTION,
            table_name: None,
            column_name: None,
            entity_name: None,
        },
        AuthAction::Update {
            table_name,
            column_name,
        } => ActionInfo {
            code: SQLITE_UPDATE,
            table_name: Some(table_name),
            column_name: Some(column_name),
            entity_name: None,
        },
        AuthAction::Attach { filename } => ActionInfo {
            code: SQLITE_ATTACH,
            table_name: None,
            column_name: None,
            entity_name: Some(filename),
        },
        AuthAction::Detach { database_name } => ActionInfo {
            code: SQLITE_DETACH,
            table_name: None,
            column_name: None,
            entity_name: Some(database_name),
        },
        AuthAction::AlterTable { table_name, .. } => ActionInfo {
            code: SQLITE_ALTER_TABLE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: None,
        },
        AuthAction::Reindex { index_name } => ActionInfo {
            code: SQLITE_REINDEX,
            table_name: None,
            column_name: None,
            entity_name: Some(index_name),
        },
        AuthAction::Analyze { table_name } => ActionInfo {
            code: SQLITE_ANALYZE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: None,
        },
        AuthAction::CreateVtable {
            table_name,
            module_name,
        } => ActionInfo {
            code: SQLITE_CREATE_VTABLE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(module_name),
        },
        AuthAction::DropVtable {
            table_name,
            module_name,
        } => ActionInfo {
            code: SQLITE_DROP_VTABLE,
            table_name: Some(table_name),
            column_name: None,
            entity_name: Some(module_name),
        },
        AuthAction::Function { function_name } => ActionInfo {
            code: SQLITE_FUNCTION,
            table_name: None,
            column_name: None,
            entity_name: Some(function_name),
        },
        AuthAction::Savepoint { savepoint_name, .. } => ActionInfo {
            code: SQLITE_SAVEPOINT,
            table_name: None,
            column_name: None,
            entity_name: Some(savepoint_name),
        },
        AuthAction::Recursive => ActionInfo {
            code: SQLITE_RECURSIVE,
            table_name: None,
            column_name: None,
            entity_name: None,
        },
    }
}

/// A single authorization rule.
pub struct AuthRule {
    /// Which action codes this rule applies to (empty = match all).
    pub actions: Vec<i32>,
    /// Table name matcher (None = match any table).
    pub table: Option<PatternMatcher>,
    /// Column name matcher (None = match any column).
    pub column: Option<PatternMatcher>,
    /// Generic entity name matcher for index/trigger/view/pragma/function names.
    pub entity: Option<PatternMatcher>,
    /// The authorization to return if this rule matches.
    pub authorization: libsql::Authorization,
}

impl AuthRule {
    fn matches(&self, info: &ActionInfo) -> bool {
        // Check action code
        if !self.actions.is_empty() && !self.actions.contains(&info.code) {
            return false;
        }
        // Check table pattern
        if let Some(ref pat) = self.table {
            match info.table_name {
                Some(name) => {
                    if !pat.matches(name) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        // Check column pattern
        if let Some(ref pat) = self.column {
            match info.column_name {
                Some(name) => {
                    if !pat.matches(name) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        // Check entity pattern
        if let Some(ref pat) = self.entity {
            match info.entity_name {
                Some(name) => {
                    if !pat.matches(name) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }
}

pub struct Authorizer {
    rules: Vec<AuthRule>,
    default: libsql::Authorization,
}

impl Authorizer {
    pub fn new(rules: Vec<AuthRule>, default: libsql::Authorization) -> Self {
        Self { rules, default }
    }

    pub fn authorize(&self, ctx: &libsql::AuthContext) -> libsql::Authorization {
        let info = extract_action_info(&ctx.action);
        for rule in &self.rules {
            if rule.matches(&info) {
                trace!(
                    "authorize(ctx = {:?}) -> {:?} (rule match)",
                    ctx,
                    rule.authorization
                );
                return rule.authorization;
            }
        }
        trace!("authorize(ctx = {:?}) -> {:?} (default)", ctx, self.default);
        self.default
    }
}

/// Legacy builder (backward compatibility)
pub struct AuthorizerBuilder {
    allow_list: HashSet<String>,
    deny_list: HashSet<String>,
}

impl AuthorizerBuilder {
    pub fn new() -> Self {
        Self {
            allow_list: HashSet::new(),
            deny_list: HashSet::new(),
        }
    }

    pub fn allow(&mut self, table: &str) -> &mut Self {
        self.allow_list.insert(table.to_string());
        self
    }

    pub fn deny(&mut self, table: &str) -> &mut Self {
        self.deny_list.insert(table.to_string());
        self
    }

    /// Converts the legacy allow/deny lists into an ordered rule set.
    ///
    /// Deny rules come first (higher priority), then allow rules.
    /// Default policy is Deny (same as the old behavior).
    pub fn build(self) -> Authorizer {
        let mut rules = Vec::new();

        // Table-bearing action codes (actions where the old authorizer checked tables)
        let table_actions: Vec<i32> = vec![
            SQLITE_CREATE_INDEX,
            SQLITE_CREATE_TABLE,
            SQLITE_CREATE_TEMP_INDEX,
            SQLITE_CREATE_TEMP_TABLE,
            SQLITE_CREATE_TEMP_TRIGGER,
            SQLITE_CREATE_TRIGGER,
            SQLITE_DELETE,
            SQLITE_DROP_INDEX,
            SQLITE_DROP_TABLE,
            SQLITE_DROP_TEMP_INDEX,
            SQLITE_DROP_TEMP_TABLE,
            SQLITE_DROP_TEMP_TRIGGER,
            SQLITE_DROP_TRIGGER,
            SQLITE_INSERT,
            SQLITE_READ,
            SQLITE_UPDATE,
            SQLITE_ALTER_TABLE,
            SQLITE_CREATE_VTABLE,
            SQLITE_DROP_VTABLE,
        ];

        // Deny rules first
        for table in &self.deny_list {
            rules.push(AuthRule {
                actions: table_actions.clone(),
                table: Some(PatternMatcher::Exact(table.clone())),
                column: None,
                entity: None,
                authorization: libsql::Authorization::Deny,
            });
        }

        // Then allow rules
        for table in &self.allow_list {
            rules.push(AuthRule {
                actions: table_actions.clone(),
                table: Some(PatternMatcher::Exact(table.clone())),
                column: None,
                entity: None,
                authorization: libsql::Authorization::Allow,
            });
        }

        // Legacy behavior: always allow SELECT (no table context)
        rules.push(AuthRule {
            actions: vec![SQLITE_SELECT],
            table: None,
            column: None,
            entity: None,
            authorization: libsql::Authorization::Allow,
        });

        // Everything else denies by default (same as old behavior)
        Authorizer::new(rules, libsql::Authorization::Deny)
    }
}
