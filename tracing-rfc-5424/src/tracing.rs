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

//! Primitives for mapping [`tracing`] concepts to those of [`syslog-tracing`](crate).
//!
//! [`TracingFormatter`] implementations handle encoding [`Span`]s and (soon) [`Event`]s into
//! text. This module provides at this time only a single implementation:
//! [`TrivialTracingFormatter`] that simply extracts the "message" field from [`Event`]s.
//!
//! [`Span`]: https://docs.rs/tracing/0.1.35/tracing/struct.Span.html
//! [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html

use backtrace::Backtrace;

type StdResult<T, E> = std::result::Result<T, E>;

/// Format [`tracing`] [`Span`]s & [`Event`]s to UTF-8-encoded strings.
///
/// [`tracing`]: https://docs.rs/tracing/latest/tracing/index.html
/// [`Span`]: https://docs.rs/tracing/0.1.35/tracing/struct.Span.html
/// [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html
///
/// Events & Spans will typically be encoded as UTF-8, if not ASCII text. However, while RFC [3164]
/// strongly suggests ASCII, it does make certain provisions for non-ASCII text. RFC [5424]
/// explicitly suggests UTF-8, but allows for other encodings. Therefore, this trait reluctantly
/// allows for arbitrary bytes.
///
/// [3164]: https://datatracker.ietf.org/doc/html/rfc3164
/// [5424]: https://datatracker.ietf.org/doc/html/rfc5424
pub trait TracingFormatter {
    type Error: std::error::Error + 'static;
    /// Accumulate an Event into a buffer
    fn format_event(
        &self,
        event: &tracing::Event,
        buf: &mut impl bytes::BufMut,
    ) -> StdResult<(), Self::Error>;
}

#[non_exhaustive]
pub enum Error {
    NoMessageField { name: &'static str, back: Backtrace },
}

impl std::fmt::Display for Error {
    // `Error` is non-exhaustive so that adding variants won't be a breaking change to our
    // callers. That means the compiler won't catch us if we miss a variant here, so we
    // always include a `_` arm.
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NoMessageField { name, .. } => {
                write!(f, "No message field found in event {}", name)
            }
        }
    }
}

impl std::fmt::Debug for Error {
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NoMessageField { name: _, back } => write!(f, "{:#?}\n{}", back, self),
        }
    }
}

impl std::error::Error for Error {}

/// A [`TracingFormatter`] that just returns an [`Event`]s "message" field, if present (fails
/// otherwise).
///
/// [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html
pub struct TrivialTracingFormatter;

struct MessageEventVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for MessageEventVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            // Regrettably, we have only a `Debug` implementation available to us; but the tracing
            // macros `info!()`, `event!()` & the like all take care to "pre-format" the `mesage`
            // field so that `value` actually refers to a `std::fmt::Arguments` instance, which will
            // print to a debug format without enclosing double-quotes.
            self.message = Some(format!("{:?}", value));
        }
    }
}

impl TracingFormatter for TrivialTracingFormatter {
    type Error = Error;
    fn format_event(
        &self,
        event: &tracing::Event,
        buf: &mut impl bytes::BufMut,
    ) -> StdResult<(), Error> {
        let mut visitor = MessageEventVisitor { message: None };
        event.record(&mut visitor);
        visitor
            .message
            .and_then(|s| {
                buf.put_slice(s.as_bytes());
                Some(())
            })
            .ok_or(Error::NoMessageField {
                name: event.metadata().name(),
                back: Backtrace::new(),
            })
    }
}
