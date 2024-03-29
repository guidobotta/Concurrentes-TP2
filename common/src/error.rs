use regex::Error as RegexError;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Error as IOError;
use std::num::{ParseIntError, ParseFloatError};
use std::string::FromUtf8Error;

/// Clase utilizada para manejar error internos del sistema
#[derive(Debug)]
pub struct ErrorInterno {
    mensaje: String,
}

impl ErrorInterno {
    /// Genera una intancia de ErrorInterno, el string recibido es utilizado para identificar el error.
    pub fn new(msg: &str) -> ErrorInterno {
        ErrorInterno {
            mensaje: msg.to_string(),
        }
    }
}

impl Display for ErrorInterno {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.mensaje)
    }
}

impl Error for ErrorInterno {
    fn description(&self) -> &str {
        &self.mensaje
    }
}

/// Enum para conversion de errores

#[derive(Debug)]
pub enum ErrorApp {
    Interno(ErrorInterno),
    ErrorIO(IOError),
    ErrorRegex(RegexError),
    ErrorUtf8(FromUtf8Error),
    ErrorParseoInt(ParseIntError),
    ErrorParseoFloat(ParseFloatError)
}

/// Tipo de resultado

pub type Resultado<T> = std::result::Result<T, ErrorApp>;

//Conversion de errores a ErrorApp

impl From<ErrorInterno> for ErrorApp {
    fn from(err: ErrorInterno) -> ErrorApp {
        ErrorApp::Interno(err)
    }
}

impl From<IOError> for ErrorApp {
    fn from(err: IOError) -> ErrorApp {
        ErrorApp::ErrorIO(err)
    }
}

impl From<RegexError> for ErrorApp {
    fn from(err: RegexError) -> ErrorApp {
        ErrorApp::ErrorRegex(err)
    }
}

impl From<FromUtf8Error> for ErrorApp {
    fn from(err: FromUtf8Error) -> ErrorApp {
        ErrorApp::ErrorUtf8(err)
    }
}

impl From<ParseIntError> for ErrorApp {
    fn from(err: ParseIntError) -> ErrorApp {
        ErrorApp::ErrorParseoInt(err)
    }
}

impl From<ParseFloatError> for ErrorApp {
    fn from(err: ParseFloatError) -> ErrorApp {
        ErrorApp::ErrorParseoFloat(err)
    }
}

impl Display for ErrorApp {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            ErrorApp::Interno(ref inner) => inner.fmt(f),
            ErrorApp::ErrorIO(ref inner) => inner.fmt(f),
            ErrorApp::ErrorRegex(ref inner) => inner.fmt(f),
            ErrorApp::ErrorUtf8(ref inner) => inner.fmt(f),
            ErrorApp::ErrorParseoInt(ref inner) => inner.fmt(f),
            ErrorApp::ErrorParseoFloat(ref inner) => inner.fmt(f),
        }
    }
}
