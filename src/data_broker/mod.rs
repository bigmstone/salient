mod lang;

use std::error::Error;

use {
    arrow::array::{
        Float32Array, Int32Array, Int8Array, ListArray, RecordBatch,
        {types::Int32Type, Array},
    },
    datafusion::{dataframe::DataFrameWriteOptions, prelude::*},
    log::info,
};

struct Storage {}

struct DataBroker {}

impl DataBroker {
    async fn new(store: &str) -> Result<Self, Box<dyn Error>> {
        let ctx = SessionContext::new();

        let sql = r#"CREATE EXTERNAL TABLE IF NOT EXISTS test (
    k INT PRIMARY KEY NOT NULL,
    v VARCHAR NOT NULL
) STORED AS PARQUET LOCATION './store/test/';"#;
        ctx.sql(sql).await?;
        let sql = "INSERT INTO test (k, v) VALUES (1, 'value1'), (2, 'value2');";
        let execution_plan = ctx.sql(sql).await?.create_physical_plan().await?;

        ctx.write_parquet(execution_plan, "./store/test", None)
            .await?;

        let table_df = ctx.table("test").await.unwrap();
        println!("TABLE SCHEMA: {:?}", table_df.schema());

        Ok(Self {})
    }
}

struct ChatSession {
    id: u64,          // Row ID/Offset
    session_id: u128, // Unique session ID as UUID4 stored as binary to allow for indexing
    role: String,     // String of role type I.E. Assistant, User, System, etc. Maybe make enum?
    content: String,  // Message content
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn create_data_broker() {
        DataBroker::new("./store/test/").await.unwrap();
    }
}
