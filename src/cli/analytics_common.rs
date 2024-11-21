use crate::cli::util::{convert_row_to_nu_value, duration_to_golang_string};
use crate::cli::{
    analytics_error, client_error_to_shell_error, deserialize_error, malformed_response_error,
    unexpected_status_code_error, AnalyticsErrorReason,
};
use crate::client::http_handler::HttpStreamResponse;
use crate::client::AnalyticsQueryRequest;
use crate::remote_cluster::RemoteCluster;
use nu_protocol::{ShellError, Signals, Span, Value};
use std::ops::Add;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::time::Instant;

pub fn send_analytics_query(
    active_cluster: &RemoteCluster,
    scope: impl Into<Option<(String, String)>>,
    statement: impl Into<String>,
    signals: Signals,
    span: Span,
    rt: Arc<Runtime>,
) -> Result<HttpStreamResponse, ShellError> {
    let response = active_cluster
        .cluster()
        .http_client()
        .analytics_query_request(
            AnalyticsQueryRequest::Execute {
                statement: statement.into(),
                scope: scope.into(),
                timeout: duration_to_golang_string(active_cluster.timeouts().analytics_timeout()),
            },
            Instant::now().add(active_cluster.timeouts().analytics_timeout()),
            signals.clone(),
            rt.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    if response.status() != 200 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content()?,
            span,
        ));
    }

    Ok(response)
}

pub fn read_analytics_response(
    identifier: String,
    response: HttpStreamResponse,
    span: Span,
    with_meta: bool,
    could_contain_mutations: bool,
) -> Result<Vec<Value>, ShellError> {
    let content = response.content()?;

    let content: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| deserialize_error(e.to_string(), span))?;

    let mut results: Vec<Value> = vec![];
    if with_meta {
        let converted = &mut convert_row_to_nu_value(&content, span, identifier)?;
        results.append(converted);
        return Ok(results);
    }

    if let Some(content_errors) = content.get("errors") {
        return if let Some(arr) = content_errors.as_array() {
            if arr.len() == 1 {
                let e = match arr.first() {
                    Some(e) => e,
                    None => {
                        return Err(malformed_response_error(
                            "analytics errors present but empty",
                            content_errors.to_string(),
                            span,
                        ))
                    }
                };
                let code = e.get("code").map(|c| c.as_i64().unwrap_or_default());
                let reason = match code {
                    Some(c) => AnalyticsErrorReason::from(c),
                    None => AnalyticsErrorReason::UnknownError,
                };
                let msg = match e.get("msg") {
                    Some(msg) => msg.to_string(),
                    None => "".to_string(),
                };
                Err(analytics_error(reason, code, msg, span))
            } else {
                let messages = arr
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                Err(analytics_error(
                    AnalyticsErrorReason::MultiErrors,
                    None,
                    messages,
                    span,
                ))
            }
        } else {
            Err(malformed_response_error(
                "analytics errors not an array",
                content_errors.to_string(),
                span,
            ))
        };
    } else if let Some(content_results) = content.get("results") {
        if let Some(arr) = content_results.as_array() {
            for result in arr {
                results.append(&mut convert_row_to_nu_value(
                    result,
                    span,
                    identifier.clone(),
                )?)
            }
        } else {
            return Err(malformed_response_error(
                "analytics rows not an array",
                content_results.to_string(),
                span,
            ));
        }
    } else if !could_contain_mutations {
        return Err(malformed_response_error(
            "analytics toplevel result not an object",
            content.to_string(),
            span,
        ));
    }

    Ok(results)
}
