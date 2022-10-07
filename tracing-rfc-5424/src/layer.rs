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

//! [`Layer`] implementations.
//!
//! [`Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html

use crate::{
    facility::Level,
    formatter::Formatter,
    rfc3164::Rfc3164,
    rfc5424::Rfc5424,
    tracing::{TracingFormatter, TrivialTracingFormatter},
    transport::{Transport, UdpTransport, UnixSocket},
};

use backtrace::Backtrace;
use tracing::Event;
use tracing_subscriber::layer::Context;

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                       module error type                                        //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// layer error type
#[non_exhaustive]
pub enum Error {
    /// Formatting layer error
    Format {
        source: Box<dyn std::error::Error>,
        back: Backtrace,
    },
    /// Transport layer error
    Transport {
        source: crate::transport::Error,
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
            Error::Format { source, .. } => {
                write!(f, "While formatting a Span or an Event, got {}", source)
            }
            Error::Transport { source, .. } => {
                write!(f, "While sending a syslog message, got {}", source)
            }
            _ => write!(f, "syslog transport layer error"),
        }
    }
}

impl std::fmt::Debug for Error {
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Format { source: _, back } => write!(f, "{}\n{:#?}", self, back),
            Error::Transport { source: _, back } => write!(f, "{}\n{:#?}", self, back),
            _ => write!(f, "{}", self),
        }
    }
}

impl std::convert::From<crate::transport::Error> for Error {
    fn from(err: crate::transport::Error) -> Self {
        Error::Transport {
            source: err,
            back: Backtrace::new(),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                          struct Layer                                          //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A [`tracing-subscriber`]-compliant [`Layer`] implementation that will send [`Event`]s &
/// [`Span`]s to a syslog daemon.
///
/// [`tracing-subscriber`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/index.html
/// [`Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
/// [`Span`]: https://docs.rs/tracing/0.1.35/tracing/struct.Span.html
/// [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html
pub struct Layer<F1: Formatter, F2: TracingFormatter, T: Transport> {
    formatter: F1,
    map_level: Box<dyn Fn(&tracing::Level) -> Level + Send + Sync>,
    tracing_formatter: F2,
    transport: T,
}

fn default_level_mapping(level: &tracing::Level) -> Level {
    match level {
        &tracing::Level::TRACE | &tracing::Level::DEBUG => Level::LOG_DEBUG,
        &tracing::Level::INFO => Level::LOG_INFO,
        &tracing::Level::WARN => Level::LOG_WARNING,
        &tracing::Level::ERROR => Level::LOG_ERR,
    }
}

impl Layer<Rfc5424, TrivialTracingFormatter, UdpTransport> {
    /// Attempt to construct a Layer that will send RFC5424-compliant syslog messages via UDP to
    /// port 514 on localhost
    pub fn try_default() -> Result<Self> {
        Ok(Layer {
            formatter: Rfc5424::default(),
            map_level: Box::new(default_level_mapping),
            tracing_formatter: TrivialTracingFormatter,
            transport: UdpTransport::local()?,
        })
    }
}

impl Layer<Rfc3164, TrivialTracingFormatter, UnixSocket> {
    /// Attempt to construct a Layer that will send RFC3164-compliant syslog messages via datagrams
    /// to the Unix socket at `/dev/log` on localhost
    pub fn try_default() -> Result<Self> {
        Ok(Layer {
            formatter: Rfc3164::try_default().map_err(|err| Error::Format {
                source: Box::new(err),
                back: Backtrace::new(),
            })?,
            map_level: Box::new(default_level_mapping),
            tracing_formatter: TrivialTracingFormatter,
            transport: UnixSocket::try_default()?,
        })
    }
}

impl<T: Transport> Layer<Rfc5424, TrivialTracingFormatter, T> {
    /// Construct a Layer that will send RFC5424-compliant messages via transport `transport`
    pub fn with_transport(transport: T) -> Layer<Rfc5424, TrivialTracingFormatter, T> {
        Layer {
            formatter: Rfc5424::default(),
            map_level: Box::new(default_level_mapping),
            tracing_formatter: TrivialTracingFormatter,
            transport: transport,
        }
    }
}

impl<S, F1, F2, T> tracing_subscriber::layer::Layer<S> for Layer<F1, F2, T>
where
    S: tracing::Subscriber,
    F1: Formatter + 'static,
    F2: TracingFormatter + 'static,
    T: Transport + 'static,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        self.formatter
            .format_event(
                (self.map_level)(event.metadata().level()),
                event,
                &self.tracing_formatter,
                None,
            ) // :=> StdResult<Vec<u8>, <F1 as Formatter>::Error>
            .map_err(|err| Error::Format {
                source: Box::new(err),
                back: Backtrace::new(),
            })
            .and_then(|v| {
                self.transport
                    .send(&v) // :=> StdResult<u32, transport::Error>
                    .map_err(|err| err.into())
            })
            .unwrap_or_else(|_| {
                ::tracing::error!("tracing-subscriber failed");
                0
            });
    }
}

#[cfg(test)]
mod prototyping {

