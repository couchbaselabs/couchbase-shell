use crate::client::QueryTransactionRequest;
use crate::state::State;
use log::{debug, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::cli::error::{deserialize_error, generic_error, no_active_cluster_error};
use crate::cli::query::{handle_query_response, query_context_from_args, send_query};
use crate::cli::{client_error_to_shell_error, malformed_response_error};
use crate::client::connection_client::ResponseMetadata;
use crate::client::query_metadata::QueryStatus;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value::Nothing;
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use tokio::runtime::Runtime;

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

    fn description(&self) -> &str {
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
    let signals = engine_state.signals().clone();
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

    let rt = Runtime::new()?;
    let response = rt.block_on(async {
        send_query(
            active_cluster,
            statement.clone(),
            None,
            maybe_scope,
            signals.clone(),
            timeout,
            span,
            txn_request,
        )
        .await
    });
    if response.is_err() {
        info!("Ending transaction due to error");
        guard.end_transaction();
    }

    let mut response = response?;

    let endpoint = response.endpoint().to_string();

    let (content, meta) = rt.block_on(async {
        let content = response
            .content()
            .await
            .map_err(|e| client_error_to_shell_error(e, span))?;
        let meta = response
            .metadata()
            .map_err(|e| client_error_to_shell_error(e, span))?;

        Ok::<(Vec<Vec<u8>>, Option<ResponseMetadata>), ShellError>((content, meta))
    })?;

    let meta = match meta {
        Some(ResponseMetadata::Query(m)) => m,
        None => {
            return Err(malformed_response_error(
                "response missing metadata",
                "".to_string(),
                span,
            ))
        }
    };

    if statement_type == TransactionStatementType::Rollback
        || statement_type == TransactionStatementType::Commit
    {
        info!("Ending transaction");
        guard.end_transaction();
    };

    if statement_type == TransactionStatementType::Start {
        if let Some(txid) = parse_txid(&content, meta.status, span)? {
            info!(
                "Updating state to start transaction for {} on {}",
                &txid, endpoint
            );
            guard.start_transaction(txid, endpoint.to_string())?;
        }
    }

    let results = rt.block_on(async {
        handle_query_response(
            call.has_flag(engine_state, stack, "with-meta")?,
            guard.active(),
            content,
            Some(meta),
            span,
        )
        .await
    })?;

    if !results.is_empty() {
        return Ok(Value::List {
            vals: results,
            internal_span: call.head,
        }
        .into_pipeline_data());
    }

    Ok(PipelineData::Value(
        Nothing {
            internal_span: span,
        },
        None,
    ))
}

fn validate_statement(statement: &str, span: Span) -> Result<(), ShellError> {
    let statement = statement.trim().to_string();
    if statement.contains(';') {
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

fn parse_txid(
    response: &Vec<Vec<u8>>,
    status: QueryStatus,
    span: Span,
) -> Result<Option<String>, ShellError> {
    if status != QueryStatus::Success {
        return Ok(None);
    }

    if response.is_empty() {
        return Ok(None);
    }

    let content = &response[0];

    let content: HashMap<String, serde_json::Value> =
        serde_json::from_slice(&content).map_err(|e| deserialize_error(e.to_string(), span))?;

    if let Some(txid) = content.get("txid") {
        if let Some(txid) = txid.as_str() {
            return Ok(Some(txid.to_string()));
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

fn parse_statement(statement: &str) -> TransactionStatementType {
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
    if !parts.is_empty() {
        match parts.first().unwrap().trim_end_matches(';') {
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
