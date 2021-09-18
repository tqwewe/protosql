use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::Clap;
use colorful::Colorful;
use commands::Protosql;
use heck::CamelCase;
use protobuf_parser::{parse, AbstractProto, FieldType, Message, RuleVariant};
use sea_schema::postgres::def::{ColumnInfo, ColumnType};
use tokio::fs::ReadDir;

use crate::log::*;

mod commands;
mod log;
mod schema;

#[tokio::main]
async fn main() {
    let opts: Protosql = Protosql::parse();
    let level = if opts.verbose {
        Level::Debug
    } else if opts.quiet {
        Level::Warn
    } else {
        Level::Info
    };
    SimpleLogger::new().with_level(level).init().unwrap();

    if let Err(err) = try_main(opts).await {
        error!("{}", err);
        if level == Level::Debug {
            err.chain()
                .skip(1)
                .for_each(|cause| eprintln!("   {}", format!("- {}", cause).dark_gray()));
        }
        std::process::exit(1);
    }
}

async fn try_main(opts: Protosql) -> Result<()> {
    if let Some(dir) = &opts.dir {
        let mut dirs = read_proto_dir(dir).await?;
        while let Some(entry) = dirs.next_entry().await? {
            let file = entry.path();
            if !verify_file(&file, &opts).await? {
                error!("found mismatch in schemas");
                std::process::exit(2);
            } else {
                info!(
                    "{}",
                    format!("{} is valid", file.file_name().unwrap().to_string_lossy()).bold()
                );
            }
            println!();
        }
    } else if let Some(file) = &opts.file {
        if !verify_file(file, &opts).await? {
            error!("found mismatch in schemas");
            std::process::exit(2);
        } else {
            let path: &Path = file.as_ref();
            info!(
                "{}",
                format!("{} is valid", path.file_name().unwrap().to_string_lossy()).bold()
            );
        }
    } else {
        error!("no --file or --dir specified");
        std::process::exit(1);
    }

    Ok(())
}

async fn verify_file(path: impl AsRef<Path>, opts: &Protosql) -> Result<bool> {
    // Open the proto file
    let file_name: &Path = path.as_ref();
    let file = tokio::fs::read_to_string(&path)
        .await
        .context("could not read proto file")?;
    let (_, proto) = parse(&file).map_err(|_| anyhow!("could not parse proto file"))?;
    info!("loaded proto file '{}'", file_name.to_str().unwrap());

    let message_name = opts.message.clone().unwrap_or_else(|| {
        let file_name = file_name.file_name().unwrap().to_string_lossy();
        let message_name = file_name.split('.').next().unwrap().to_camel_case();
        if opts.dir.is_none() {
            info!(
                "--message not specified, assuming message '{}'",
                message_name
            );
        }
        message_name
    });
    let message = find_proto_message(&proto, &message_name)?;
    info!("found message '{}'", message_name);

    let table_name = opts.table.clone().unwrap_or_else(|| {
        let file_name = file_name.file_name().unwrap().to_string_lossy();
        let table_name = file_name.split('.').next().unwrap();
        if opts.dir.is_none() {
            info!("--table not specified, assuming table '{}'", table_name);
        }
        table_name.to_string()
    });
    let table_columns =
        schema::discover_table_columns(&opts.uri, &opts.schema, &table_name).await?;
    info!("connected to database");

    if table_columns.is_empty() {
        warn!("table {}.{} has no columns", opts.schema, table_name);
        std::process::exit(2);
    }
    info!(
        "found {} columns on table {}.{}",
        table_columns.len(),
        opts.schema,
        table_name
    );

    Ok(verify_message_with_columns(&message, &table_columns))
}

// async fn load_proto_file(path: impl AsRef<Path>) -> Result<AbstractProto> {
//     let file = tokio::fs::read_to_string(path)
//         .await
//         .context("could not read proto file")?;
//     let (_, abstract_proto) = parse(file).map_err(|_| anyhow!("could not parse proto file"))?;
//     // let proto_file =
//     //     FileDescriptor::parse(file).map_err(|_| anyhow!("could not parse proto file"))?;
//     Ok(abstract_proto)
// }

async fn read_proto_dir(path: impl AsRef<Path>) -> Result<ReadDir> {
    let dir = tokio::fs::read_dir(path)
        .await
        .context("could not read protos directory")?;
    Ok(dir)
}

fn find_proto_message<'a>(proto: &'a AbstractProto, message_name: &str) -> Result<Message<'a>> {
    proto
        .messages
        .iter()
        .find(|message| {
            message
                .name
                .as_ref()
                .map(|name| name.as_ref() == message_name)
                .unwrap_or(false)
        })
        .cloned()
        .ok_or_else(|| anyhow!("could not find message {}", message_name))
}