    use super::*;

    use tracing::Callsite;

    // I confess, `tracing` internals are a bit opaque to me, yet. In addition, they are explicitly
    // unstable. For that reason, I don't want to do too much work, here; just enough to easily give
    // myself Events against which I can test.

    struct TestCallsite {
        metadata: &'static tracing::Metadata<'static>,
    }
    impl tracing_core::callsite::Callsite for TestCallsite {
        fn set_interest(&self, _interest: tracing_core::subscriber::Interest) {}
        fn metadata(&self) -> &tracing::Metadata<'static> {
            self.metadata
        }
    }
    // I *wish* I could deal in TestCallsite instances of arbitrary lifetime, but Identifier
    // needs a reference with 'static duration.
    impl TestCallsite {
        pub const fn new(metadata: &'static tracing::Metadata<'static>) -> TestCallsite {
            TestCallsite { metadata: metadata }
        }
    }

    #[test]
    fn test_rfc_5424_impl() {
        // Just exercise `default()`; be sure it compiles & returns something sane.
        let _f = Rfc5424::default();

        let f = Rfc5424::builder()
            .hostname_as_string("bree.local".to_string())
            .unwrap()
            .appname_as_string("prototyping".to_string())
            .unwrap()
            .pid_as_string("123".to_string())
            .unwrap()
            .build();

        let fmtr = TrivialTracingFormatter;

        // Non-macro replication of the logic of `event!()`-- will need to wrap this up in a macro
        // at some point.
        static CALLSITE: TestCallsite = {
            static METADATA: tracing::Metadata = tracing::Metadata::new(
                "test event metadata",
                "test-target",
                tracing::Level::INFO,
                Some(file!()),
                Some(line!()),
                Some(module_path!()),
                // fieldset,
                tracing::field::FieldSet::new(
                    &["message"],
                    tracing_core::callsite::Identifier(&CALLSITE),
                ),
                tracing_core::metadata::Kind::EVENT,
            );
            TestCallsite::new(&METADATA)
        };

        // Would love to wrap this up into a utility function, but the Event takes the ValueSet _by
        // reference_, so we need a way to keep it alive for the lifetime of the Event. Might be
        // time to wrap this up in a macro.
        (|value_set: ::tracing::field::ValueSet| {
            let event = Event::new(CALLSITE.metadata(), &value_set);
            let rsp: Vec<u8> = f
                .format_event(
                    Level::LOG_INFO,
                    &event,
                    &fmtr,
                    Some(std::time::UNIX_EPOCH.into()),
                )
                .unwrap();

            assert_eq!(
                std::str::from_utf8(&rsp).unwrap(),
                "<14>1 1970-01-01T00:00:00+00:00 bree.local prototyping 123 - - Hello, world!"
            );
        })(tracing::valueset!(
            CALLSITE.metadata().fields(),
            "{}",
            "Hello, world!"
        ));

        (|value_set: ::tracing::field::ValueSet| {
            let event = Event::new(CALLSITE.metadata(), &value_set);
            let rsp: Vec<u8> = f
                .format_event(
                    Level::LOG_INFO,
                    &event,
                    &fmtr,
                    Some(std::time::UNIX_EPOCH.into()),
                )
                .unwrap();

            assert_eq!(
                std::str::from_utf8(&rsp).unwrap(),
                "<14>1 1970-01-01T00:00:00+00:00 bree.local prototyping 123 - - Hello, 世界!"
            );
        })(tracing::valueset!(
            CALLSITE.metadata().fields(),
            "{}",
            "Hello, 世界!"
        ));

        let f = Rfc5424::builder()
            .hostname_as_string("bree.local".to_string())
            .unwrap()
            .appname_as_string("prototyping".to_string())
            .unwrap()
            .pid_as_string("123".to_string())
            .unwrap()
            .with_bom(true)
            .build();

        (|value_set: ::tracing::field::ValueSet| {
            let event = Event::new(CALLSITE.metadata(), &value_set);
            let rsp: Vec<u8> = f
                .format_event(
                    Level::LOG_INFO,
                    &event,
                    &fmtr,
                    Some(std::time::UNIX_EPOCH.into()),
                )
                .unwrap();

            let mut golden =
                Vec::from("<14>1 1970-01-01T00:00:00+00:00 bree.local prototyping 123 - - ");
            golden.push(0xef as u8);
            golden.push(0xbb as u8);
            golden.push(0xbf as u8);
            golden.extend_from_slice("Hello, world!".as_bytes());

            assert_eq!(rsp, golden);
        })(tracing::valueset!(
            CALLSITE.metadata().fields(),
            "{}",
            "Hello, world!"
        ));
    }
}
