use tracing::trace;

use std::collections::HashSet;

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

    pub fn build(self) -> Authorizer {
        Authorizer::new(self.allow_list, self.deny_list)
    }
}

pub struct Authorizer {
    allow_list: HashSet<String>,
    deny_list: HashSet<String>,
}

impl Authorizer {
    pub fn new(
        allow_list: HashSet<String>,
        deny_list: HashSet<String>,
    ) -> Self {
        Self {
            allow_list,
            deny_list,
        }
    }

    pub fn authorize(&self, ctx: &libsql::AuthContext) -> libsql::Authorization {
        use libsql::AuthAction;
        let ret = match ctx.action {
            AuthAction::Unknown { .. } => libsql::Authorization::Deny,
            AuthAction::CreateIndex { table_name, .. } => self.authorize_table(table_name),
            AuthAction::CreateTable { table_name, .. } => self.authorize_table(table_name),
            AuthAction::CreateTempIndex { table_name, .. } => self.authorize_table(table_name),
            AuthAction::CreateTempTable { table_name, .. } => self.authorize_table(table_name),
            AuthAction::CreateTempTrigger { table_name, .. } => self.authorize_table(table_name),
            AuthAction::CreateTempView { .. } => libsql::Authorization::Deny,
            AuthAction::CreateTrigger { table_name, .. } => self.authorize_table(table_name),
            AuthAction::CreateView { .. } => libsql::Authorization::Deny,
            AuthAction::Delete { table_name, .. } => self.authorize_table(table_name),
            AuthAction::DropIndex { table_name, .. } => self.authorize_table(table_name),
            AuthAction::DropTable { table_name, .. } => self.authorize_table(table_name),
            AuthAction::DropTempIndex { table_name, .. } => self.authorize_table(table_name),
            AuthAction::DropTempTable { table_name, .. } => self.authorize_table(table_name),
            AuthAction::DropTempTrigger { table_name, .. } => self.authorize_table(table_name),
            AuthAction::DropTempView { .. } => libsql::Authorization::Deny,
            AuthAction::DropTrigger { .. } => libsql::Authorization::Deny,
            AuthAction::DropView { .. } => libsql::Authorization::Deny,
            AuthAction::Insert { table_name, .. } => self.authorize_table(table_name),
            AuthAction::Pragma { .. } => libsql::Authorization::Deny,
            AuthAction::Read { table_name, .. } => self.authorize_table(table_name),
            AuthAction::Select { .. } => libsql::Authorization::Allow,
            AuthAction::Transaction { .. } => libsql::Authorization::Deny,
            AuthAction::Update { table_name, .. } => self.authorize_table(table_name),
            AuthAction::Attach { .. } => libsql::Authorization::Deny,
            AuthAction::Detach { .. } => libsql::Authorization::Deny,
            AuthAction::AlterTable { table_name, .. } => self.authorize_table(table_name),
            AuthAction::Reindex { .. } => libsql::Authorization::Deny,
            AuthAction::Analyze { .. } => libsql::Authorization::Deny,
            AuthAction::CreateVtable { .. } => libsql::Authorization::Deny,
            AuthAction::DropVtable { .. } => libsql::Authorization::Deny,
            AuthAction::Function { .. } => libsql::Authorization::Deny,
            AuthAction::Savepoint { .. } => libsql::Authorization::Deny,
            AuthAction::Recursive { .. } => libsql::Authorization::Deny,
        };
        trace!("authorize(ctx = {:?}) -> {:?}", ctx, ret);
        ret
    }

    fn authorize_table(&self, table: &str) -> libsql::Authorization {
        if self.deny_list.contains(table) {
            return libsql::Authorization::Deny;
        }
        if self.allow_list.contains(table) {
            return libsql::Authorization::Allow;
        }
        libsql::Authorization::Deny
    }
}
