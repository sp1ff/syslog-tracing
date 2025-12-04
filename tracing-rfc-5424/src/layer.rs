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

//! [tracing-rfc-5424](crate) [`Layer`] implementations.
//!
//! [`Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
//!
//! A basic struct [`Layer`] is defined, but implementations are provided only for a few (sensible)
//! combinations of type parameters. Consumers of this crate are of course free to implement the
//! [`TracingFormatter`], [`SyslogFormatter`] and [`Transport`] traits for themselves & provide
//! their own implementations.

use crate::{
    formatter::SyslogFormatter,
    rfc3164::Rfc3164,
    rfc5424::Rfc5424,
    tracing::{TracingFormatter, TrivialTracingFormatter},
    transport::{Transport, UdpTransport},
};

#[cfg(unix)]
use crate::transport::UnixSocket;

use backtrace::Backtrace;
use tracing::Event;
use tracing_subscriber::layer::Context;

// When the tracing-log feature is enabled, use NormalizeEvent to extract file/line metadata
// from events that originated from the `log` crate. This follows the same pattern used by
// tracing-subscriber's fmt layer.
// See: https://github.com/tokio-rs/tracing/blob/master/tracing-subscriber/src/fmt/fmt_layer.rs
#[cfg(feature = "tracing-log")]
use tracing_log::NormalizeEvent;

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                       module error type                                        //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// module error type
#[non_exhaustive]
pub enum Error {
    /// Formatting layer error
    Format {
        source: Box<dyn std::error::Error>,
        back: Backtrace,
    },
    /// Transport layer error
    Transport {
        source: Box<dyn std::error::Error>,
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
pub struct Layer<S, F1: SyslogFormatter, F2: TracingFormatter<S>, T: Transport<F1>>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    syslog_formatter: F1,
    tracing_formatter: F2,
    transport: T,
    // I need the Subscriber implementation type as a type parameter to transmit it to the
    // TracingFormatter trait. 👇 gets the compiler to shut-up about unused type parameters.
    subscriber_type: std::marker::PhantomData<S>,
}

/// A [`Layer`] implementation with the following characteristics:
///
/// - Uses the "trivial" formatter for mapping from Tracing evengs to messages
/// - Speaks RFC 5424 for syslog
/// - Sends the resulting messages over UDP
///
/// May be used with any [`tracing_subscriber::Subscriber`] implementation that supports
/// [`LookupSpan`].
///
/// [`tracing_subscriber::Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
/// [`LookupSpan`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/registry/trait.LookupSpan.html
impl<S> Layer<S, Rfc5424, TrivialTracingFormatter, UdpTransport>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    /// Attempt to construct a [`Layer`] that will send RFC5424-compliant syslog messages via UDP to
    /// port 514 on localhost
    pub fn try_default() -> Result<Self> {
        Ok(Layer {
            syslog_formatter: Rfc5424::default(),
            tracing_formatter: TrivialTracingFormatter::default(),
            transport: UdpTransport::local().map_err(|err| Error::Transport {
                source: Box::new(err),
                back: Backtrace::new(),
            })?,
            subscriber_type: std::marker::PhantomData,
        })
    }
}

/// A [`Layer`] implementation with the following characteristics:
///
/// - Uses the "trivial" formatter for mapping from Tracing evengs to messages
/// - Speaks RFC 3164 for syslog
/// - Sends the resulting messages over a local Unix Domain socket
///
/// May be used with any [`tracing_subscriber::Subscriber`] implementation that supports
/// [`LookupSpan`].
///
/// [`tracing_subscriber::Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
/// [`LookupSpan`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/registry/trait.LookupSpan.html
#[cfg(unix)]
impl<S> Layer<S, Rfc3164, TrivialTracingFormatter, UnixSocket>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    /// Attempt to construct a Layer that will send RFC3164-compliant syslog messages via datagrams
    /// to the Unix socket at `/dev/log` on localhost
    pub fn try_default() -> Result<Self> {
        Ok(Layer {
            syslog_formatter: Rfc3164::try_default().map_err(|err| Error::Format {
                source: Box::new(err),
                back: Backtrace::new(),
            })?,
            tracing_formatter: TrivialTracingFormatter::default(),
            transport: UnixSocket::try_default().map_err(|err| Error::Transport {
                source: Box::new(err),
                back: Backtrace::new(),
            })?,
            subscriber_type: std::marker::PhantomData,
        })
    }
}

