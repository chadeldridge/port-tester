use log::{error, warn};
use thiserror::Error;

// Barrowed from eza.
/// Exit code for successful execution.
pub const CODE_SUCCESS: i32 = 0;

/// Exit code for when there was at least one I/O error during execution.
pub const CODE_RUNTIME_ERROR: i32 = 1;

/// Exit code for when the command-line options are invalid.
pub const CODE_OPTIONS_ERROR: i32 = 3;

/// Exit code for missing file permissions
pub const CODE_PERMISSION_DENIED: i32 = 13;

// Barrowed heavily from bat because I'm still learning.

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SourceError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJson(#[from] ::serde_json::Error),
    //#[error(transparent)]
    //SerdeYaml(#[from] ::serde_yaml::Error),
    #[error("Unsupported input format: {0}")]
    UnsupportedInputFormat(String),
    #[error("Unsupported output format: {0}")]
    UnsupportedOutputFormat(String),
    #[error("Invalid input source: {0}")]
    InvalidInputSource(String),
    #[error("{0}")]
    Msg(String),
}

impl From<&'static str> for SourceError {
    fn from(s: &'static str) -> Self {
        SourceError::Msg(s.to_owned())
    }
}

impl From<String> for SourceError {
    fn from(s: String) -> Self {
        SourceError::Msg(s)
    }
}

#[derive(Debug)]
pub struct Error {
    code: Option<i32>,
    context: String,
    print_help: bool,
    source: SourceError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.context.is_empty() {
            write!(f, "{}", self.source)
        } else {
            write!(f, "{}\n{}", self.context, self.source)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl Error {
    pub fn new(source: SourceError) -> Self {
        Error {
            code: None,
            context: String::new(),
            print_help: false,
            source,
        }
    }

    pub fn code(&self) -> Option<i32> {
        self.code
    }

    pub fn mut_code(&mut self, code: i32) {
        self.code = Some(code);
    }

    pub fn set_code(mut self, code: i32) -> Self {
        self.code = Some(code);
        self
    }

    pub fn unset_code(mut self) -> Self {
        self.code = None;
        self
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    pub fn mut_context(&mut self, context: &str) {
        if !self.context.is_empty() {
            self.context = format!("{}\n{}", context, self.context);
        } else {
            self.context = context.to_owned();
        }
    }

    pub fn set_context(mut self, context: &str) -> Self {
        if !context.is_empty() {
            self.context = context.to_owned();
        } else {
            self.context = format!("{}\n{}", context, self.context);
        }

        self
    }

    pub fn is_print_help(&self) -> bool {
        self.print_help
    }

    pub fn print_help(mut self) -> Self {
        self.print_help = true;
        self
    }

    pub fn with_print_help(mut self, print_help: bool) -> Self {
        self.print_help = print_help;
        self
    }

    pub fn source(&self) -> &SourceError {
        &self.source
    }

    pub fn mut_source(&mut self, source: SourceError) {
        self.source = source;
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn handle_error(error: &Error) {
    match &error.source {
        SourceError::Io(io_err) if io_err.kind() == std::io::ErrorKind::BrokenPipe => {
            warn!("Broken pipe encountered: {}", io_err);
            ::std::process::exit(0);
        }
        //SourceError::SerdeJson(_) | SourceError::SerdeYaml(_) => {
        SourceError::SerdeJson(_) => {
            error!("Error while parsing file: {error}")
        }
        _ => {
            error!("{error}")
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_source_error_variants() {
        let io_error = std::io::Error::other("io error");
        let err_io = SourceError::Io(io_error);
        assert!(matches!(err_io, SourceError::Io(..)));
        //let _ = SourceError::SerdeJson(serde_json::Error::io(io_error));
        //let _ = SourceError::SerdeYaml(serde_yaml::errors::new(io_error));
        let err_uif = SourceError::UnsupportedInputFormat("unsupported".to_string());
        assert!(matches!(err_uif, SourceError::UnsupportedInputFormat(_)));
        let err_uof = SourceError::UnsupportedOutputFormat("unsupported".to_string());
        assert!(matches!(err_uof, SourceError::UnsupportedOutputFormat(_)));
        let err_iis = SourceError::InvalidInputSource("invalid".to_string());
        assert!(matches!(err_iis, SourceError::InvalidInputSource(_)));
        let err_msg = SourceError::Msg("message".to_string());
        assert!(matches!(err_msg, SourceError::Msg(_)));
    }

    #[test]
    fn test_source_error_debug() {
        let err = SourceError::Msg("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Msg"));
    }

    #[test]
    fn test_source_error_from_string() {
        let err: SourceError = String::from("error message").into();
        assert!(matches!(err, SourceError::Msg(ref msg) if msg == "error message"));
    }

    #[test]
    fn test_source_error_from_str() {
        let err: SourceError = "error message".into();
        assert!(matches!(err, SourceError::Msg(ref msg) if msg == "error message"));
    }

    #[test]
    fn test_error_builder_methods() {
        let source_err = SourceError::Msg("source error".to_string());
        let err = Error::new(source_err)
            .set_context("additional context")
            .print_help()
            .set_code(42);
        assert_eq!(err.context, "additional context");
        assert!(err.print_help);
        assert_eq!(err.code, Some(42));
    }

    #[test]
    fn test_error_display() {
        let source_err = SourceError::Msg("source error".to_string());
        let err = Error::new(source_err).set_context("additional context");
        let display_str = format!("{}", err);
        assert!(display_str.contains("additional context"));
        assert!(display_str.contains("source error"));
    }

    #[test]
    fn test_handle_error_broken_pipe() {
        let io_error = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe");
        let err = Error::new(SourceError::Io(io_error));
        // This will exit the process, so we can't actually call it in a test.
        // handle_error(&err);
        // Instead, we just ensure it matches the broken pipe case.
        match &err.source {
            SourceError::Io(io_err) if io_err.kind() == std::io::ErrorKind::BrokenPipe => {}
            _ => panic!("Expected broken pipe error"),
        }
    }

    #[test]
    fn test_handle_error_other() {
        let source_err = SourceError::Msg("some error".to_string());
        let err = Error::new(source_err);
        // This will log the error, so we can't easily test the logging output here.
        // handle_error(&err);
        // Instead, we just ensure it matches the other case.
        match &err.source {
            SourceError::Msg(msg) if msg == "some error" => {}
            _ => panic!("Expected some error message"),
        }
    }

    #[test]
    fn test_result_type_alias() {
        fn example_function() -> Result<i32> {
            Ok(42)
        }
        let result = example_function();
        assert_eq!(result.unwrap(), 42);
    }
}
