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

//! Layer implementations

use crate::error::Result;
use crate::facility::Level;
use crate::formatter::Formatter;
use crate::rfc5424::Rfc5424;
use crate::tracing::{TracingFormatter, TrivialTracingFormatter};
use crate::transport::{Transport, UdpTransport};

use tracing::Event;
use tracing_subscriber::layer::Context;

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
    pub fn default() -> Result<Self> {
        let transport = UdpTransport::local()?;
        Ok(Layer {
            formatter: Rfc5424::default(),
            map_level: Box::new(default_level_mapping),
            tracing_formatter: TrivialTracingFormatter,
            transport: transport,
        })
    }
}

impl<T: Transport> Layer<Rfc5424, TrivialTracingFormatter, T> {
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
            )
            .and_then(|v| self.transport.send(&v))
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

    use tracing::{debug, error, info, trace, warn};
    use tracing_subscriber::{
        layer::SubscriberExt, // Needed to get `with()`
        registry::Registry,
    };

    #[test]
    #[cfg(feature = "rsyslogd")]
    fn test_tracing_via_udp() {
        // Exercise `default()`, just to be sure it compiles.
        let _subscriber = Registry::default().with(Layer::default().unwrap());

        // Setup the real subsriber...
        let subscriber = Registry::default().with(Layer::with_transport(
            UdpTransport::new("127.0.0.1:5514").unwrap(),
        ));
        // and install it.
        let _guard = tracing::subscriber::set_default(subscriber);

        trace!("Hello, 世界!");
        debug!("Hello, 世界!");
        info!("Hello, 世界!");
        warn!("Hello, 世界!");
        error!("Hello, 世界!");
    }
}
