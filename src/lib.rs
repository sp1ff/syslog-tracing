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
//! A [`tracing-subscriber`] [`Layer`] implementation for sending [`tracing`] [`Span`]s and
//! [`Event`]s to a [`syslog`] [daemon]
//!
//! [`tracing-subscriber`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/index.html
//! [`Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
//! [`tracing`]: https://docs.rs/tracing/0.1.35/tracing/index.html
//! [`Span`]: https://docs.rs/tracing/0.1.35/tracing/struct.Span.html
//! [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html
//! [`syslog`]: https://en.wikipedia.org/wiki/Syslog
//! [daemon]: https://en.wikipedia.org/wiki/Daemon_(computing)
//!
//! # Introduction
//!
//! The [`tracing`] crate is a "scoped, structured logging and diagnostics system". It provides a
//! superset of the features offered by logging crates such as [fern] and [log4rs] particularly
//! suited to asynchronous programming. Of particular interest here is that it makes a very clear
//! distinction between producers of events & their consumers ([`Subscriber`]s, in [`tracing`]
//! parlance); so much so that the [`tracing`] crate provides no support for _consuming_ events,
//! other than the definition of the [`Subscriber`] trait.
//!
//! [`tracing`]: https://docs.rs/tracing/latest/tracing/index.html
//! [fern]: https://docs.rs/fern/latest/fern/index.html
//! [log4rs]: https://docs.rs/log4rs/latest/log4rs/
//! [`Subscriber`]: https://docs.rs/tracing/0.1.34/tracing/trait.Subscriber.html
//!
//! The [`tracing-subscriber`] crate (also part of the [Tokio] project) provides a few basic
//! implementations along with "utilities for implementing and composing tracing subscribers." A key
//! notion introduced by this crate is the idea of a [`Layer`]. "Unlike Subscribers, which implement
//! a complete strategy for how trace data is collected, Layers provide modular implementations of
//! specific behaviors." The concern here is that, in general, a consumer of [`tracing`] event data
//! may want to do various things at the same time with that data: write it to a log file, just write
//! it to `stdout`, shuttle it off to a log collection daemon, and so on.
//!
//! [`tracing-subscriber`]: https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/index.html
//! [Tokio]: https://tokio.rs/
//! [`Layer`]: https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/layer/trait.Layer.html
//!
//! This could easily give rise to a geometric explosion in types: `LogFile`, `LogFileWithStdout`,
//! `LogFileWithLogStash`, `LogFileWithLogStashAndStdout`, and so forth. The idea behind [`Layer`]s
//! is to decompose each facet of event handling into its own type, and then "stack them up" in a
//! [`Subscriber`] as the application developer desires. In a sense, this design pattern is
//! reminiscent of [Alexanrescu]'s concept of [traits classes] in C++-- a way of decomposing
//! functionality into distinct facets and composing them linearly rather than geometrically.
//!
//! [`Layer`]: https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/layer/trait.Layer.html
//! [Alexanrescu]: https://en.wikipedia.org/wiki/Andrei_Alexandrescu
//! [traits classes]: https://erdani.com/publications/traits.html
//!
//! This crate provides a [`Layer`] implementation for dispatching [`tracing`] events to a
//! [`syslog`] daemon.
//!
//! [`Layer`]: https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/layer/trait.Layer.html
//! [`tracing`]: https://docs.rs/tracing/latest/tracing/index.html
//! [`syslog`]: https://en.wikipedia.org/wiki/Syslog
//!
//! # Usage
//!
//! [`tracing-syslog`](crate)'s [`Layer`] comes with sane defaults:
//!
//! ```rust
//! use tracing::info;
//! use syslog_tracing::layer::Layer;
//! use tracing_subscriber::registry::Registry;
//! use tracing_subscriber::layer::SubscriberExt; // Needed to get `with()`
//!
//! // The default configuration is to format syslog messages as per RFC 5424
//! // and to send them via UDP to port 514 on the localhost.
//! let subscriber = Registry::default().with(Layer::default().unwrap());
//!
//! info!("Hello, world!");
//! ```
//!
//! Will produce syslog entries that look something like this:
//!
//! ```text
//! Jun 23 16:10:55 hostname appname[pid] Hello, world!
//! ```
//!
//! That said, the transport layer, the syslog formatting and the means of formatting [`tracing`]
//! [`Event`]s & [`Span`]s are configurable:
//!
//! [`tracing`]: https://docs.rs/tracing/latest/tracing/index.html
//! [`Event`]: https://docs.rs/tracing/0.1.35/tracing/struct.Event.html
//! [`Span`]: https://docs.rs/tracing/0.1.35/tracing/struct.Span.html
//!
//! ```no_run
//! use tracing::info;
//! use syslog_tracing::layer::Layer;
//! use syslog_tracing::transport::UdpTransport;
//! use tracing_subscriber::registry::Registry;
//! use tracing_subscriber::layer::SubscriberExt; // Needed to get `with()`
//!
//! // The default configuration is to format syslog messages as per RFC 5424
//! // and to send them via UDP to port 514 on the localhost.
//! let subscriber = Registry::default().with(Layer::with_transport(
//!     UdpTransport::new("some.other.host:5514").unwrap()));
//!
//! info!("Hello, world!");
//! ```
//!
//! Will send the syslog packet to a daemon on port 5514 on some.other.host.

pub mod error;
pub mod facility;
pub mod formatter;
pub mod layer;
pub mod rfc5424;
pub mod tracing;
pub mod transport;
