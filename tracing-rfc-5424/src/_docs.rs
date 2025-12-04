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

//! # General tracing-rfc-5424 Documenation
//!
//! ## Introduction
//!
//! General (i.e. not documenting a particular struct or a method) documentation goes here. It's
//! bare bones at the moment, but I hope to develop it over time.
//!
//! ## From tracing Events to syslog Messages
//!
//! The translation from tracing [Event]s to syslog messages happens in three steps:
//!
//! [Event]: tracing::Event
//!
//! 1. formatting the [Event] to a textual message (and syslog-compliant verbosity level)
//! 2. incorporating that message into a syslog packet compliant with your daemon’s implementation
//! 3. transporting that packet to your daemon
//!
//! ### Formatting a Tracing Event
//!
//! Trait [TracingFormatter] defines the process of converting tracing [Span]s & [Event]s to
//! UTF-8-encoded strings & syslog priorities. Both RFCs [3164] & [5424] see the "message"” as
//! free-form text; their differences seem to come-down to the details of textual encoding.
//! Therefore, this trait concerns itself simply with translating from tracing entities to a
//! UTF-8-encoded message.
//!
//! [TracingFormatter]: crate::tracing::TracingFormatter
//!
//! [Span]: tracing::Span
//! [3164]: https://datatracker.ietf.org/doc/html/rfc3164
//! [5424]: https://datatracker.ietf.org/doc/html/rfc5424
//!
//! At the time of this writing, only a trival implementation is provided.
//!
//! ### From Textual Message to a syslog Packet
//!
//! [SyslogFormatter] is the trait that governs taking a Level, textual message & timestamp and
//! formatting it to an implementation-defined type (that must deref to `[u8]`) that represents the
//! syslog packet. This crate provides implementations that will produce messages compliant with
//! RFC [3164] & [5424].
//!
//! [SyslogFormatter]: crate::formatter::SyslogFormatter
//!
//! ### Sending the syslog Packet
//!
//! Finally, the completed syslog packet must be transmitted to the syslog daemon. The [Transport]
//! trait defines this behavior, and a number of implementations are provided:
//!
//! [Transport]: crate::transport::Transport
//!
//! - [TcpTransport](crate::transport::TcpTransport)
//! - [UdpTransport](crate::transport::UdpTransport)
//! - [UnixSocket](crate::transport::UnixSocket)
//! - [UnixSocketStream](crate::transport::UnixSocketStream)
//!
//! ## How This Process Plugs-In to the Tracing Framework
//!
//! This process connects to the tracing framework through the [Layer] type. [Layer] is
//! parameterized by implementations of each step in the process:
//!
//! [Layer]: crate::layer::Layer
//!
//! ```ignore
//! pub struct Layer<S, F1: SyslogFormatter, F2: TracingFormatter<S>, T: Transport<F1>> where
//!    S: Subscriber + for<'a> LookupSpan<'a>,
//! ```
//!
//! so that there are many concrete [Layer] implementations available, each corresponding to a
//! particular choice of methods for [Event] formatting, syslog formatting, & transport.
//!
//! [Layer] implements [tracing_subscriber::layer::Layer], so it can be "stacked" on top of other
//! layers in your tracing [Subscriber]. When added to a [Subscriber] in this way, it will be
//! informed of tracing events and given the opportunity to do somethign with them. At the time of
//! this writing, it only handles [Event]s, though I'd like to add support for other tracing
//! primitives, such as [Span]s. When it receives an [Event], it hands it to its "tracing formatter"
//! to get a textual message representing the [Event], along with a syslog-compliant Level. It hands
//! these off to its "syslog formatter" to get a complete syslog message, which it finally hands off
//! to its [Transport] implementation.
//!
//! [Subscriber]: tracing::Subscriber
//!
//! ## Tracing Metadata
//!
//! The [Rfc5424] syslog formatter can optionally be configured to transmit tracing metadata (e.g.
//! file name & line number) as Structured Data. The caller can of course configure the SDID
//! (Structured Data IDentifier) as desired, but the author obtained a Private Enterprise Number
//! (PEN), and the implementation will by default use an SDID of "tracing-meta@64700".
//!
//! [Rfc5424]: crate::rfc5424::Rfc5424