fn verify_message_with_columns(message: &Message, table_columns: &[ColumnInfo]) -> bool {
    // let max_items = message.fields.len().max(table_columns.len());
    let mut success = true;

    for proto_field in &message.fields {
        // println!("{:#?}", proto_field);
        let table_field = match table_columns
            .iter()
            .find(|col| col.name == proto_field.name.as_ref())
        {
            Some(col) => col,
            None => {
                success = false;
                warn!(
                    "missing field in database table: {} {}",
                    proto_field.name.as_ref().bold(),
                    format!("{:?}", proto_field.typ).dark_gray()
                );
                continue;
            }
        };

        // Verify types
        if matches!(
            proto_field.rule.variant,
            protobuf_parser::RuleVariant::Repeated
        ) {
            if !matches!(table_field.col_type, ColumnType::Array) {
                success = false;
                warn!(
                    "field '{}' is repeated, but database type is not an array",
                    proto_field.name.as_ref()
                );
                continue;
            }
        } else {
            let valid_type = match &proto_field.typ {
                FieldType::Int32 => matches!(table_field.col_type, ColumnType::Integer),
                FieldType::Int64 => matches!(table_field.col_type, ColumnType::BigInt),
                FieldType::Uint32 => matches!(table_field.col_type, ColumnType::Integer),
                FieldType::Uint64 => matches!(table_field.col_type, ColumnType::BigInt),
                FieldType::Sint32 => matches!(table_field.col_type, ColumnType::Integer),
                FieldType::Sint64 => matches!(table_field.col_type, ColumnType::BigInt),
                FieldType::Bool => matches!(table_field.col_type, ColumnType::Boolean),
                FieldType::Fixed64 => matches!(table_field.col_type, ColumnType::BigInt),
                FieldType::Sfixed64 => matches!(table_field.col_type, ColumnType::BigInt),
                FieldType::Double => matches!(table_field.col_type, ColumnType::DoublePrecision),
                FieldType::String => matches!(
                    table_field.col_type,
                    ColumnType::Varchar(_) | ColumnType::Uuid
                ),
                FieldType::Bytes => matches!(table_field.col_type, ColumnType::Bytea),
                FieldType::Fixed32 => matches!(table_field.col_type, ColumnType::Integer),
                FieldType::Sfixed32 => matches!(table_field.col_type, ColumnType::Integer),
                FieldType::Float => matches!(table_field.col_type, ColumnType::Real),
                FieldType::MessageOrEnum(name) => match name.as_ref() {
                    "google.protobuf.Timestamp" => matches!(
                        table_field.col_type,
                        ColumnType::Timestamp(_) | ColumnType::TimestampWithTimeZone(_)
                    ),
                    _ => {
                        warn!(
                            "unknown type '{}' on field '{}'",
                            name.as_ref(),
                            proto_field.name.as_ref()
                        );
                        false
                    }
                },
                FieldType::Map(_) => {
                    warn!(
                        "protobuf maps are not supported on field '{}'",
                        proto_field.name.as_ref()
                    );
                    false
                }
                FieldType::Group(_) => {
                    warn!(
                        "protobuf groups are not supported on field '{}'",
                        proto_field.name.as_ref()
                    );
                    false
                }
            };
            if !valid_type {
                success = false;
                warn!(
                    "field '{}' has type '{:?}' which not match database type '{:?}'",
                    proto_field.name.as_ref(),
                    proto_field.typ,
                    table_field.col_type
                );
                continue;
            }
        }

        // Verify nullable
        let field_optional = proto_field.rule.variant == RuleVariant::Optional
            && proto_field.rule.position.is_some();
        let column_optional = table_field.not_null.is_none();
        if field_optional && !column_optional {
            success = false;
            warn!(
                "field '{}' is marked as {} in database, but should be {}",
                table_field.name,
                "NOT NULL".bold(),
                "NULL".bold()
            );
        } else if !field_optional && column_optional {
            success = false;
            warn!(
                "field '{}' is marked as {} in database, but should be {}",
                table_field.name,
                "NULL".bold(),
                "NOT NULL".bold()
            );
        }
    }

    for table_column in table_columns {
        if !message
            .fields
            .iter()
            .any(|field| field.name.as_ref() == table_column.name)
        {
            success = false;
            let field_null_str = if table_column.not_null.is_some() {
                "nullable=false"
            } else {
                "nullable=true"
            };
            let field_default_string = if let Some(def) = &table_column.default {
                format!("default={}", def.0)
            } else {
                String::new()
            };
            warn!(
                "unknown field in database table: {} {}",
                table_column.name.clone().bold(),
                format!(
                    "{}, {}, {}",
                    format!("{:?}", table_column.col_type)
                        .split('(')
                        .next()
                        .unwrap(),
                    field_null_str,
                    field_default_string
                )
                .dark_gray()
            );
        }
    }

    success
}
