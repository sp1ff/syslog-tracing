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

//! The syslog transport layer
//! ==========================
//!
//! # Introduction
//!
//! This module defines the [`Transport`] trait that all implementations must support, as well
//! as the UDP, TCP & Unix socket (datagram as well as stream) implementations.
//!
//! # Examples
//!
//! To send syslog messages over UDP to a daemon listening on port 514 (the default) on localhost:
//!
//! ```rust
//! use tracing_rfc_5424::transport::UdpTransport;
//! let transpo = UdpTransport::local().unwrap();
//! ```
//!
//! On a non-standard port on another host:
//!
//! ```rust
//! use tracing_rfc_5424::transport::UdpTransport;
//! let transpo = UdpTransport::new("some-host.domain.io:5514");
//! assert!(transpo.is_err()); // no such host, after all
//! ```
//!
//! To send messages over UDP to a local Unix socket:
//!
//! ```rust
//! use tracing_rfc_5424::transport::UnixSocket;
//! let transpo = UnixSocket::new("/i/am/not/there.s");
//! assert!(transpo.is_err()); // no such socket, after all
//! ```
//!
//! # Discussion
//!
//! Why not just implement [`std::io::Write`] (and [`tokio::io::AsyncWrite`])? Because the
//! abstraction doesn't make sense for us; [`std::io::Write`] is an abstraction for a
//! general-purpose, byte-oriented sink. "The write method will attempt to write some data into the
//! object, returning how many bytes were successfully written." Our semantics are different: "take
//! this serialized message & transmit it to the syslog daemon".
//!
//! [`tokio::io::AsyncWrite`]: https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html
//!
//! [`tracing_subscriber::Layer`] is synchronous, but I could, I suppose, provide an async Transport
//! abstraction, enabling someone to write:
//!
//! [`tracing_subscriber::Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/index.html#reexport.Layer
//!
//! ```text
//! async fn f(&self, msg: &str) {
//!     self.syslog_formatter.on_msg(msg)?)
//!         .and_then(|thing| self.transport.send(thing).await?)
//! }
//! ```

use crate::formatter::SyslogFormatter;

use backtrace::Backtrace;

use std::{
    net::TcpStream,
    path::Path,
};

#[cfg(unix)]
use std::os::unix::net::{UnixDatagram, UnixStream};

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                       common error type                                        //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// syslog transport layer errors
#[non_exhaustive]
pub enum Error {
    /// I/O error
    Io {
        source: std::io::Error,
        back: Backtrace,
    },
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io {
            source: err,
            back: Backtrace::new(),
        }
    }
}

impl std::fmt::Display for Error {
    // `Error` is non-exhaustive so that adding variants won't be a breaking change to our
    // callers. That means the compiler won't catch us if we miss a variant here, so we
    // always include a `_` arm.
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io { source, .. } => write!(f, "I/O error: {}", source),
            _ => write!(f, "syslog transport layer error"),
        }
    }
}

