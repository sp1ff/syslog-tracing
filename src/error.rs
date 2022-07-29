// Copyright (C) 2022 Michael Herstine <sp1ff@pobox.com>
//
// This file is part of syslog-tracing.
//
// syslog-tracing is free software: you can redistribute it and/or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// mpdpopm is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even
// the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General
// Public License for more details.
//
// You should have received a copy of the GNU General Public License along with mpdpopm.  If not,
// see <http://www.gnu.org/licenses/>.
//! [syslog-tracing](crate) errors

use backtrace::Backtrace;

/// [tracing-syslog](crate) error type
///
/// [tracing-syslog](crate) eschews libraries like [thiserror], [anyhow] & [Snafu] in favor of
/// a straightforward enumeration with a few match arms chosen on the basis what the caller will
/// need to repond.
///
/// [thiserror]: https://docs.rs/thiserror
/// [anyhow]: https://docs.rs/anyhow
/// [Snafu]: https://docs.rs/snafu/latest/snafu
#[non_exhaustive]
pub enum Error {
    BadRfc5424AppName {
        name: Vec<u8>,
        back: Backtrace,
    },
    BadRfc5424Hostname {
        name: Vec<u8>,
        back: Backtrace,
    },
    BadRfc5424IpAddress,
    BadRfc5424ProcId {
        name: Vec<u8>,
        back: Backtrace,
    },
    /// Failed to fetch the current executable (via std::env)
    NoExecutable {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        back: Backtrace,
    },
    /// Failed to fetch hostname (via libc)
    NoHostname {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        back: Backtrace,
    },
    /// An Event had no message field
    NoMessageField {
        name: &'static str,
        back: Backtrace,
    },
    /// General transport layer error
    Transport {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        back: Backtrace,
    },
}

impl std::fmt::Display for Error {
    // `Error` is non-exhaustive so that adding variants won't be a breaking change to our
    // callers. That means the compiler won't catch us if we miss a variant here, so we
    // always include a `_` arm.
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BadRfc5424Hostname { .. } => {
                write!(
                    f,
                    "The provided or discovered hostname is not compliant with RFC 5424"
                )
            }
            Error::BadRfc5424IpAddress => {
                write!(
                    f,
                    "The provided or discovered IP address is not compliant with RFC 5424"
                )
            }
            Error::NoMessageField { name, .. } => write!(
                f,
                "Event '{}' had no message field, and so was not forwarded to a syslog daemon",
                name
            ),
            Error::Transport { source, .. } => write!(f, "Transport error: {:?}", source),
            _ => write!(f, "Other tracing-sylog error"),
        }
    }
}

impl std::fmt::Debug for Error {
    // `Error` is non-exhaustive so that adding variants won't be a breaking change to our
    // callers. That means the compiler won't catch us if we miss a variant here, so we
    // always include a `_` arm.
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BadRfc5424Hostname { name: _, back } => write!(f, "{}\n{:?}", self, back),
            Error::BadRfc5424IpAddress => write!(f, "{}", self),
            Error::NoMessageField { name: _, back } => write!(f, "{}\n{:?}", self, back),
            Error::Transport { source: _, back } => write!(f, "{}\n{:?}", self, back),
            err => write!(f, "tracing-syslog error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
