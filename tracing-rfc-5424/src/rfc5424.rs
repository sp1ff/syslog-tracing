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

//! RFC [5424]-compliant syslog message formatting
//!
//! [5424]: https://datatracker.ietf.org/doc/html/rfc5424
//!
//! [`Rfc5424`] is a [`SyslogFormatter`] that produces syslog messages according to RFC 5424. The
//! RFC asserts that it obsoletes RFC [3164], but in practice the older format is still in use. In
//! particular, [rsyslog] by default speaks it on the Unix domain socket (although it speaks 5424
//! over TCP/IP sockets).
//!
//! [3164]: https://datatracker.ietf.org/doc/html/rfc3164
//! [rsyslog]: https://www.rsyslog.com/

use crate::{
    byte_utils::bytes_from_os_str,
    facility::{Facility, Level},
    formatter::SyslogFormatter,
};

use backtrace::Backtrace;

use chrono::prelude::*;

type StdResult<T, E> = std::result::Result<T, E>;

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                       module error type                                        //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// module error type
#[non_exhaustive]
pub enum Error {
    BadAppName {
        name: Vec<u8>,
        back: Backtrace,
    },
    BadHostname {
        name: Vec<u8>,
        back: Backtrace,
    },
    BadIpAddress,
    BadProcId {
        name: Vec<u8>,
        back: Backtrace,
    },
    /// Failed to format the `tracing` Event
    BadTracingFormat {
        source: Box<dyn std::error::Error>,
        back: Backtrace,
    },
    /// Failed to fetch the current executable (via std::env)
    NoExecutable {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        back: Backtrace,
    },
    /// Failed to fetch hostname (via libc)
    NoHostname {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
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
            Error::BadAppName { name, .. } => {
                write!(f, "{:?} is not an RFC 5424-compliant app name", name)
            }
            Error::BadHostname { name, .. } => {
                write!(f, "{:?} is not an RFC 5424-compliant host name", name)
            }
            Error::BadIpAddress => write!(f, "Failed to obtain a local IP address"),
            Error::BadTracingFormat { source, .. } => {
                write!(f, "While formatting an Event or Span, got {}", source)
            }
            Error::NoExecutable { source, .. } => write!(
                f,
                "While extracting the name of the current process, got {}",
                source
            ),
            Error::NoHostname { source, .. } => write!(
                f,
                "While extracting the name of the current host, got {}",
                source
            ),
            Error::BadProcId { name, back } => {
                write!(f, "Bad proc id. name: {name:?}, backtrace: {back:?}",)
            }
        }
    }
}

impl std::fmt::Debug for Error {
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "RFC 5424 error: {}", self)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                         utility types                                          //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A [`Vec<u8>`] instance with the additional constraint that it must be less than 256 bytes
/// of ASCII.
pub struct Hostname(Vec<u8>);

impl Hostname {
    /// An RFC 5424-compliant hostname is at most 255 bytes of ASCII
    pub fn new(bytes: Vec<u8>) -> Result<Hostname> {
        if bytes.is_ascii() && bytes.len() < 256 {
            Ok(Hostname(bytes))
        } else {
            Err(Error::BadHostname {
                name: bytes,
                back: Backtrace::new(),
            })
        }
    }
}

impl std::default::Default for Hostname {
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
            // 👇 :=> StdResult<Hostname, Error>
            .and_then(|hn| Hostname::new(bytes_from_os_str(hn)))
            // 👇 will return the Ok(Hostname), or call the closure :=> StdResult<Hostname, Error>
            .or_else(|_err| {
                let ip: StdResult<std::net::IpAddr, Error> =
                    local_ip_address::local_ip().map_err(|_| Error::BadIpAddress);
                ip.and_then(|ip| {
                    let buf = ip.to_string().into_bytes();
                    if buf.len() < 256 {
                        Ok(Hostname(buf))
                    } else {
                        Err(Error::BadIpAddress)
                    }
                })
            }) // 👈 :=> StdResult<Hostname, Error>
            .or_else::<Error, _>(|_| Ok(Hostname(b"-".to_vec())))
            .unwrap()
    }
}

impl std::convert::TryFrom<String> for Hostname {
    type Error = Error;
    fn try_from(x: String) -> StdResult<Self, Self::Error> {
        Hostname::new(x.into_bytes())
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
            Err(Error::BadAppName {
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
                    None => vec![b'-'],
                })
            })
            .unwrap()
    }
}

#[cfg(test)]
mod test_names {

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
            Err(Error::BadProcId {
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

/// A syslog formatter that produces RFC [5424]-conformant syslog messages.
///
/// [5424]: https://datatracker.ietf.org/doc/html/rfc5424
pub struct Rfc5424 {
    facility: Facility,
    hostname: Hostname,
    appname: AppName,
    pid: ProcId,
    with_bom: bool,
}

impl std::default::Default for Rfc5424 {
    fn default() -> Self {
        Rfc5424 {
            facility: Facility::LOG_USER,
            hostname: Hostname::default(),
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
    pub fn hostname(mut self, hostname: Hostname) -> Self {
        self.imp.hostname = hostname;
        self
    }
    pub fn hostname_as_string(mut self, hostname: String) -> Result<Self> {
        self.imp.hostname = Hostname::try_from(hostname)?;
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

impl SyslogFormatter for Rfc5424 {
    type Error = Error;
    type Output = Vec<u8>;
    fn format(
        &self,
        level: Level,
        msg: &str,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<Self::Output> {
        let mut buf = format!(
            "<{}>1 {} ",
            self.facility as u8 | level as u8,
            timestamp
                .unwrap_or(Utc::now())
                .to_rfc3339_opts(SecondsFormat::Micros, false)
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
            buf.put_u8(0xef_u8);
            buf.put_u8(0xbb_u8);
            buf.put_u8(0xbf_u8);
        }

        buf.put_slice(msg.as_bytes());

        Ok(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_against_issue_014_regression() {
        let test_message = String::from_utf8(Rfc5424::builder()
            .facility(Facility::LOG_USER)
            .hostname_as_string("bree".to_owned())
            .unwrap(/* known good */)
            .appname_as_string("unit test suite".to_owned())
            .unwrap(/* known good */)
            .build()
            .format(Level::LOG_NOTICE, "This is a test message; its timestamp had better not have more than 6 digits in the fractional seconds place", None)
            .unwrap(/* known good */))
            .unwrap(/* known good */);
        eprintln!("Test message: {test_message}\n");
        let i = test_message.find('.').unwrap(/* known good */);
        let j = test_message.find('+').unwrap(/* known good */);
        assert!(j - i - 1 <= 6);
    }
}
