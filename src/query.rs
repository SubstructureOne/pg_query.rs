use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use prost::Message;

use crate::*;
use crate::bindings::*;
use crate::error::*;

/// Represents the resulting fingerprint containing both the raw integer form as well as the
/// corresponding 16 character hex value.
pub struct Fingerprint {
    pub value: u64,
    pub hex: String,
}

/// Parses the given SQL statement into the given abstract syntax tree.
///
/// # Example
///
/// ```rust
/// use pg_query::{Node, Nodes};
///
/// let result = pg_query::parse("SELECT * FROM contacts");
/// assert!(result.is_ok());
/// let result = result.unwrap();
/// assert!(matches!(result[0].node, Some(Nodes::SelectStmt(_))));
/// ```
pub fn parse(statement: &str) -> Result<Vec<crate::Node>> {
    let input = CString::new(statement)?;
    let result = unsafe { pg_query_parse_protobuf(input.as_ptr()) };
    let protobuf = if !result.error.is_null() {
        let message = unsafe { CStr::from_ptr((*result.error).message) }
            .to_string_lossy()
            .to_string();
        Err(Error::Parse(message))
    } else {
        let data = unsafe {
            std::slice::from_raw_parts(
                result.parse_tree.data as *const u8,
                result.parse_tree.len as usize,
            )
        };
        ParseResult::decode(data).map_err(Error::Decode)
    };
    unsafe { pg_query_free_protobuf_parse_result(result) };
    Ok(protobuf?.stmts.into_iter().filter_map(|s| s.stmt.and_then(|s| Some(*s))).collect())
}

/// Normalizes the given SQL statement, returning a parametized version.
///
/// # Example
///
/// ```rust
/// let result = pg_query::normalize("SELECT * FROM contacts WHERE name='Paul'");
/// assert!(result.is_ok());
/// let result = result.unwrap();
/// assert_eq!(result, "SELECT * FROM contacts WHERE name=$1");
/// ```
pub fn normalize(statement: &str) -> Result<String> {
    let input = CString::new(statement).unwrap();
    let result = unsafe { pg_query_normalize(input.as_ptr() as *const c_char) };
    let normalized_query = if !result.error.is_null() {
        let message = unsafe { CStr::from_ptr((*result.error).message) }
            .to_string_lossy()
            .to_string();
        return Err(Error::Parse(message));
    } else {
        let n = unsafe { CStr::from_ptr(result.normalized_query) };
        Ok(n.to_string_lossy().to_string())
    };
    unsafe { pg_query_free_normalize_result(result) };
    normalized_query
}

/// Fingerprints the given SQL statement. Useful for comparing parse trees across different implementations
/// of `libpg_query`.
///
/// # Example
///
/// ```rust
/// let result = pg_query::fingerprint("SELECT * FROM contacts WHERE name='Paul'");
/// assert!(result.is_ok());
/// let result = result.unwrap();
/// assert_eq!(result.hex, "0e2581a461ece536");
/// ```
pub fn fingerprint(statement: &str) -> Result<Fingerprint> {
    let input = CString::new(statement)?;
    let result = unsafe { pg_query_fingerprint(input.as_ptr()) };
    let fingerprint = if !result.error.is_null() {
        let message = unsafe { CStr::from_ptr((*result.error).message) }
            .to_string_lossy()
            .to_string();
        Err(Error::Parse(message))
    } else {
        let hex = unsafe { CStr::from_ptr(result.fingerprint_str) };
        Ok(Fingerprint {
            value: result.fingerprint,
            hex: hex.to_string_lossy().to_string(),
        })
    };
    unsafe { pg_query_free_fingerprint_result(result) };
    fingerprint
}

/// An experimental API which parses a PLPGSQL function. This currently returns the raw JSON structure.
///
/// # Example
///
/// ```rust
/// let result = pg_query::parse_plpgsql("
///     CREATE OR REPLACE FUNCTION cs_fmt_browser_version(v_name varchar, v_version varchar)
///     RETURNS varchar AS $$
///     BEGIN
///         IF v_version IS NULL THEN
///             RETURN v_name;
///         END IF;
///         RETURN v_name || '/' || v_version;
///     END;
///     $$ LANGUAGE plpgsql;
/// ");
/// assert!(result.is_ok());
/// ```
pub fn parse_plpgsql(stmt: &str) -> Result<serde_json::Value> {
    let input = CString::new(stmt)?;
    let result = unsafe { pg_query_parse_plpgsql(input.as_ptr()) };
    let structure = if !result.error.is_null() {
        let message = unsafe { CStr::from_ptr((*result.error).message) }
            .to_string_lossy()
            .to_string();
        Err(Error::Parse(message))
    } else {
        let raw = unsafe { CStr::from_ptr(result.plpgsql_funcs) };
        serde_json::from_str(&raw.to_string_lossy())
            .map_err(|e| Error::InvalidJson(e.to_string()))
    };
    unsafe { pg_query_free_plpgsql_parse_result(result) };
    structure
}
