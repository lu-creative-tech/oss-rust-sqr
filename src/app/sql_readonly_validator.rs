use sqlparser::ast::{
    CreateTable, CreateIndex, Delete, Insert, ObjectName, ObjectType, SetExpr, Statement, TableObject,
};
use sqlparser::dialect::MsSqlDialect;
use sqlparser::parser::{Parser, ParserError};
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    CreateTableNotTemp,
    DropTableNotTemp,
    CreateIndexNotTemp,
    InsertNotTemp,
    IntoClauseNotTemp,
    DeleteNotAllowed(String),
    ParseError(ParserError),
    GenericError(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::CreateTableNotTemp => {
                write!(f, "Only temporary tables can be created")
            }
            ValidationError::DropTableNotTemp => {
                write!(f, "Only temporary tables can be dropped")
            }
            ValidationError::CreateIndexNotTemp => {
                write!(f, "CREATE INDEX allowed only on temporary tables")
            }
            ValidationError::InsertNotTemp => {
                write!(f, "INSERT allowed only on temporary tables")
            }
            ValidationError::IntoClauseNotTemp => {
                write!(f, "SELECT INTO allowed only on temporary tables")
            }
            ValidationError::ParseError(e) => {
                write!(f, "SQL parse error: {e}")
            }
            ValidationError::DeleteNotAllowed(msg) => {
                write!(f, "{msg}")
            }
            ValidationError::GenericError(msg) => {
                write!(f, "{msg}")
            }
        }
    }
}

impl Error for ValidationError {}

impl From<Box<dyn std::error::Error>> for ValidationError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        ValidationError::GenericError(err.to_string())
    }
}

impl From<ParserError> for ValidationError {
    fn from(err: ParserError) -> Self {
        ValidationError::ParseError(err)
    }
}

pub fn validate_readonly_except_temp_tables(sql: &str) -> Result<(), ValidationError> {
    let dialect = MsSqlDialect {};

    let ast = Parser::parse_sql(&dialect, sql)?;

    for statement in ast {
        match statement {
            Statement::Query(query) => {
                if is_query_allowed(&*query.body)? {
                    continue;
                }
                return Err(ValidationError::CreateTableNotTemp);
            }
            Statement::Comment { .. } => continue,
            Statement::CreateTable(CreateTable { ref name, .. }) => {
                if is_temp_table_name(name)? {
                    continue; // Allow creation of temp tables
                }
                return Err(ValidationError::CreateTableNotTemp);
            }
            Statement::Drop {
                object_type,
                names,
                ..
            } => {
                if matches!(object_type, ObjectType::Table)
                    && names.len() == 1
                    && is_temp_table_name_opt(names.get(0))?
                {
                    continue; // Allow drop table on temp tables
                }
                return Err(ValidationError::DropTableNotTemp);
            }
            Statement::Insert(ref insert) => {
                if is_insert_allowed(insert)? {
                    continue;
                }
                return Err(ValidationError::InsertNotTemp);
            }
            Statement::Delete(ref delete) => {
                if is_delete_allowed(delete)? {
                    continue;
                }

                return Err(ValidationError::DeleteNotAllowed("Only temporary tables can be deleted".into()));
            }
            // TODO: add more tests, mix create SP, alter table, etc...
            // TODO: add insert, update, delete, also try to include tables as variables @table_foo(a int, b int)
            Statement::CreateIndex(CreateIndex { ref table_name, .. }) => {
                if is_temp_table_name(table_name)? {
                    continue; // Allow creating indexes on temp tables
                }
                return Err(ValidationError::CreateIndexNotTemp);
            }
            _ => return Err(ValidationError::GenericError("Query not allowed: it must be a readonly query; write operation are only allowed on temp tables; creation of procedure are not allowed".into()))
        }
    }

    Ok(())
}

fn is_temp_table_name(name: &ObjectName) -> Result<bool, ValidationError> {
    Ok(name.to_string().starts_with('#'))
}

fn is_temp_table_name_opt(name_opt: Option<&ObjectName>) -> Result<bool, ValidationError> {
    let name = name_opt.ok_or_else(|| {
        ValidationError::GenericError("Incorrect table name".to_string())
    })?;
    Ok(name.to_string().starts_with('#'))
}

