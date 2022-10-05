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

//! RFC [5424]-compliant syslog message formatting
//!
//! [5424]: https://datatracker.ietf.org/doc/html/rfc5424
//!
//! [`Rfc5424`] is a [`Formatter`] that produces syslog messages according to RFC 5424.

use crate::{
    byte_utils::bytes_from_os_str,
    error::{Error, Result},
    facility::{Facility, Level},
    formatter::Formatter,
    tracing::TracingFormatter,
};

use backtrace::Backtrace;

use chrono::prelude::*;

type StdResult<T, E> = std::result::Result<T, E>;

/// A [`Vec<u8>`] instance with the additional constraint that it must be less than 256 bytes
/// of ASCII.
pub struct Rfc5424Hostname(Vec<u8>);

impl Rfc5424Hostname {
    /// An RFC 5424-compliant hostname is at most 255 bytes of ASCII
    pub fn new(bytes: Vec<u8>) -> Result<Rfc5424Hostname> {
        if bytes.is_ascii() && bytes.len() < 256 {
            Ok(Rfc5424Hostname(bytes))
        } else {
            Err(Error::BadRfc5424Hostname {
                name: bytes,
                back: Backtrace::new(),
            })
        }
    }
}

impl std::default::Default for Rfc5424Hostname {
    /// Attempt to figure-out an RFC [5424]-compliant hostname.
    ///
    /// The order of preference for the contents of the HOSTNAME field is as follows:
    ///
    /// 1.  FQDN
    /// 2.  Static IP address
    /// 3.  hostname
    /// 4.  Dynamic IP address
    /// 5.  the NILVALUE
    ///
    /// This implementation doesn't quite do that; for reasons of expedience, it will first simply try
    /// [gethostname()], then uses [netlink] to try & find an IP address. I'd like to come back & tighten
    /// this up.
    ///
    /// [5424]: https://datatracker.ietf.org/doc/html/rfc5424
    /// [gethostname()]: https://man7.org/linux/man-pages/man2/gethostname.2.html
    /// [netlink]: https://man7.org/linux/man-pages/man7/netlink.7.html
    fn default() -> Self {
        // `hostname::get()` returns an `Result<OsString,_>`, which is really kind of a hassle to work
        // with...
        hostname::get()
            .map_err(|err| Error::NoHostname {
                source: Box::new(err),
                back: Backtrace::new(),
            })
            // vvv :=> StdResult<Rfc5424Hostname, Error>
            .and_then(|hn| Rfc5424Hostname::new(bytes_from_os_str(hn)))
            // vvv will return the Ok(Rfc5424Hostname), or call the closure :=>
            // StdResult<Rfc5424Hostname, Error>
            .or_else(|_err| {
                let ip: StdResult<std::net::IpAddr, Error> =
                    local_ip_address::local_ip().map_err(|_| Error::BadRfc5424IpAddress);
                ip.and_then(|ip| {
                    let buf = ip.to_string().into_bytes();
                    if buf.len() < 256 {
                        Ok(Rfc5424Hostname(buf))
                    } else {
                        Err(Error::BadRfc5424IpAddress)
                    }
                })
            }) // :=> StdResult<Rfc5424Hostname, Error>
            .or_else::<Error, _>(|_| Ok(Rfc5424Hostname(b"-".to_vec())))
            .unwrap()
    }
}

impl std::convert::TryFrom<String> for Rfc5424Hostname {
    type Error = Error;
    fn try_from(x: String) -> StdResult<Self, Self::Error> {
        Rfc5424Hostname::new(x.into_bytes())
    }
}

/// A string with the additional constraint contstraing that it is less than forty-nine bytes of
/// ASCII.
pub struct AppName(Vec<u8>);

impl std::fmt::Display for AppName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> StdResult<(), std::fmt::Error> {
        write!(f, "{}", std::str::from_utf8(&self.0).unwrap())
    }
}

impl AppName {
    pub fn new(bytes: Vec<u8>) -> Result<AppName> {
        if bytes.is_ascii() && bytes.len() < 49 {
            Ok(AppName(bytes))
        } else {
            Err(Error::BadRfc5424AppName {
                name: bytes,
                back: Backtrace::new(),
            })
        }
    }
}

impl std::convert::TryFrom<String> for AppName {
    type Error = Error;
    fn try_from(x: String) -> StdResult<Self, Self::Error> {
        AppName::new(x.into_bytes())
    }
}

