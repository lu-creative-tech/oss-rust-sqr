use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;


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
