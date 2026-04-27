

pub enum AuthType {
    AzCliToken(String),
    ConnectionString(String),
}

pub enum Filter {
    Static { name: String, value: String },
    Discrete { name: String, values: Vec<String> }
}

pub struct AppContext {
    auth_type: AuthType,
    query: String,
    filters: Vec<Filter>
}