impl std::default::Default for AppName {
    /// Attempt to figure-out an RFC [5424] Application Name.
    ///
    /// The APP-NAME field SHOULD identify the device or application that originated the message.  It is
    /// a string without further semantics. It is intended for filtering messages on a relay or
    /// collector.
    ///
    /// This implementation relies on [`std::env::current_exe`]. It cannot fail; if for any reason that
    /// value cannot be retrieved, or is not ASCII, it simply returns "-".
    ///
    /// [5424]: https://datatracker.ietf.org/doc/html/rfc5424
    fn default() -> Self {
        std::env::current_exe() // :=> StdResult<PathBuf, std::io::Error>
            .map_err(|err| Error::NoExecutable {
                source: Box::new(err),
                back: Backtrace::new(),
            })
            .and_then(|pbuf| {
                AppName::new(match pbuf.file_name() {
                    // Arrrghhhh... wicked copy!
                    Some(os_str) => bytes_from_os_str(os_str.to_os_string()),
                    None => vec!['-' as u8],
                })
            })
            .unwrap()
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn app_name() {
        let _x = AppName::default(); // At least _exercise_ `Default`

        let x: &[u8] = b"0123456789012345678901234567890123456789012345678";
        let v: Vec<u8> = x.into();
        assert!(AppName::new(v).is_err());

        let x: &[u8] = b"udp-test";
        let v: Vec<u8> = x.into();
        assert!(AppName::new(v).is_ok());
    }
}

/// A string with the additional constraint contstraing that it is less than 129 bytes of ASCII.
pub struct ProcId(Vec<u8>);

impl std::fmt::Display for ProcId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> StdResult<(), std::fmt::Error> {
        write!(f, "{}", std::str::from_utf8(&self.0).unwrap())
    }
}

impl ProcId {
    pub fn new(bytes: Vec<u8>) -> Result<ProcId> {
        if bytes.is_ascii() && bytes.len() < 129 {
            Ok(ProcId(bytes))
        } else {
            Err(Error::BadRfc5424ProcId {
                name: bytes,
                back: Backtrace::new(),
            })
        }
    }
}

impl std::convert::TryFrom<String> for ProcId {
    type Error = Error;
    fn try_from(x: String) -> StdResult<Self, Self::Error> {
        ProcId::new(x.into_bytes())
    }
}

impl std::default::Default for ProcId {
    /// Attempt to figure-out an RFC [5424] Process ID.
    ///
    /// While generally this field has been the OS process identifier, "PROCID is a value that is
    /// included in the message, having no interoperable meaning, except that a change in the value
    /// indicates there has been a discontinuity in syslog reporting."
    ///
    /// This implementation relies on [`std::process::id`]. It cannot fail.
    ///
    /// [5424]: https://datatracker.ietf.org/doc/html/rfc5424
    fn default() -> Self {
        ProcId::new(format!("{}", std::process::id()).into_bytes()).unwrap()
    }
}

/// A formatter that produces RFC [5424]-conformant syslog messages.
///
/// [5424]: https://datatracker.ietf.org/doc/html/rfc5424
pub struct Rfc5424 {
    facility: Facility,
    hostname: Rfc5424Hostname,
    appname: AppName,
    pid: ProcId,
    with_bom: bool,
}

impl std::default::Default for Rfc5424 {
    fn default() -> Self {
        Rfc5424 {
            facility: Facility::LOG_USER,
            hostname: Rfc5424Hostname::default(),
            appname: AppName::default(),
            pid: ProcId::default(),
            with_bom: false,
        }
    }
}

pub struct Rfc5424Builder {
    imp: Rfc5424,
}

impl Rfc5424Builder {
    pub fn facility(mut self, facility: Facility) -> Self {
        self.imp.facility = facility;
        self
    }
    pub fn hostname(mut self, hostname: Rfc5424Hostname) -> Self {
        self.imp.hostname = hostname;
        self
    }
    pub fn hostname_as_string(mut self, hostname: String) -> Result<Self> {
        self.imp.hostname = Rfc5424Hostname::try_from(hostname)?;
        Ok(self)
    }
    pub fn appname_as_string(mut self, appname: String) -> Result<Self> {
        self.imp.appname = AppName::try_from(appname)?;
        Ok(self)
    }
    pub fn pid_as_string(mut self, pid: String) -> Result<Self> {
        self.imp.pid = ProcId::try_from(pid)?;
        Ok(self)
    }
    pub fn with_bom(mut self, with_bom: bool) -> Self {
        self.imp.with_bom = with_bom;
        self
    }
    pub fn build(self) -> Rfc5424 {
        self.imp
    }
}

impl Rfc5424 {
    pub fn builder() -> Rfc5424Builder {
        Rfc5424Builder {
            imp: Rfc5424::default(),
        }
    }
}

impl Formatter for Rfc5424 {
    fn format_event(
        &self,
        level: Level,
        event: &tracing::Event,
        fmtr: &impl TracingFormatter,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<Vec<u8>> {
        let mut buf = format!(
            "<{}>1 {} ",
            self.facility as u8 | level as u8,
            timestamp.unwrap_or(Utc::now()).to_rfc3339()
        )
        .into_bytes();

        use bytes::buf::BufMut;
        buf.put_slice(&self.hostname.0);

        buf.put_slice(format!(" {} {} - - ", self.appname, self.pid).as_bytes());

        // From the RFC

        // "The character set used in MSG SHOULD be UNICODE, encoded using UTF-8 as specified in
        // [RFC3629].  If the syslog application cannot encode the MSG in Unicode, it MAY use
        // any other encoding."

        // "If a syslog application encodes MSG in UTF-8, the string MUST start with the Unicode
        // byte order mask (BOM), which for UTF-8 is ABNF %xEF.BB.BF.  The syslog application
        // MUST encode in the "shortest form" and MAY use any valid UTF-8 sequence."
        if self.with_bom {
            buf.put_u8(0xef as u8);
            buf.put_u8(0xbb as u8);
            buf.put_u8(0xbf as u8);
        }

        fmtr.format_event(event, &mut buf)?;
        Ok(buf)
    }
}
