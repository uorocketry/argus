// use atsamd_hal::dmac;
use core::convert::Infallible;
use defmt::{write, Format};
use derive_more::From;
use embedded_sdmmc as sd;
use messages::ErrorContext;
use nb::Error as NbError;
/// Open up atsamd hal errors without including the whole crate.

/// Contains all the various error types that can be encountered in the Rocket codebase. Extra errors
/// types should be added to this list whenever needed.
#[derive(From)]
pub enum RocketErrorType {
    /// An Infallible error. This error should never happen.
    Infallible(Infallible),
    /// Error from the Postcard serialization library.
    PostcardError(postcard::Error),
    /// Error that occurred while spawning an RTIC task. Contains the name of the failed task.
    SpawnError(&'static str),
    /// Error from the SD card library.
    SdCardError(sd::Error<sd::SdMmcError>),
    /// Error from the Mavlink library.
    MavlinkError(messages::mavlink::error::MessageWriteError),
    MavlinkReadError(messages::mavlink::error::MessageReadError),
    NbError(NbError<Infallible>),
}

impl defmt::Format for RocketErrorType {
    fn format(&self, f: defmt::Formatter) {
        match self {
            RocketErrorType::Infallible(_) => {
                write!(f, "Infallible error encountered!")
            }
            RocketErrorType::PostcardError(e) => {
                write!(f, "Postcard error: {}", e)
            }
            RocketErrorType::SpawnError(e) => {
                write!(f, "Could not spawn task '{}'", e);
            }
            RocketErrorType::SdCardError(_) => {
                write!(f, "SD card error!");
            }
            RocketErrorType::MavlinkError(_) => {
                write!(f, "Mavlink error!");
            }
            RocketErrorType::MavlinkReadError(_) => {
                write!(f, "Mavlink read error!");
            }
            RocketErrorType::NbError(_) => {
                write!(f, "Nb error!");
            }
        }
    }
}

/// Standard Rocket error. This type should be used as the return type for most functions that can
/// fail and that returns a `Result`.
#[derive(Format)]
pub struct RocketError {
    error: RocketErrorType,
    context: Option<ErrorContext>,
}

impl RocketError {
    pub fn get_context(&self) -> Option<ErrorContext> {
        self.context
    }
}

/// Utility trait for implementing an easy way to convert a RTIC spawn error to a [`RocketError`].
/// This is necessary because RTIC doesn't have a standard error type.
pub trait SpawnError {
    fn spawn_error(self, task: &'static str) -> Result<(), RocketError>;
}

impl<T, E> SpawnError for Result<T, E> {
    /// Converts an RTIC spawn error into a [`RocketError`]. While this function can be used on any
    /// `Result`, this should only be called on a `Result` from a `spawn` or `spawn_after` operation.
    fn spawn_error(self, task: &'static str) -> Result<(), RocketError> {
        match self {
            Ok(_) => Ok(()),
            Err(_) => Err(RocketError {
                error: RocketErrorType::SpawnError(task),
                context: None,
            }),
        }
    }
}

/// Allow the RocketErrorType to convert into an RocketError.
impl<E> From<E> for RocketError
where
    E: Into<RocketErrorType>,
{
    fn from(value: E) -> Self {
        RocketError {
            error: value.into(),
            context: None,
        }
    }
}

/// Trait to allow converting some errors to a RocketError, while also adding a ErrorContext to it.
pub trait ErrorContextTrait<T> {
    fn error_context(self, context: ErrorContext) -> Result<T, RocketError>;
}

impl<T, E> ErrorContextTrait<T> for Result<T, E>
where
    E: Into<RocketErrorType>,
{
    fn error_context(self, context: ErrorContext) -> Result<T, RocketError> {
        self.map_err(|e| RocketError {
            error: e.into(),
            context: Some(context),
        })
    }
}
