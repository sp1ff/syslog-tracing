// Copyright (C) 2022-2025 Michael Herstine <sp1ff@pobox.com>
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

//! Primitives for mapping [`tracing`] entities to syslog messages.
//!
//! [`TracingFormatter`] implementations handle encoding [`Event`]s and [`Span`]s into text. This
//! module provides at this time only a single implementation: [`TrivialTracingFormatter`] that
//! simply extracts the "message" field from [`Event`]s.
//!
//! [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html
//! [`Span`]: https://docs.rs/tracing/0.1.35/tracing/struct.Span.html

use crate::facility::Level;

use backtrace::Backtrace;

type StdResult<T, E> = std::result::Result<T, E>;

/// Format [`tracing`] [`Span`]s & [`Event`]s to UTF-8-encoded strings & syslog priorities.
///
/// [`tracing`]: https://docs.rs/tracing/latest/tracing/index.html
/// [`Span`]: https://docs.rs/tracing/0.1.35/tracing/struct.Span.html
/// [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html
///
/// The translation from [`tracing`] events to syslog messages occurs in three parts:
///
/// [`tracing`]: https://docs.rs/tracing/latest/tracing/index.html
///
/// 1. formatting the Span or Event to a textual message
///
/// 2. incorporating that message into a syslog packet compliant with your daemon's implementation
///
/// 3. transporting that packet to your daemon
///
/// Trait [`TracingFormatter`] formally defines step 1: implementations shall provide methods that
/// will be invoked upon various [`tracing`] events ("span entered", "span exited", "event", and so
/// forth); each method will indicate, firstly, whether this event shall produce a [`syslog`] log
/// message, and if so, what the message field of that log line shall be.
///
/// [`tracing`]: https://docs.rs/tracing/latest/tracing/index.html
/// [`syslog`]: https://en.wikipedia.org/wiki/Syslog
///
/// RFCs 3164 & 5424 mostly differ in the details of the fields that comprise the packet, but both
/// contain a "message" field. But what is a "message"? Well, according to RFC 3164:
///
/// - The MSG part of the syslog packet MUST contain visible (printing) characters.
///
/// - The code set traditionally and most often used has also been seven-bit ASCII in an eight-bit
///   field. In this code set, the only allowable characters are the ABNF VCHAR values (%d33-126) and
///   spaces (SP value %d32).
///
/// According to RFC 5424:
///
/// - The character set used in MSG SHOULD be UNICODE, encoded using UTF-8 as specified in
///   RFC 3629.  If the syslog application cannot encode the MSG in Unicode, it MAY use any other
///   encoding.
///
/// - The syslog application SHOULD avoid octet values below 32 (the traditional US-ASCII control
///   character range except DEL).  These values are legal, but a syslog application MAY modify these
///   characters upon reception.  For example, it might change them into an escape sequence (e.g.,
///   value 0 may be changed to "\0").  A syslog application SHOULD NOT modify any other octet values.
///
/// In other words, the two RFCs see the "message" as free-form; their differences seem to
/// come-down to textual encoding. Therefore, this trait concerns itself simply with translating
/// from [`tracing`] entities to a UTF-8-encoded messages. Downstream, particular implementations can do
/// as they see fit with it, enabling code like:
///
/// ```text
///  fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
///      self.tracing_formatter.on_event(event, ctx)?
///           .and_then(|text| self.syslog_formatter.format(text)?)
///           .and_then(|thing| self.transport.send(thing)?)
///  }
/// ```
pub trait TracingFormatter<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    type Error: std::error::Error + 'static;
    /// An event has occurred
    fn on_event(
        &self,
        event: &tracing::Event,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> StdResult<Option<(String, Level)>, Self::Error>;
    /// A span with the given ID was entered
    fn on_enter(
        &self,
        _id: &tracing_core::span::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> StdResult<Option<(String, Level)>, Self::Error> {
        Ok(Option::None)
    }
    /// A span with the given ID was exited
    fn on_exit(
        &self,
        _id: &tracing_core::span::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> StdResult<Option<(String, Level)>, Self::Error> {
        Ok(Option::None)
    }
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

fn default_level_mapping(level: &tracing::Level) -> Level {
    match level {
        &tracing::Level::TRACE | &tracing::Level::DEBUG => Level::LOG_DEBUG,
        &tracing::Level::INFO => Level::LOG_INFO,
        &tracing::Level::WARN => Level::LOG_WARNING,
        &tracing::Level::ERROR => Level::LOG_ERR,
    }
}

/// A [`TracingFormatter`] that just returns an [`Event`]s "message" field, if present (fails
/// otherwise). It doesn't respond to any other events.
///
/// [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html
pub struct TrivialTracingFormatter {
    map_level: Box<dyn Fn(&tracing::Level) -> Level + Send + Sync>,
}

impl std::default::Default for TrivialTracingFormatter {
    fn default() -> Self {
        TrivialTracingFormatter {
            map_level: Box::new(default_level_mapping),
        }
    }
}

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

impl<S> TracingFormatter<S> for TrivialTracingFormatter
where
    S: tracing_core::subscriber::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    type Error = Error;
    fn on_event(
        &self,
        event: &tracing::Event,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> StdResult<Option<(String, Level)>, Error> {
        let mut visitor = MessageEventVisitor { message: None };
        event.record(&mut visitor);
        visitor
            .message
            .ok_or(Error::NoMessageField {
                name: event.metadata().name(),
                back: Backtrace::new(),
            })
            .map(|s| Some((s, (*self.map_level)(event.metadata().level()))))
    }
}