impl std::fmt::Debug for Error {
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io { source: _, back } => write!(f, "{}\n{:#?}", self, back),
            _ => write!(f, "{}", self),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                        Transport trait                                         //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Operations all transport layers must support.
pub trait Transport<F: SyslogFormatter> {
    type Error: std::error::Error;
    /// Send a slice of byte on this transport mechanism.
    fn send(&self, buf: F::Output) -> std::result::Result<(), Self::Error>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                         UDP Transport                                          //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Sending syslog messages via UDP datagrams.
pub struct UdpTransport {
    socket: std::net::UdpSocket,
}

impl UdpTransport {
    /// Construct a [`Transport`] implementation via UDP at `addr`.
    pub fn new<A: std::net::ToSocketAddrs>(addr: A) -> Result<UdpTransport> {
        // Bind to any available port on localhost...
        let socket = std::net::UdpSocket::bind("127.0.0.1:0")?;
        // and connect to the syslog daemon at `addr`...
        socket.connect(addr)?;
        // and we're done!
        Ok(UdpTransport { socket })
    }
    /// Construct a [`Transport`] implementation via UDP at localhost:514
    pub fn local() -> Result<UdpTransport> {
        UdpTransport::new("localhost:514")
    }
}

impl<F> Transport<F> for UdpTransport
where
    F: SyslogFormatter,
{
    type Error = Error;
    fn send(&self, buf: F::Output) -> std::result::Result<(), Self::Error> {
        self.socket.send(&buf)?;
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                         TCP Transport                                          //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Sending syslog message via TCP streams
///
/// Note that this implementation, at present, uses non-transparent framing with a trailing
/// character of 10/0x0a/newline.
pub struct TcpTransport {
    socket: std::net::TcpStream,
}

impl TcpTransport {
    /// Construct a [`Transport`] implementation via TCP at `addr`.
    pub fn new<A: std::net::ToSocketAddrs>(addr: A) -> Result<TcpTransport> {
        Ok(TcpTransport {
            socket: TcpStream::connect(addr)?,
        })
    }
    /// Construct a [`Transport`] implementation via TCP at localhost:514
    pub fn try_default() -> Result<TcpTransport> {
        TcpTransport::new("localhost:514")
    }
}

impl<F> Transport<F> for TcpTransport
where
    F: SyslogFormatter,
{
    type Error = Error;
    fn send(&self, buf: F::Output) -> std::result::Result<(), Self::Error> {
        use std::io::Write;
        // Trick I learned from tracing-subscriber.
        // <https://docs.rs/tracing-subscriber/0.3.11/src/tracing_subscriber/fmt/fmt_layer.rs.html#867-903>
        // The problem is that `std::io::Write()` takes a `&mut self` and we just have a
        // `&self`. Therefore if I naively call:
        //
        //     self.socket.write_all(buf)
        //
        // the compiler will complain.
        //
        // The workaround depends upon the fact that `Write` is implemented both on `UnixStream` and
        // `&UnixStream`. So: I declare a mutable variable `writer` whose type is `&UnixStream`...
        let mut writer: &TcpStream = &self.socket;
        // and invoke `write_all()` on _that_ receiver, whose type is `&mut &UnixStream`--
        // i.e. "self" will be `&UnixStream` not `UnixStream`.
        //
        // Reddit discussion here:
        // <https://www.reddit.com/r/rust/comments/v2uxze/getting_a_mutable_reference_to_self_in_a_method/>
        writer.write(&buf)?;
        writer.write(&[10])?;
        writer.flush()?;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                    Unix Domain Sockets/UDP                                     //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Sending syslog messages via Unix socket (datagram)
#[cfg(unix)]
pub struct UnixSocket {
    socket: UnixDatagram,
}

#[cfg(unix)]
impl UnixSocket {
    /// Construct a [`Transport`] implementation via Unix datagram sockets at `path`.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<UnixSocket> {
        let sock = UnixDatagram::unbound()?;
        sock.connect(path)?;
        Ok(UnixSocket { socket: sock })
    }
    pub fn try_default() -> Result<UnixSocket> {
        UnixSocket::new("/dev/log")
    }
}

#[cfg(unix)]
impl<F> Transport<F> for UnixSocket
where
    F: SyslogFormatter,
{
    type Error = Error;
    fn send(&self, buf: F::Output) -> std::result::Result<(), Self::Error> {
        self.socket.send(&buf)?;
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                    Unix Domain Sockets/TCP                                     //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Sending syslog messages via Unix socket (stream)
///
/// Note that this implementation, at present, uses non-transparent framing with a trailing
/// character of 10/0x0a/newline.
#[cfg(unix)]
pub struct UnixSocketStream {
    socket: UnixStream,
}

#[cfg(unix)]
impl UnixSocketStream {
    /// Construct a [`Transport`] implementation via Unix sockets at `path`.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<UnixSocketStream> {
        Ok(UnixSocketStream {
            socket: UnixStream::connect(path)?,
        })
    }
    pub fn try_default() -> Result<UnixSocket> {
        UnixSocket::new("/dev/log")
    }
}

#[cfg(unix)]
impl<F> Transport<F> for UnixSocketStream
where
    F: SyslogFormatter,
{
    type Error = Error;
    fn send(&self, buf: F::Output) -> std::result::Result<(), Self::Error> {
        use std::io::Write;

        // Trick I learned from tracing-subscriber.
        // <https://docs.rs/tracing-subscriber/0.3.11/src/tracing_subscriber/fmt/fmt_layer.rs.html#867-903>
        // The problem is that `std::io::Write()` takes a `&mut self` and we just have a
        // `&self`. Therefore if I naively call:
        //
        //     self.socket.write_all(buf)
        //
        // the compiler will complain.
        //
        // The workaround depends upon the fact that `Write` is implemented both on `UnixStream` and
        // `&UnixStream`. So: I declare a mutable variable `writer` whose type is `&UnixStream`...
        let mut writer: &UnixStream = &self.socket;
        // and invoke `write_all()` on _that_ receiver, whose type is `&mut &UnixStream`--
        // i.e. "self" will be `&UnixStream` not `UnixStream`.
        //
        // Reddit discussion here:
        // <https://www.reddit.com/r/rust/comments/v2uxze/getting_a_mutable_reference_to_self_in_a_method/>
        writer.write(&buf)?;
        writer.write(&[10])?;
        writer.flush()?;

        Ok(())
    }
}
