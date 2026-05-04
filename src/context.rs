use std::{path::PathBuf};

use azure_core::credentials::{TokenCredential, TokenRequestOptions as AzTokenRequestOptions};
use azure_identity::{AzureCliCredential, AzureCliCredentialOptions};
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;
use tokio::io::{self, AsyncReadExt};
use uuid::Uuid;
use crate::{app, app::AppContext, cli::ArgAuthType};

#[derive(Deserialize, Debug)]
#[serde(tag = "filter-type")]
pub enum FilterDto {
    #[serde(rename = "discrete", alias = "Discrete")]
    Discrete {
        #[serde(rename = "filter-name")]
        name: String,
        #[serde(flatten)]
        values: DiscreteValueDto,
    },
    #[serde(rename = "static", alias = "Static")]
    Static {
        #[serde(rename = "filter-name")]
        name: String,
        #[serde(flatten)]
        value: StaticValueDto,
    },
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum DiscreteValueDto {
    #[serde(rename = "filter-value-str")]
    String { #[serde(rename = "filter-value-str")] values: Vec<String> },
    #[serde(rename = "filter-value-dec")]
    Decimal { #[serde(rename = "filter-value-dec")] values: Vec<f64> },
    #[serde(rename = "filter-value-date")]
    Date { #[serde(rename = "filter-value-date")] values: Vec<NaiveDate> },
    #[serde(rename = "filter-value-datetime")]
    DateTime { #[serde(rename = "filter-value-datetime")] values: Vec<NaiveDateTime> },
    #[serde(rename = "filter-value-uuid")]
    Uuid { #[serde(rename = "filter-value-uuid")] values: Vec<Uuid> },
    Unknown(serde_json::Value)
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum StaticValueDto {
    #[serde(rename = "filter-value-str")]
    String { #[serde(rename = "filter-value-str")] value: String },
    #[serde(rename = "filter-value-dec")]
    Decimal { #[serde(rename = "filter-value-dec")] value: f64 },
    #[serde(rename = "filter-value-date")]
    Date { #[serde(rename = "filter-value-date")] value: NaiveDate },
    #[serde(rename = "filter-value-datetime")]
    DateTime { #[serde(rename = "filter-value-datetime")] value: NaiveDateTime },
    #[serde(rename = "filter-value-uuid")]
    Uuid { #[serde(rename = "filter-value-uuid")] value: Uuid },
    Unknown(serde_json::Value)
}

async fn validate_and_open_file(input_name: &str, path: &PathBuf, max_size_mb: u64) -> Result<tokio::fs::File, Box<dyn std::error::Error>> {

    let file_name= path
        .file_name()
        .ok_or_else(|| format!("Could not get the file name for the input \"{}\"", input_name))?.to_string_lossy();

    let input_file = tokio::fs::File::open(path).await?;
    let metadata = input_file.metadata().await?;

    if !metadata.is_file() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("The input \"{}\" with path \"{}\" is not a file", input_name, file_name))))
    }

    if metadata.len() > max_size_mb * 1024 * 1024 {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::FileTooLarge,
            format!("The input \"{}\" with path \"{}\" is too large", input_name, file_name))))
    }

    Ok(input_file)
}

async fn get_sql_query(file: &mut tokio::fs::File) -> Result<String, Box<dyn std::error::Error>> {

    let mut sql_query = String::new();
    file.read_to_string(&mut sql_query).await?;
    return Ok(sql_query);
}

fn map_static_value(static_value: StaticValueDto, position: u32) -> Result<app::StaticValue, Box<dyn std::error::Error>> {
    let result: app::StaticValue = match static_value {
        StaticValueDto::String { value} => app::StaticValue::String(value),
        StaticValueDto::Decimal { value } => app::StaticValue::Decimal(value),
        StaticValueDto::Date { value } => app::StaticValue::Date(value),
        StaticValueDto::DateTime { value } => app::StaticValue::DateTime(value),
        StaticValueDto::Uuid { value } => app::StaticValue::Uuid(value),
        StaticValueDto::Unknown(v) => return Err(format!("Could not parse static filter value at position {} {:?}", position, v).into()),
        _ => return Err(format!("Could not parse static filter value at {}", position).into())
    };

    Ok(result)
}

fn map_discrete_value(discrete_value: DiscreteValueDto, position: u32) -> Result<app::DiscreteValue, Box<dyn std::error::Error>> {
        let result: app::DiscreteValue = match discrete_value {
        DiscreteValueDto::String { values } => app::DiscreteValue::String(values),
        DiscreteValueDto::Decimal { values } => app::DiscreteValue::Decimal(values),
        DiscreteValueDto::Date { values } => app::DiscreteValue::Date(values),
        DiscreteValueDto::DateTime { values } => app::DiscreteValue::DateTime(values),
        DiscreteValueDto::Uuid { values } => app::DiscreteValue::Uuid(values),
        DiscreteValueDto::Unknown(v) => return Err(format!("Could not parse discrete filter value at position {} {:?}", position, v).into()),
        _ => return Err(format!("Could not parse discrete filter value at position {}", position).into())
    };

    Ok(result)
}


async fn validate_and_get_filters(filters_file: &mut tokio::fs::File) -> Result<Vec<app::Filter>, Box<dyn std::error::Error>> {
    
    let mut filters_json = String::new();
    filters_file.read_to_string(&mut filters_json).await?;
    let filters_dto: Vec<FilterDto> = serde_json::from_str(filters_json.as_str())?;

    let mut result = vec![];
    if filters_dto.is_empty() {
        return Ok(result);
    }

    let mut pos = 0u32;
    let mut count_discrete = 0u32;
    for filter in filters_dto {
        pos += 1;

        match filter {
            FilterDto::Discrete { name, values } => {
                count_discrete += 1;
                if count_discrete > 1 {
                    return Err("Multiple discrete filters are not allowed".into())
                }
                
                result.push(app::Filter::Discrete { name, values: map_discrete_value(values, pos)? });
            },

            FilterDto::Static { name, value } => {
                result.push(app::Filter::Static { name, value: map_static_value(value, pos)? });
            }
        }
    }

    Ok(result)
}

async fn get_az_cli_token() -> Result<String, Box<dyn std::error::Error>> {

    // TODO: add tenant_id as an optional cli arg
    let options = AzureCliCredentialOptions {
        // tenant_id: Some(az_tenant_id),
        ..Default::default()
    };

    let az_creds = AzureCliCredential::new(options.into())?;
    const DB_SCOPE: &str = "https://database.windows.net/.default";
    let token_response = az_creds
        .get_token(&[DB_SCOPE], AzTokenRequestOptions::default().into())
        .await?;
    
    Ok(token_response.token.secret().to_string())
}

pub async fn from_cli_args(value: crate::cli::CliArgs) -> Result<AppContext, Box<dyn std::error::Error>> {

    // TODO: define max_sql_file, max_filters_file, max_conn_string as cli args and apply good defaults
    let mut sql_file = validate_and_open_file("sql file", &value.sql_file, 10).await?;
    let sql_query = get_sql_query(&mut sql_file).await?;

    let mut app_filters: Vec<app::Filter> = vec![];
    if let Some(filters_path) = value.filters_file {
        let mut filters_file = validate_and_open_file("filters file", &filters_path, 100).await?;
        app_filters = validate_and_get_filters(&mut filters_file).await?;
    }

    let auth_type = match value.auth {
        ArgAuthType::UseAzCliToken => app::AuthType::AzCliToken(get_az_cli_token().await?),
        ArgAuthType::UseConnectionString(path) => {
            let mut conn_string_file = validate_and_open_file("connection string", &path, 1).await?;
            let mut conn_string = String::new();

            conn_string_file.read_to_string(&mut conn_string).await?;
            app::AuthType::ConnectionString(conn_string)
        }
    };
    
    Ok(AppContext {
        query: sql_query,
        auth_type: auth_type,
        filters: app_filters
    })
}

/*
[
    { // MAX 1 discrete filter.
        // The resulting report will have a tab for each value,
        // if there is a value with length bigger than the
        // tab max length the tabs will be a sequence 1, 2, 3...

        "filter-type": "discrete",
        "filter-name": "@period",
        "filter-value-str": ["01/2026", "02/2026", "03/2026"]
    },
    { // Multiple static filters are allowed
    
        "filter-type": "static",
        "filter-name": "@document_type",
        "filter-value-str": "CreditNote"
    }
]
*/


