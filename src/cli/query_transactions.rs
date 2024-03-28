use crate::cli::util::is_http_status;
use crate::client::QueryTransactionRequest;
use crate::state::State;
use log::{debug, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::cli::error::{deserialize_error, generic_error, no_active_cluster_error};
use crate::cli::query::{handle_query_response, query_context_from_args, send_query};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct QueryTransactions {
    state: Arc<Mutex<State>>,
}

impl QueryTransactions {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for QueryTransactions {
    fn name(&self) -> &str {
        "query transactions"
    }

    fn signature(&self) -> Signature {
        Signature::build("query transactions")
            .required("statement", SyntaxShape::String, "the query statement")
            .named(
                "bucket",
                SyntaxShape::String,
                "the bucket to query against",
                None,
            )
            .named(
                "scope",
                SyntaxShape::String,
                "the scope to query against",
                None,
            )
            .named("query-timeout", SyntaxShape::Int, "timeout (milliseconds) to apply to the query", None)
            .named("transaction-timeout", SyntaxShape::Int, "timeout (milliseconds) to apply to the transaction, only applicable when starting the transaction", None)
            .switch("with-meta", "include toplevel metadata", None)
            .switch("disable-context", "disable automatically detecting the query context based on the active bucket and scope", None)
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Performs a n1ql query as a part of a transaction"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        query(self.state.clone(), engine_state, stack, call, input)
    }
}

// See https://github.com/couchbase/godbc/blob/master/n1ql/n1ql.go for a lot of the approach here.
fn query(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let statement: String = call.req(engine_state, stack, 0)?;

    validate_statement(&statement, span)?;

    let statement_type = parse_statement(&statement);

    let st = state.clone();
    let mut guard = st.lock().unwrap();

    let active_cluster = match guard.active_cluster() {
        Some(c) => c,
        None => {
            return Err(no_active_cluster_error(span));
        }
    };

    let active_txn = guard.active_transaction();

    let txn_request = if let Some(txn_state) = active_txn {
        info!(
            "Continuing existing transaction for {} on {}",
            txn_state.id(),
            txn_state.endpoint()
        );
        QueryTransactionRequest::new(None, txn_state.id(), txn_state.endpoint())
    } else if statement_type == TransactionStatementType::Start {
        let timeout = call
            .get_flag(engine_state, stack, "transaction-timeout")?
            .map(|t: i64| Duration::from_millis(t as u64))
            .unwrap_or(active_cluster.timeouts().transaction_timeout());

        info!(
            "Starting a new transaction with timeout {}ms",
            &timeout.as_millis()
        );
        QueryTransactionRequest::new(timeout, None, None)
    } else {
        return Err(generic_error(
            "No active transaction",
            "No transaction is currently active, run BEGIN WORK to start one".to_string(),
            span,
        ));
    };

    let maybe_scope = query_context_from_args(active_cluster, engine_state, stack, call)?;

    let timeout = call
        .get_flag(engine_state, stack, "query-timeout")?
        .map(|t: i64| Duration::from_millis(t as u64));

    debug!("Running n1ql query transaction {}", &statement);

    let response = send_query(
        active_cluster,
        statement.clone(),
        maybe_scope,
        ctrl_c.clone(),
        timeout,
        span,
        txn_request,
    );
    if response.is_err() {
        info!("Ending transaction due to error");
        guard.end_transaction();
    }
    let response = response?;

    if is_http_status(&response, 200, span).is_err() {
        info!("Ending transaction due to non-200 status code");
        guard.end_transaction();
    };

    if statement_type == TransactionStatementType::Rollback
        || statement_type == TransactionStatementType::Commit
    {
        info!("Ending transaction");
        guard.end_transaction();
    };

    if statement_type == TransactionStatementType::Start {
        if let Some(txid) = parse_txid(response.content(), span)? {
            info!(
                "Updating state to start transaction for {} on {}",
                &txid,
                response.endpoint()
            );
            guard.start_transaction(txid, response.endpoint())?;
        }
    }

    let results = handle_query_response(
        call.has_flag(engine_state, stack, "with-meta")?,
        guard.active(),
        response,
        span,
    )?;

    if results.len() > 0 {
        return Ok(Value::List {
            vals: results,
            internal_span: call.head,
        }
        .into_pipeline_data());
    }

    Ok(PipelineData::new_with_metadata(None, span))
}

fn validate_statement(statement: &String, span: Span) -> Result<(), ShellError> {
    let statement = statement.trim().to_string();
    if statement.contains(";") {
        if let Some(p) = statement.chars().position(|e| e == ';') {
            if p != statement.len() - 1 {
                return Err(generic_error(
                    "statement cannot contain more than a single query statement",
                    "if your statement contains multiple statements then you must execute them individually".to_string(),
                    span));
            }
        }
    }

    Ok(())
}

fn query_status_value_success(status: Option<&serde_json::value::Value>) -> bool {
    if let Some(st) = status {
        if let Some(s) = st.as_str() {
            if s != "success" {
                return false;
            }
        } else {
            return false;
        }
    } else {
        return false;
    }

    true
}

fn parse_txid(response: &str, span: Span) -> Result<Option<String>, ShellError> {
    let content: HashMap<String, serde_json::Value> =
        serde_json::from_str(response).map_err(|e| deserialize_error(e.to_string(), span))?;
    let status = content.get("status");
    if !query_status_value_success(status) {
        return Ok(None);
    }

    let results = content.get("results");
    if let Some(results) = results {
        if let Some(results) = results.as_array() {
            if let Some(result) = results.get(0) {
                if let Some(map) = result.as_object() {
                    if let Some(txid) = map.get("txid") {
                        if let Some(txid) = txid.as_str() {
                            return Ok(Some(txid.to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
enum TransactionStatementType {
    None,
    Start,
    Commit,
    Rollback,
}

fn parse_statement(statement: &String) -> TransactionStatementType {
    let mut statement = statement.trim().to_string();
    statement = match statement.get(..32) {
        Some(s) => s.to_string(),
        None => statement,
    };
    statement = statement.to_lowercase();
    let parts: Vec<String> = statement
        .split_whitespace()
        .map(|e| e.to_string())
        .collect();
    if parts.len() > 0 {
        match parts.get(0).unwrap().trim_end_matches(';') {
            "start" => {
                if parts.len() > 1 {
                    return TransactionStatementType::Start;
                }
            }
            "begin" => {
                if parts.len() > 1 {
                    return TransactionStatementType::Start;
                }
            }
            "commit" => return TransactionStatementType::Commit,
            "rollback" => {
                if parts.len() < 3 {
                    return TransactionStatementType::Rollback;
                }
            }
            _ => {}
        }
    }

    TransactionStatementType::None
}
