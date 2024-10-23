use crate::json_row_stream::JsonRowStream;
use futures::{Stream, StreamExt};
use futures_core::FusedStream;
use nu_protocol::ShellError;
use serde_json::Value;
use std::cmp::{PartialEq, PartialOrd};
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(PartialEq, Eq, PartialOrd, Debug)]
enum RowStreamState {
    Start = 0,
    Rows = 1,
    PostRows = 2,
    End = 3,
}

pub struct RawJsonRowStreamer {
    stream: JsonRowStream,
    rows_attrib: String,
    buffered_row: Vec<u8>,
    attribs: HashMap<String, Value>,
    state: RowStreamState,
}

impl RawJsonRowStreamer {
    pub fn new(stream: JsonRowStream, rows_attrib: impl Into<String>) -> Self {
        Self {
            stream,
            rows_attrib: rows_attrib.into(),
            buffered_row: Vec::new(),
            attribs: HashMap::new(),
            state: RowStreamState::Start,
        }
    }

    async fn begin(&mut self) -> Result<(), ShellError> {
        if self.state != RowStreamState::Start {
            return Err(ShellError::GenericError {
                error: "".to_string(),
                msg: "Unexpected parsing state during begin".to_string(),
                span: None,
                help: None,
                inner: vec![],
            });
        }

        let first = match self.stream.next().await {
            Some(result) => result?,
            None => {
                return Err(ShellError::GenericError {
                    error: "Expected first line to be non-empty".to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: vec![],
                })
            }
        };

        if &first[..] != b"{" {
            return Err(ShellError::GenericError {
                error: "Expected an opening brace for the result".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: vec![],
            });
        }
        loop {
            match self.stream.next().await {
                Some(item) => {
                    let mut item =
                        String::from_utf8(item?).map_err(|e| ShellError::GenericError {
                            error: e.to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![],
                        })?;
                    if item.is_empty() || item == "}" {
                        self.state = RowStreamState::End;
                        break;
                    }
                    if item.contains(&self.rows_attrib) {
                        if let Some(maybe_row) = self.stream.next().await {
                            let maybe_row = maybe_row?;
                            let str_row = std::str::from_utf8(&maybe_row).map_err(|e| {
                                ShellError::GenericError {
                                    error: e.to_string(),
                                    msg: "".to_string(),
                                    span: None,
                                    help: None,
                                    inner: vec![],
                                }
                            })?;
                            // if there are no more rows, immediately move to post-rows
                            if str_row == "]" {
                                self.state = RowStreamState::PostRows;
                                break;
                            } else {
                                // We can't peek, so buffer the first row
                                self.buffered_row = maybe_row;
                            }
                        }
                        self.state = RowStreamState::Rows;
                        break;
                    }

                    // Wrap the line in a JSON object to deserialize
                    item = format!("{{{}}}", item);
                    let json_value: HashMap<String, Value> =
                        serde_json::from_str(&item).map_err(|e| ShellError::GenericError {
                            error: e.to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![],
                        })?;

                    // Save the attribute for the metadata
                    for (k, v) in json_value {
                        self.attribs.insert(k, v);
                    }
                }
                None => {
                    self.state = RowStreamState::End;
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn has_more_rows(&self) -> bool {
        if self.state < RowStreamState::Rows {
            return false;
        }

        if self.state > RowStreamState::Rows {
            return false;
        }

        !self.buffered_row.is_empty()
    }

    pub async fn read_row(&mut self) -> Result<Option<Vec<u8>>, ShellError> {
        if self.state < RowStreamState::Rows {
            return Err(ShellError::GenericError {
                error: "Unexpected parsing state during read rows".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: vec![],
            });
        }

        // If we've already read all rows or rows is null, we return None
        if self.state > RowStreamState::Rows {
            return Ok(None);
        }

        let row = self.buffered_row.clone();

        if let Some(maybe_row) = self.stream.next().await {
            let maybe_row = maybe_row?;
            let str_row =
                std::str::from_utf8(&maybe_row).map_err(|e| ShellError::GenericError {
                    error: e.to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: vec![],
                })?;
            if str_row == "]" {
                self.state = RowStreamState::PostRows;
            } else {
                self.buffered_row = maybe_row;
            }
        }

        Ok(Some(row))
    }

    async fn end(&mut self) -> Result<(), ShellError> {
        if self.state < RowStreamState::PostRows {
            return Err(ShellError::GenericError {
                error: "Unexpected parsing state during end".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: vec![],
            });
        }

        // Check if we've already read everything
        if self.state > RowStreamState::PostRows {
            return Ok(());
        }

        loop {
            match self.stream.next().await {
                Some(item) => {
                    let mut item =
                        String::from_utf8(item?).map_err(|e| ShellError::GenericError {
                            error: e.to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![],
                        })?;

                    if item == "}" || item.is_empty() {
                        self.state = RowStreamState::End;
                        break;
                    }
                    item = format!("{{{}}}", item);
                    let json_value: HashMap<String, Value> =
                        serde_json::from_str(&item).map_err(|e| ShellError::GenericError {
                            error: e.to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![],
                        })?;
                    for (k, v) in json_value {
                        self.attribs.insert(k, v);
                    }
                }
                None => {
                    self.state = RowStreamState::End;
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn read_prelude(&mut self) -> Result<Vec<u8>, ShellError> {
        self.begin().await?;
        serde_json::to_vec(&self.attribs).map_err(|e| ShellError::GenericError {
            error: e.to_string(),
            msg: "".to_string(),
            span: None,
            help: None,
            inner: vec![],
        })
    }

    pub async fn read_epilog(&mut self) -> Result<Vec<u8>, ShellError> {
        self.end().await?;
        serde_json::to_vec(&self.attribs).map_err(|e| ShellError::GenericError {
            error: e.to_string(),
            msg: "".to_string(),
            span: None,
            help: None,
            inner: vec![],
        })
    }
}

impl FusedStream for RawJsonRowStreamer {
    fn is_terminated(&self) -> bool {
        matches!(self.state, RowStreamState::End) || matches!(self.state, RowStreamState::PostRows)
    }
}

impl Stream for RawJsonRowStreamer {
    type Item = Result<Vec<u8>, ShellError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.state < RowStreamState::Rows {
            return Poll::Ready(Some(Err(ShellError::GenericError {
                error: "Unexpected parsing state during read rows".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: vec![],
            })));
        }

        // Check if we've already read everything
        if self.state >= RowStreamState::PostRows {
            return Poll::Ready(None);
        }

        let row = self.buffered_row.clone();

        let this = self.get_mut();

        match this.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(stream_row))) => {
                let str_row =
                    std::str::from_utf8(&stream_row).map_err(|e| ShellError::GenericError {
                        error: e.to_string(),
                        msg: "".to_string(),
                        span: None,
                        help: None,
                        inner: vec![],
                    })?;
                if str_row == "]" {
                    this.state = RowStreamState::PostRows;
                } else {
                    this.buffered_row = stream_row;
                }
                Poll::Ready(Some(Ok(row)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => {
                this.state = RowStreamState::End;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
