use std::{
    io::{self, Stdout, Write},
    process::{Command, ExitStatus, Output, Stdio},
};
use thiserror::Error;

/* pub enum DataStoreError {
    #[error("data store disconnected")]
    Disconnect(#[from] io::Error),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("unknown data store error")]
    Unknown,
} */

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("io: {0:?}")]
    IO(#[from] IOError),
    #[error("ExitCode {code:?}: {error}")]
    StdError { error: String, code: Option<i32> },
}

#[derive(Error, Debug)]
pub enum IOError {
    #[error("failed to create child process: {0:?}")]
    ChildProcess(io::Error),
    #[error("failed to write to stdin: {0:?}")]
    WriteToStdin(io::Error),
    #[error("failed to wait for output: {0:?}")]
    WaitForOutput(io::Error),
}

pub struct Executor {
    command: Command,
}

impl Executor {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn run(&mut self, row: &str) -> Result<Vec<u8>, ExecutorError> {
        let mut child = self
            .command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| IOError::ChildProcess(e))?;

        // Write the row data to the stdin of the child process
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(row.as_bytes())
                .map_err(|e| IOError::WriteToStdin(e))?;

            stdin.flush().map_err(|e| IOError::WriteToStdin(e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| IOError::WaitForOutput(e))?;

        dbg!(&output);

        match output.status.code() {
            Some(0) => Ok(output.stdout),
            Some(code) => Err(ExecutorError::StdError {
                error: format!("{:?} + {:?}", output.stderr, output.stdout),
                code: Some(code),
            }),
            None => Err(ExecutorError::StdError {
                error: format!("{:?} + {:?}", output.stderr, output.stdout),
                code: None,
            }),
        }
    }
}
