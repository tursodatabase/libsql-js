export = SqliteError;
declare function SqliteError(message: any, code: any, rawCode: any): SqliteError;
declare class SqliteError {
    constructor(message: any, code: any, rawCode: any);
    code: string;
    rawCode: any;
    name: string;
}
//# sourceMappingURL=sqlite-error.d.ts.map