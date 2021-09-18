use std::rc::Rc;

use anyhow::{Context, Result};
use sea_schema::{
    postgres::{def::ColumnInfo, discovery::SchemaDiscovery},
    sea_query::Alias,
};
use sqlx::PgPool;

pub async fn discover_table_columns(
    uri: &str,
    schema: &str,
    table: &str,
) -> Result<Vec<ColumnInfo>> {
    let pool = PgPool::connect(uri)
        .await
        .context("could not connect to database")?;

    let schema_discovery = SchemaDiscovery::new(pool, schema);
    let columns = schema_discovery
        .discover_columns(Rc::new(Alias::new(schema)), Rc::new(Alias::new(table)))
        .await;

    Ok(columns)
}