impl<S, T: Transport<Rfc5424>, TF: TracingFormatter<S>> Layer<S, Rfc5424, TF, T>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    /// construct Layer with custom inners
    pub fn new(syslog_formatter: Rfc5424, tracing_formatter: TF, transport: T) -> Self {
        Layer {
            syslog_formatter,
            tracing_formatter,
            transport,
            subscriber_type: std::marker::PhantomData,
        }
    }
}

/// Customize a [`Layer`] implementation with the following characteristics:
///
/// - Uses the "trivial" formatter for mapping from Tracing evengs to messages
/// - Speaks RFC 5424 for syslog
/// - Sends the resulting messages over UDP
///
/// With a custom [`Transport`] implementation.  May be used with any
/// [`tracing_subscriber::Subscriber`] implementation that supports [`LookupSpan`].
///
/// [`tracing_subscriber::Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
/// [`LookupSpan`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/registry/trait.LookupSpan.html
impl<S, T: Transport<Rfc5424>> Layer<S, Rfc5424, TrivialTracingFormatter, T>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    /// Construct a Layer that will send RFC5424-compliant messages via transport `transport`
    pub fn with_transport(transport: T) -> Self {
        Layer {
            syslog_formatter: Rfc5424::default(),
            tracing_formatter: TrivialTracingFormatter::default(),
            transport,
            subscriber_type: std::marker::PhantomData,
        }
    }

    /// Construct a Layer that will send RFC5424-compliant messages via transport `transport`
    pub fn with_transport_and_syslog_formatter(transport: T, formatter: Rfc5424) -> Self {
        Layer {
            syslog_formatter: formatter,
            tracing_formatter: TrivialTracingFormatter::default(),
            transport,
            subscriber_type: std::marker::PhantomData,
        }
    }
}

/// This is the Big Tuna-- the [`Layer`] implementation.
///
/// [`Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
impl<S, F1, F2, T> tracing_subscriber::layer::Layer<S> for Layer<S, F1, F2, T>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    F1: SyslogFormatter + 'static,
    F2: TracingFormatter<S> + 'static,
    T: Transport<F1> + 'static,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // When the tracing-log feature is enabled, use normalized_metadata() to get
        // file/line info for events that originated from the `log` crate.
        // For native tracing events, normalized_metadata() returns None and we use
        // the event's own metadata.
        // See: https://github.com/tokio-rs/tracing/blob/9978c3663bcd58de14b3cf089ad24cb63d00a922/tracing-subscriber/src/fmt/format/pretty.rs#L182
        #[cfg(feature = "tracing-log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "tracing-log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "tracing-log"))]
        let meta = event.metadata();

        self.tracing_formatter
            .on_event(event, ctx) // :=> StdResult<Option<(String, Level)>, <F1 as SyslogFormatter>::Error>
            .map_err(|err| Error::Format {
                source: Box::new(err),
                back: Backtrace::new(),
            }) // 👈:=> StdResult<Option<(String, Level)>, Error>
            .and_then(|x| {
                // x is an Option<(String, Level)>
                if let Some((msg, level)) = x {
                    Ok(self
                        .transport
                        .send(
                            self.syslog_formatter
                                .format(level, &msg, None, meta)
                                .map_err(|err| Error::Format {
                                    source: Box::new(err),
                                    back: Backtrace::new(),
                                })?,
                        )
                        .map_err(|err| Error::Transport {
                            source: Box::new(err),
                            back: Backtrace::new(),
                        })?)
                } else {
                    Ok(())
                }
            })
            .unwrap_or_else(|_err| {
                ::tracing::error!("tracing-subscriber failed");
            })
    }
}

#[cfg(test)]
mod smoke {

    use super::*;

