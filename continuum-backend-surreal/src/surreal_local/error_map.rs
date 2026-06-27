//! Map Surreal and serde errors to [`LogError`](continuum_core::error::LogError).

use continuum_core::error::LogError;

pub fn map_err(e: &surrealdb::Error) -> LogError {
    LogError::Backend(e.to_string())
}

pub fn map_serde(e: &serde_json::Error) -> LogError {
    LogError::Backend(e.to_string())
}

pub fn take_rows<T: serde::de::DeserializeOwned>(
    resp: &mut surrealdb::IndexedResults,
    index: usize,
) -> Result<Vec<T>, LogError> {
    let value: surrealdb::types::Value = resp.take(index).map_err(|e| map_err(&e))?;
    let json = value.into_json_value();
    match json {
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<T>, _>>()
            .map_err(|e| map_serde(&e)),
        serde_json::Value::Null => Ok(vec![]),
        one => Ok(vec![serde_json::from_value(one).map_err(|e| map_serde(&e))?]),
    }
}