fn is_query_allowed(query_body: &SetExpr) -> Result<bool, ValidationError> {
    match query_body {
        SetExpr::Select(select) => {
            if let Some(ref select_into) = select.into {
                if is_temp_table_name(&select_into.name)? {
                    return Ok(true); // Allow "select .. into" on temp tables
                }
                return Err(ValidationError::IntoClauseNotTemp);
            }
            Ok(true)
        }
        _ => Ok(true),
    }
}

fn is_insert_allowed(insert: &Insert) -> Result<bool, ValidationError> {
    match insert.table {
        TableObject::TableName(ref name) => {
            if is_temp_table_name(name)? {
                return Ok(true); // Allow inserts on temp tables
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn is_delete_allowed(delete: &Delete) -> Result<bool, ValidationError> {
    if !delete.tables.is_empty() {
        return Err(ValidationError::DeleteNotAllowed(
            "Syntax rejected: Only 'DELETE FROM #table' syntax is allowed."
                .to_string(),
        ));
    }

    let table_with_joins = match &delete.from {
        sqlparser::ast::FromTable::WithFromKeyword(list) => list,
        _ => {
            return Err(ValidationError::DeleteNotAllowed(
                "Syntax rejected: The 'FROM' keyword is required."
                    .to_string(),
            ));
        }
    };

    if table_with_joins.len() != 1 {
        return Err(ValidationError::DeleteNotAllowed(
            "Syntax rejected: Only a single table source is allowed."
                .to_string(),
        ));
    }

    let from = &table_with_joins[0];
    if !from.joins.is_empty() {
        return Err(ValidationError::DeleteNotAllowed(
            "Syntax rejected: Joins are not allowed."
                .to_string(),
        ));
    }

    if let sqlparser::ast::TableFactor::Table { name, alias, .. } = &from.relation {
        if alias.is_some() {
            return Err(ValidationError::DeleteNotAllowed(
                "Syntax rejected: Table aliases are not allowed."
                    .to_string(),
            ));
        }

        return is_temp_table_name(name);
    }

    return Err(ValidationError::DeleteNotAllowed(
                "Syntax rejected: Only 'DELETE FROM #table' syntax is allowed."
                    .to_string(),
            ));
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Allowed (returns Ok) ---

    #[test]
    fn test_select_is_allowed() {
        let result = validate_readonly_except_temp_tables("SELECT * FROM users");
        assert!(result.is_ok(), "Expected validation to succeed");
    }

    #[test]
    fn test_select_with_join_is_allowed() {
        let sql = "SELECT u.name, o.total FROM users u JOIN orders o ON o.user_id = u.id";
        assert!(validate_readonly_except_temp_tables(sql).is_ok(), "Expected validation to succeed");
    }

    #[test]
    fn test_create_temp_table_is_allowed() {
        let sql = "CREATE TABLE #tmp (id INT, name VARCHAR(100))";
        assert!(validate_readonly_except_temp_tables(sql).is_ok(), "Expected validation to succeed");
    }

    #[test]
    fn test_drop_temp_table_is_allowed() {
        let sql = "DROP TABLE #tmp";
        assert!(validate_readonly_except_temp_tables(sql).is_ok(), "Expected validation to succeed");
    }

    #[test]
    fn test_insert_into_temp_table_is_allowed() {
        let sql = "INSERT INTO #tmp (id, name) VALUES (1, 'hello')";
        assert!(validate_readonly_except_temp_tables(sql).is_ok(), "Expected validation to succeed");
    }

    #[test]
    fn test_select_into_temp_table_is_allowed() {
        let sql = "SELECT * INTO #dest FROM users";
        assert!(validate_readonly_except_temp_tables(sql).is_ok(), "Expected validation to succeed");
    }

    #[test]
    fn test_multiple_statements_all_allowed() {
        let sql = r#"
            CREATE TABLE #tmp (id INT);
            INSERT INTO #tmp VALUES (1);
            SELECT * FROM #tmp;
            DROP TABLE #tmp;
        "#;
        assert!(validate_readonly_except_temp_tables(sql).is_ok(), "Expected validation to succeed");
    }

    #[test]
    fn test_create_index_on_temp_table_is_allowed() {
        let sql = "CREATE INDEX idx_tmp_id ON #tmp (id)";
        assert!(validate_readonly_except_temp_tables(sql).is_ok(), "Expected validation to succeed");
    }

    #[test]
    fn test_create_unique_index_on_temp_table_is_allowed() {
        let sql = "CREATE UNIQUE INDEX idx_tmp_id ON #tmp (id)";
        assert!(validate_readonly_except_temp_tables(sql).is_ok(), "Expected validation to succeed");
    }

    // --- Rejected (returns Err with specific variant) ---

    #[test]
    fn test_create_index_on_non_temp_table_is_rejected() {
        let sql = "CREATE INDEX idx_users_id ON users (id)";
        let result = validate_readonly_except_temp_tables(sql);

        match result {
            Err(e) => assert_eq!(e, ValidationError::CreateIndexNotTemp),
            _ => panic!("ValidationError::CreateIndexNotTemp expected!")
        }
    }

    #[test]
    fn test_create_stored_procedure_is_rejected() {
        let sql = "CREATE PROCEDURE sp_name AS BEGIN SELECT 1 END";
        let result = validate_readonly_except_temp_tables(sql);

        assert!(result.is_err(), "ValidationError expected!");
    }

    #[test]
    fn test_create_index_on_temp_and_stored_procedure_returns_err() {
        let sql = r#"
            CREATE TABLE #tmp (id INT);
            CREATE INDEX idx_tmp_id ON #tmp (id);
            CREATE PROCEDURE sp_test AS BEGIN SELECT * FROM #tmp END
        "#;
        let result = validate_readonly_except_temp_tables(sql);
        
        assert!(result.is_err(), "ValidationError expected!");
    }

    #[test]
    fn test_create_non_temp_table_is_rejected() {
        let sql = "CREATE TABLE users (id INT)";
        let result = validate_readonly_except_temp_tables(sql);

        match result {
            Err(e) => assert_eq!(e, ValidationError::CreateTableNotTemp),
            _ => panic!("ValidationError::CreateTableNotTemp expected!")
        }
    }

    #[test]
    fn test_drop_non_temp_table_is_rejected() {
        let sql = "DROP TABLE users";
        let result = validate_readonly_except_temp_tables(sql);
        
        match result {
            Err(e) => assert_eq!(e, ValidationError::DropTableNotTemp),
            _ => panic!("ValidationError::DropTableNotTemp expected!")
        }
    }

    #[test]
    fn test_insert_into_non_temp_table_is_rejected() {
        let sql = "INSERT INTO users (id) VALUES (1)";
        let result = validate_readonly_except_temp_tables(sql);

        match result {
            Err(e) => assert_eq!(e, ValidationError::InsertNotTemp),
            _ => panic!("ValidationError::InsertNotTemp expected!")
        }
    }

    #[test]
    fn test_select_into_non_temp_table_is_rejected() {
        let sql = "SELECT * INTO users_archive FROM users";
        let result = validate_readonly_except_temp_tables(sql);

        match result {
            Err(e) => assert_eq!(e, ValidationError::IntoClauseNotTemp),
            _ => panic!("ValidationError::IntoClauseNotTemp expected!")
        }
    }

    #[test]
    fn test_drop_temp_table_without_hash_is_rejected() {
        let sql = "DROP TABLE tmp";
        let result = validate_readonly_except_temp_tables(sql);

        match result {
            Err(e) => assert_eq!(e, ValidationError::DropTableNotTemp),
            _ => panic!("ValidationError::DropTableNotTemp expected!")
        }
    }

    #[test]
    fn test_create_index_on_non_hash_table_is_rejected() {
        let sql = "CREATE INDEX idx_tmp_id ON tmp (id)";
        let result = validate_readonly_except_temp_tables(sql);

        match result {
            Err(e) => assert_eq!(e, ValidationError::CreateIndexNotTemp),
            _ => panic!("ValidationError::CreateIndexNotTemp expected!")
        }
    }

    #[test]
    fn test_mixed_allowed_and_rejected_returns_err() {
        let sql = "SELECT * FROM users; INSERT INTO real_table VALUES (1)";
        let result = validate_readonly_except_temp_tables(sql);

        assert!(result.is_err(), "ValidationError expected!");
    }

    // --- Parse errors ---

    #[test]
    fn test_invalid_sql_returns_parse_error() {
        let sql = "SELEC * FROM";
        let result = validate_readonly_except_temp_tables(sql);
        
        match result {
            Err(ValidationError::ParseError(_)) => {},
            other => panic!("expected ParseError, got {other:?}")
        }
    }
}


