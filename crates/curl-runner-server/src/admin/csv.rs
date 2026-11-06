//! CSV exports: streamed `text/csv` with a dated filename, RFC 4180 quoting
//! and spreadsheet formula escaping.

use axum::body::{Body, Bytes};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use chrono::NaiveDate;
use futures_util::{Stream, StreamExt, stream};

use crate::error::ApiError;

/// One cell. A value a spreadsheet would run as a formula (it starts with
/// `=`, `+`, `-`, `@`, a tab or a carriage return) gets a leading `'`. A
/// value with a comma, quote or line break is quoted, with quotes doubled.
pub fn cell(value: &str) -> String {
    let value = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

/// One record, CRLF-terminated.
pub fn line<S: AsRef<str>>(cells: &[S]) -> String {
    let mut out = cells
        .iter()
        .map(|c| cell(c.as_ref()))
        .collect::<Vec<_>>()
        .join(",");
    out.push_str("\r\n");
    out
}

/// Stream `header` and then each row as `<name>-<today>.csv`. Rows are
/// written as they arrive; an error mid-way ends the download early.
pub fn stream<S>(name: &str, today: NaiveDate, header: &[&str], rows: S) -> Response
where
    S: Stream<Item = Result<Vec<String>, ApiError>> + Send + 'static,
{
    let head = stream::once(std::future::ready(Ok::<_, ApiError>(Bytes::from(line(
        header,
    )))));
    let body = head.chain(rows.map(|row| row.map(|cells| Bytes::from(line(&cells)))));
    let disposition = format!(
        "attachment; filename=\"{name}-{}.csv\"",
        today.format("%Y-%m-%d")
    );
    let mut res = Body::from_stream(body).into_response();
    let headers = res.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_get_a_quote_prefix() {
        assert_eq!(cell("=1+1"), "'=1+1");
        assert_eq!(cell("+1"), "'+1");
        assert_eq!(cell("-1"), "'-1");
        assert_eq!(cell("@SUM(A1)"), "'@SUM(A1)");
        assert_eq!(cell("\tx"), "'\tx");
        assert_eq!(cell("a=b"), "a=b");
        assert_eq!(cell(""), "");
    }

    #[test]
    fn quoting_follows_rfc_4180() {
        assert_eq!(cell("a,b"), "\"a,b\"");
        assert_eq!(cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(cell("two\nlines"), "\"two\nlines\"");
        assert_eq!(cell("=\"x\""), "\"'=\"\"x\"\"\"");
        assert_eq!(line(&["a", "b,c", ""]), "a,\"b,c\",\r\n");
    }
}