    use crate::facility::Level;

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
            TestCallsite { metadata }
        }
    }

    #[test]
    #[allow(clippy::redundant_closure_call)]
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

        let _fmtr = TrivialTracingFormatter::default();

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
            let _event = Event::new(CALLSITE.metadata(), &value_set);
            let rsp: Vec<u8> = f
                .format(
                    Level::LOG_INFO,
                    "Hello, world!",
                    Some(std::time::UNIX_EPOCH.into()),
                    CALLSITE.metadata(),
                )
                .unwrap();

            assert_eq!(
                std::str::from_utf8(&rsp).unwrap(),
                "<14>1 1970-01-01T00:00:00.000000+00:00 bree.local prototyping 123 - - Hello, world!"
            );
        })(tracing::valueset!(
            CALLSITE.metadata().fields(),
            "{}",
            "Hello, world!"
        ));

        (|value_set: ::tracing::field::ValueSet| {
            let _event = Event::new(CALLSITE.metadata(), &value_set);
            let rsp: Vec<u8> = f
                .format(
                    Level::LOG_INFO,
                    "Hello, 世界!",
                    Some(std::time::UNIX_EPOCH.into()),
                    CALLSITE.metadata(),
                )
                .unwrap();

            assert_eq!(
                std::str::from_utf8(&rsp).unwrap(),
                "<14>1 1970-01-01T00:00:00.000000+00:00 bree.local prototyping 123 - - Hello, 世界!"
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
            let _event = Event::new(CALLSITE.metadata(), &value_set);
            let rsp: Vec<u8> = f
                .format(
                    Level::LOG_INFO,
                    "Hello, world!",
                    Some(std::time::UNIX_EPOCH.into()),
                    CALLSITE.metadata(),
                )
                .unwrap();

            let mut golden =
                Vec::from("<14>1 1970-01-01T00:00:00.000000+00:00 bree.local prototyping 123 - - ");
            golden.push(0xef_u8);
            golden.push(0xbb_u8);
            golden.push(0xbf_u8);
            golden.extend_from_slice("Hello, world!".as_bytes());

            assert_eq!(rsp, golden);
        })(tracing::valueset!(
            CALLSITE.metadata().fields(),
            "{}",
            "Hello, world!"
        ));
    }

    #[test]
    fn test_structured_data() {
        // Test with include_target enabled
        let f = Rfc5424::builder()
            .hostname_as_string("bree.local".to_string())
            .unwrap()
            .appname_as_string("prototyping".to_string())
            .unwrap()
            .pid_as_string("123".to_string())
            .unwrap()
            .with_tracing_target(true)
            .build();

        static CALLSITE: TestCallsite = {
            static METADATA: tracing::Metadata = tracing::Metadata::new(
                "test event metadata",
                "test-target",
                tracing::Level::INFO,
                Some(file!()),
                Some(line!()),
                Some(module_path!()),
                tracing::field::FieldSet::new(
                    &["message"],
                    tracing_core::callsite::Identifier(&CALLSITE),
                ),
                tracing_core::metadata::Kind::EVENT,
            );
            TestCallsite::new(&METADATA)
        };

        let rsp: Vec<u8> = f
            .format(
                Level::LOG_INFO,
                "Hello, world!",
                Some(std::time::UNIX_EPOCH.into()),
                CALLSITE.metadata(),
            )
            .unwrap();

        assert_eq!(
            std::str::from_utf8(&rsp).unwrap(),
            "<14>1 1970-01-01T00:00:00.000000+00:00 bree.local prototyping 123 - [tracing-meta@64700 target=\"test-target\"] Hello, world!"
        );

        // Test with include_source_location enabled
        let f_loc = Rfc5424::builder()
            .hostname_as_string("bree.local".to_string())
            .unwrap()
            .appname_as_string("prototyping".to_string())
            .unwrap()
            .pid_as_string("123".to_string())
            .unwrap()
            .with_tracing_source_location(true)
            .build();

        let rsp: Vec<u8> = f_loc
            .format(
                Level::LOG_INFO,
                "Hello, world!",
                Some(std::time::UNIX_EPOCH.into()),
                CALLSITE.metadata(),
            )
            .unwrap();

        let output = std::str::from_utf8(&rsp).unwrap();
        // Should contain file and line but not target
        let expected_file = CALLSITE.metadata().file().unwrap();
        let expected_line = CALLSITE.metadata().line().unwrap();
        let expected = format!(
            "<14>1 1970-01-01T00:00:00.000000+00:00 bree.local prototyping 123 - [tracing-meta@64700 file=\"{}\" line=\"{}\"] Hello, world!",
            expected_file, expected_line
        );
        assert_eq!(output, expected);

        // Test with include_module enabled
        let f_module = Rfc5424::builder()
            .hostname_as_string("bree.local".to_string())
            .unwrap()
            .appname_as_string("prototyping".to_string())
            .unwrap()
            .pid_as_string("123".to_string())
            .unwrap()
            .with_tracing_module(true)
            .build();

        let rsp: Vec<u8> = f_module
            .format(
                Level::LOG_INFO,
                "Hello, world!",
                Some(std::time::UNIX_EPOCH.into()),
                CALLSITE.metadata(),
            )
            .unwrap();

        let output = std::str::from_utf8(&rsp).unwrap();
        // Should contain module but not target or file/line
        let expected_module = CALLSITE.metadata().module_path().unwrap();
        let expected = format!(
            "<14>1 1970-01-01T00:00:00.000000+00:00 bree.local prototyping 123 - [tracing-meta@64700 module=\"{}\"] Hello, world!",
            expected_module
        );
        assert_eq!(output, expected);

        // Test with both target and source_location enabled
        let f_both = Rfc5424::builder()
            .hostname_as_string("bree.local".to_string())
            .unwrap()
            .appname_as_string("prototyping".to_string())
            .unwrap()
            .pid_as_string("123".to_string())
            .unwrap()
            .with_tracing_target(true)
            .with_tracing_source_location(true)
            .build();

        let rsp: Vec<u8> = f_both
            .format(
                Level::LOG_INFO,
                "Hello, world!",
                Some(std::time::UNIX_EPOCH.into()),
                CALLSITE.metadata(),
            )
            .unwrap();

        let output = std::str::from_utf8(&rsp).unwrap();
        // Should contain target and location, but not module
        let expected_file = CALLSITE.metadata().file().unwrap();
        let expected_line = CALLSITE.metadata().line().unwrap();
        let expected = format!(
            "<14>1 1970-01-01T00:00:00.000000+00:00 bree.local prototyping 123 - [tracing-meta@64700 target=\"test-target\" file=\"{}\" line=\"{}\"] Hello, world!",
            expected_file, expected_line
        );
        assert_eq!(output, expected);

        // Test with all metadata enabled
        let f_all = Rfc5424::builder()
            .hostname_as_string("bree.local".to_string())
            .unwrap()
            .appname_as_string("prototyping".to_string())
            .unwrap()
            .pid_as_string("123".to_string())
            .unwrap()
            .with_tracing_target(true)
            .with_tracing_module(true)
            .with_tracing_source_location(true)
            .build();

        let rsp: Vec<u8> = f_all
            .format(
                Level::LOG_INFO,
                "Hello, world!",
                Some(std::time::UNIX_EPOCH.into()),
                CALLSITE.metadata(),
            )
            .unwrap();

        let output = std::str::from_utf8(&rsp).unwrap();
        // Should contain all metadata fields
        let expected_module = CALLSITE.metadata().module_path().unwrap();
        let expected_file = CALLSITE.metadata().file().unwrap();
        let expected_line = CALLSITE.metadata().line().unwrap();
        let expected = format!(
            "<14>1 1970-01-01T00:00:00.000000+00:00 bree.local prototyping 123 - [tracing-meta@64700 target=\"test-target\" module=\"{}\" file=\"{}\" line=\"{}\"] Hello, world!",
            expected_module, expected_file, expected_line
        );
        assert_eq!(output, expected);
    }

    /// Test for issue #14 regression: timestamp fractional seconds should not exceed 6 digits
    #[test]
    fn test_against_issue_014_regression() {
        use crate::facility::Facility;

        static CALLSITE: TestCallsite = {
            static METADATA: tracing::Metadata = tracing::Metadata::new(
                "issue014 test",
                "test-target",
                tracing::Level::INFO,
                Some(file!()),
                Some(line!()),
                Some(module_path!()),
                tracing::field::FieldSet::new(
                    &["message"],
                    tracing_core::callsite::Identifier(&CALLSITE),
                ),
                tracing_core::metadata::Kind::EVENT,
            );
            TestCallsite::new(&METADATA)
        };

        let test_message = String::from_utf8(
            Rfc5424::builder()
                .facility(Facility::LOG_USER)
                .hostname_as_string("bree".to_owned())
                .unwrap()
                .appname_as_string("unit test suite".to_owned())
                .unwrap()
                .build()
                .format(
                    Level::LOG_NOTICE,
                    "This is a test message; its timestamp had better not have more than 6 digits in the fractional seconds place",
                    None,
                    CALLSITE.metadata(),
                )
                .unwrap(),
        )
        .unwrap();

        eprintln!("Test message: {test_message}\n");
        let i = test_message.find('.').unwrap();
        let j = test_message.find('+').unwrap();
        assert!(
            j - i - 1 <= 6,
            "Fractional seconds should not exceed 6 digits"
        );
    }
}
