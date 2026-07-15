use azure_core::Error;
use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;

pub mod sql_readonly_validator;

#[derive(Debug)]
pub enum AuthType {
    AzCliToken(String),
    ConnectionString(String),
}

#[derive(Debug)]
pub enum Filter {
    Static { name: String, value: StaticValue },
    Discrete { name: String, values: DiscreteValue }
}

#[derive(Debug)]
pub enum DiscreteValue {
    String(Vec<String>),
    Decimal(Vec<f64>),
    Date(Vec<NaiveDate>),
    DateTime(Vec<NaiveDateTime>),
    Uuid(Vec<Uuid>),
}

#[derive(Debug)]
pub enum StaticValue {
    String(String),
    Decimal(f64),
    Date(NaiveDate),
    DateTime(NaiveDateTime),
    Uuid(Uuid),
}

#[derive(Debug)]
pub struct AppContext {
    pub auth_type: AuthType,
    pub query: String,
    pub filters: Vec<Filter>
}

pub fn execute_report(ctx: &AppContext) -> Result<(), Box<dyn std::error::Error>> {

    sql_readonly_validator::validate_readonly_except_temp_tables(&ctx.query);
    // Connect to DB
    // Apply static filter if any
    // If there are discrete filters:
    //....For each discrete filter
    //.........> execute query and save the results in an excel file
    // else:
    // execute query and save the results in an excel file
    // print when the report is done and where the reports were saved
    // and how much time took each report to run
    todo!()
}