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

//! The syslog transport layer.
//!
//! This module defines the [`Transport`] trait that all implementations must support, as well
//! as the UDP implementation. Other implementations are in the works.
//!
//! # Examples
//!
//! To send syslog messages over UDP to a daemon listening on port 514 (the default) on localhost:
//!
//! ```rust
//! use syslog_tracing::transport::UdpTransport;
//! let transpo = UdpTransport::local().unwrap();
//! ```
//!
//! On a non-standard port on another host:
//!
//! ```rust
//! use syslog_tracing::transport::UdpTransport;
//! let transpo = UdpTransport::new("some-host.domain.io:5514");
//! assert!(transpo.is_err()); // no such host, after all
//! ```
//!

use crate::error::{Error, Result};

use backtrace::Backtrace;

use std::{os::unix::net::UnixDatagram, path::Path};

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                      transport mechanisms                                      //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Operations all transport layers must support.
pub trait Transport {
    /// Send a slice of byte on this transport mechanism.
    ///
    /// It would be nice to make this more general, to accept input in a variety of forms that might
    /// support zero-copy, but that the end of the day, UDP, TCP & Unix sockets all operate on a
    /// contiguous slice of `u8`, so we require that our caller assemble one.
    fn send(&self, buf: &[u8]) -> Result<usize>;
}

/// Sending syslog messages via UDP datagrams.
pub struct UdpTransport {
    socket: std::net::UdpSocket,
}

impl UdpTransport {
    /// Construct a [`Transport`] implementation via UDP at `addr`.
    pub fn new<A: std::net::ToSocketAddrs>(addr: A) -> Result<UdpTransport> {
        // Bind to any available port on localhost...
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").map_err(|err| Error::Transport {
            source: Box::new(err),
            back: Backtrace::new(),
        })?;
        // and connect to the syslog daemon at `addr`:
        socket.connect(addr).map_err(|err| Error::Transport {
            source: Box::new(err),
            back: Backtrace::new(),
        })?;
        Ok(UdpTransport { socket })
    }
    /// Construct a [`Transport`] implementation via UDP at localhost:514
    pub fn local() -> Result<UdpTransport> {
        UdpTransport::new("localhost:514")
    }
}

impl Transport for UdpTransport {
    fn send(&self, buf: &[u8]) -> Result<usize> {
        self.socket.send(buf).map_err(|err| Error::Transport {
            source: Box::new(err),
            back: Backtrace::new(),
        })
    }
}

/// Sending syslog messages via Unix socket.
#[cfg(target_os = "linux")]
pub struct UnixSocket {
    socket: UnixDatagram,
}

#[cfg(target_os = "linux")]
impl UnixSocket {
    /// Construct a [`Transport`] implementation via Unix sockets at `path`.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<UnixSocket> {
        let sock = UnixDatagram::unbound().map_err(|err| Error::Transport {
            source: Box::new(err),
            back: Backtrace::new(),
        })?;
        sock.connect(path).map_err(|err| Error::Transport {
            source: Box::new(err),
            back: Backtrace::new(),
        })?;
        Ok(UnixSocket { socket: sock })
    }
    pub fn try_default() -> Result<UnixSocket> {
        UnixSocket::new("/dev/log")
    }
}

#[cfg(target_os = "linux")]
impl Transport for UnixSocket {
    fn send(&self, buf: &[u8]) -> Result<usize> {
        self.socket.send(buf).map_err(|err| Error::Transport {
            source: Box::new(err),
            back: Backtrace::new(),
        })?;
        Ok(buf.len())
    }
}
