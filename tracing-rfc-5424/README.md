tracing-rfc-5424
================

# Introduction

[tracing-rfc-5424](https://github.com/sp1ff/syslog-tracing/tracing-rfc-5424) is a [tracing-subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/index.html) [Layer](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html) implementation that sends [tracing](https://docs.rs/tracing/latest/tracing/index.html)[Event](https://docs.rs/tracing/latest/tracing/struct.Event.html)s  to a [syslog](https://en.wikipedia.org/wiki/Syslog) [daemon](https://en.wikipedia.org/wiki/Daemon_(computing)).

[tracing](https://docs.rs/tracing/latest/tracing/index.html) is a "scoped, structured logging and diagnostics system". It provides a superset of the features offered by logging crates such as [fern](https://docs.rs/fern/latest/fern/index.html) and [log4rs](https://docs.rs/log4rs/latest/log4rs/) particularly suited to asynchronous programming. Of particular interest is that it makes a very clear distinction between producers of events & their consumers ([Subscriber](https://docs.rs/tracing/0.1.34/tracing/trait.Subscriber.html)s, in [tracing](https://docs.rs/tracing/latest/tracing/index.html) parlance); so much so that the [tracing](https://docs.rs/tracing/latest/tracing/index.html) crate provides no support for <span class="underline">consuming</span> events, other than the definition of the [Subscriber](https://docs.rs/tracing/0.1.34/tracing/trait.Subscriber.html) trait.

The [tracing-subscriber](https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/index.html) crate (also part of the [Tokio](https://tokio.rs/) project) provides a few basic implementations along with "utilities for implementing and composing tracing subscribers." A key notion introduced by this crate is the idea of a [Layer](https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/layer/trait.Layer.html). "Unlike Subscribers, which implement a complete strategy for how trace data is collected, Layers provide modular implementations of specific behaviors." The concern here is that, in general, a consumer of [tracing](https://docs.rs/tracing/latest/tracing/index.html) event data may want to do various things at the same time with that data: write it to a log file, just write it to `stdout`, shuttle it off to a log collection daemon, produce metrics based on it, and so on.

This could easily give rise to a geometric explosion in types: `LogFile`, `LogFileWithStdout`, `LogFileWithLogStash`, `LogFileWithLogStashAndStdout`, and so forth. The idea behind [Layer](https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/layer/trait.Layer.html)s is to decompose each facet of event handling into its own type, and then "stack them up" in a [Subscriber](https://docs.rs/tracing/0.1.34/tracing/trait.Subscriber.html) as the application developer desires. In a sense, this design pattern is reminiscent of [Alexandrescu](https://en.wikipedia.org/wiki/Andrei_Alexandrescu)'s concept of [traits classes](https://erdani.com/publications/traits.html) in C++&#x2013; a way of decomposing functionality into orthogonal facets and composing them linearly rather than geometrically.

[tracing-rfc-5424](https://github.com/sp1ff/syslog-tracing/tracing-rfc-5424) is a [Layer](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html) implementation that sends [Event](https://docs.rs/tracing/latest/tracing/struct.Event.html)s (and, soon, [Span](https://docs.rs/tracing/latest/tracing/struct.Span.html)s) to a syslog daemon. Both RFCs [3164](https://www.rfc-editor.org/rfc/rfc3164) (BSD syslog) and [5424](https://www.rfc-editor.org/rfc/rfc5424.html) are supported. Internet & Unix domain sockets are supported for transport, both in their datagram & stream flavors.
# License

This crate is released under the [GPL 3.0](https://spdx.org/licenses/GPL-3.0-or-later.html).
# Prerequisites

Other than [tracing](https://github.com/tokio-rs/tracing) and a syslog daemon, none. [tracing-rfc-5424](https://github.com/sp1ff/syslog-tracing/tracing-rfc-5424) was developed against rust 1.58.1, although [tracing](https://github.com/tokio-rs/tracing) requires 1.49.

# Installation

To add [tracing-rfc-5424](https://github.com/sp1ff/syslog-tracing/tracing-rfc-5424) to your crate, just say `cargo add tracing-rfc-5424`, or add it directly to your `Cargo.toml`:

    [dependencies]
    tracing-rfc-5424 = "0.1.1"
# Usage

To talk to a local syslog daemon over UDP:

    use tracing::info;
    use tracing_rfc_5424::{
        layer::Layer, rfc5424::Rfc5424, tracing::TrivialTracingFormatter, transport::UdpTransport,
    };
    use tracing_subscriber::{
        layer::SubscriberExt, // Needed to get `with()`
        registry::Registry,
    };
    
    // Setup the subsriber...
    let subscriber = Registry::default()
        .with(Layer::<Layer::<tracing_subscriber::Registry, Rfc5424, TrivialTracingFormatter, UdpTransport>::try_default().unwrap());
    // and install it.
    let _guard = tracing::subscriber::set_default(subscriber);
    
    info!("Hello, world!");
    // Will produce a syslog line something like:
    // Jun 23 16:10:55 my-host my-app[pid] Hello, world!

To talk to a local Unix socket:

    use tracing::info;
    use tracing_rfc_5424::{rfc3164::Rfc3164, tracing::TrivialTracingFormatter, transport::UnixSocket};
    use tracing_subscriber::{
        layer::SubscriberExt, // Needed to get `with()`
        registry::Registry,
    };

      // Setup the subsriber...
    let subscriber = subscriber
        .with(tracing_rfc_5424::layer::Layer::<Layer::<tracing_subscriber::Registry, Rfc3164, TrivialTracingFormatter, UnixSocket>::try_default().unwrap());
    // and install it.
    let _guard = tracing::subscriber::set_default(subscriber);

    info!("Hello, world!");
# Status, Rationale and Roadmap

This is a preliminary release; the version number (0.1.x) is intended to convey that. See this crate's parent [workspace](https://github.com/sp1ff/syslog-tracing) for more on the roadmap.

Bugs, comments, problems, criticism, PRs, feature requests &c welcome at [sp1ff@pobox.com](mailto:sp1ff@pobox.com) and in the [issues](https://github.com/sp1ff/syslog-tracing/issues).

