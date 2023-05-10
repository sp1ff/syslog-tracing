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

//! RFC 3164-compliant syslog message formatting
//! ============================================
//!
//! # Introduction
//!
//! [`Rfc3164`] is a [`SyslogFormatter`] that produces syslog messages according to RFC [3164] (AKA
//! the BSD syslog protocol). The protocol is descriptive rather than prescriptive in that it
//! attempted to describe what was already present in the wild, rather than describe something new.
//!
//! [3164]: https://datatracker.ietf.org/doc/html/rfc3164
//!
//! Although older than RFC [5424] it is still useful because [rsyslog],
//! when configured to listen on a Unix Domain socket (i.e. `/dev/log`) will [use] the so-called
//! "special parser" to handle incoming messages, which does not support RFC 5424 (see e.g. [here]).
//!
//! [5424]: https://datatracker.ietf.org/doc/html/rfc5424
//! [rsyslog]: https://www.rsyslog.com/
//! [use]: https://unix.stackexchange.com/questions/622801/does-linuxs-rsyslog-support-rfc-5424
//! [here]: https://github.com/rsyslog/rsyslog/issues/4749
//!
//!

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

/// syslog transport layer errors
#[non_exhaustive]
pub enum Error {
    /// Non-compliant hostname provided
    BadHostname { name: Vec<u8>, back: Backtrace },
    /// Failed to retrieve an IP address in lieu of a hostname
    BadIpAddress {
        source: local_ip_address::Error,
        back: Backtrace,
    },
    /// Non-compliant tag provided
    BadTag { name: Vec<u8>, back: Backtrace },
    /// Failed to format the `tracing` Event
    BadTracingFormat {
        source: Box<dyn std::error::Error>,
        back: Backtrace,
    },
    /// I/O error
    Io {
        source: std::io::Error,
        back: Backtrace,
    },
    /// Unable to deduce a compliant tag
    NoTag {
        pathb: std::path::PathBuf,
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
            Error::BadHostname { name, .. } => {
                write!(f, "{:?} is not an RFC3164-compliant hostname", name)
            }
            Error::BadIpAddress { source, .. } => write!(
                f,
                "While attempting to retrieve an IP address for this host, got {}",
                source
            ),
            Error::BadTag { name, .. } => write!(f, "{:?} is not an RFC3164-compliant tag", name),
            Error::BadTracingFormat { source, .. } => write!(
                f,
                "While attempting to format an Event or Span, got {}",
                source
            ),
            Error::Io { source, .. } => write!(f, "I/O error: {}", source),
            Error::NoTag { pathb, .. } => {
                write!(f, "{:#?} does not yield an RFC3164-compliant tag", pathb)
            }
            _ => write!(f, "syslog transport layer error"),
        }
    }
}

impl std::fmt::Debug for Error {
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BadHostname { name: _, back } => write!(f, "{}\n{:#?}", self, back),
            Error::BadIpAddress { source: _, back } => write!(f, "{}\n{:#?}", self, back),
            Error::BadTag { name: _, back } => write!(f, "{}\n{:#?}", self, back),
            Error::BadTracingFormat { source: _, back } => write!(f, "{}\n{:#?}", self, back),
            Error::Io { source: _, back } => write!(f, "{}\n{:#?}", self, back),
            Error::NoTag { pathb: _, back } => write!(f, "{}\n{:#?}", self, back),
            _ => write!(f, "{}", self),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

////////////////////////////////////////////////////////////////////////////////////////////////////
//                                         utility types                                          //
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A `Vec<u8>` instance with the additional constraint that its contents be ASCII above the value
/// 32 (space)
pub struct Rfc3164Hostname(Vec<u8>);

impl Rfc3164Hostname {
    /// An RFC 3164-compliant hostname is made-up of ASCII above 32/space. The RFC states "The
    /// Domain Name MUST NOT be included in the HOSTNAME field" which I interpret to mean that _if_
    /// one is using a true [hostname], `bytes` should contain only letters, digits and `-`. That
    /// said, one _may_ use an IP v4 address for this field, so this method doesn't attempt to
    /// enforce this condition & instead relies on the caller to do so (this _is_ respected by
    /// [`Rfc3164Hostname::try_default`]).
    ///
    /// [hostname]: https://man7.org/linux/man-pages/man7/hostname.7.html
    pub fn new(bytes: Vec<u8>) -> Result<Rfc3164Hostname> {
        if bytes.iter().all(|&x| x > 32 && x < 128) {
            Ok(Rfc3164Hostname(bytes))
        } else {
            Err(Error::BadHostname {
                name: bytes,
                back: Backtrace::new(),
            })
        }
    }
    /// Remove the domain (if any) from a host name
    ///
    /// This method will remove anything including & after the first `.` in `bytes`.
    fn strip_domain(mut bytes: Vec<u8>) -> Vec<u8> {
        if let Some(idx) = bytes.iter().position(|&x| x == b'.') {
            bytes.truncate(idx);
        }
        bytes
    }
    /// Attempt to figure-out an RFC [3164]-compliant hostname.
    ///
    /// Per the RFC:
    ///
    /// The HOSTNAME field will contain only the hostname, the IPv4 address, or the IPv6 address of
    /// the originator of the message.  The preferred value is the hostname.  If the hostname is
    /// used, the HOSTNAME field MUST contain the hostname of the device as specified in STD 13.
    /// It should be noted that this MUST NOT contain any embedded spaces.  The Domain Name MUST NOT
    /// be included in the HOSTNAME field.  If the IPv4 address is used, it MUST be shown as the
    /// dotted decimal notation as used in STD 13.  If an IPv6 address is used, any valid
    /// representation used in RFC 2373 MAY be used.
    ///
    /// [3164]: https://datatracker.ietf.org/doc/html/rfc3164
    pub fn try_default() -> Result<Rfc3164Hostname> {
        // `hostname::get()` returns an `Result<OsString,_>`, which is really kind of a hassle to work
        // with...
        hostname::get()
            .map_err(|err| err.into())
            // 👇 :=> StdResult<Rfc3164Hostname, Error>
            .and_then(|hn| {
                Rfc3164Hostname::new(Rfc3164Hostname::strip_domain(bytes_from_os_str(hn)))
            })
            // 👇 will return the Ok(Rfc3164Hostname), or call the closure :=> StdResult<Rfc3164Hostname, Error>
            .or_else(|_err| {
                let ip: StdResult<std::net::IpAddr, Error> =
                    local_ip_address::local_ip().map_err(|err| Error::BadIpAddress {
                        source: err,
                        back: Backtrace::new(),
                    });
                ip.map(|ip| Rfc3164Hostname(ip.to_string().into_bytes()))
            })
    }
}

impl std::convert::TryFrom<String> for Rfc3164Hostname {
    type Error = Error;
    fn try_from(x: String) -> StdResult<Self, Self::Error> {
        Rfc3164Hostname::new(x.into_bytes())
    }
}

/// A `Vec<u8>` instance with the additional constraint that it be ASCII alphanumeric characters
///
/// Per the RFC: "The value in the TAG field will be the name of the program or process that
/// generated the message. The TAG is a string of ABNF alphanumeric characters that MUST NOT exceed
/// 32 characters.  Any non-alphanumeric character will terminate the TAG field and will be assumed
/// to be the starting character of the CONTENT field."
///
/// An RFC 3164 message field is described, oddly, as having two parts: the "tag" & the
/// "content". The tag is clearly meant to be a process name, but really only needs to signify the
/// source of the message on the source host. The odd part is, the Process ID is considered to be a
/// part of the content, not the tag: "The process name is commonly displayed in the TAG field.
/// Quite often, additional information is included at the beginning of the CONTENT field.  The
/// format of "TAG\[pid\]:" - without the quote marks - is common.  The left square bracket is used
/// to terminate the TAG field in this case and is then the first character in the CONTENT field.
///
/// Therefore [`Tag`] simply represents an ASCII alphanumeric string that is less than or equal to
/// 32 characters in length.
pub struct Tag(Vec<u8>);

impl Tag {
    pub fn new(bytes: Vec<u8>) -> Result<Tag> {
        if bytes.len() <= 32
            && bytes.iter().all(|&x| {
                (b'0'..=b'9').contains(&x)
                    || (b'A'..=b'Z').contains(&x)
                    || (b'a'..=b'z').contains(&x)
            })
        {
            Ok(Tag(bytes))
        } else {
            Err(Error::BadTag {
                name: bytes,
                back: Backtrace::new(),
            })
        }
    }
    /// Strip non-compliant ASCII characters
    fn strip_non_compliant(x: Vec<u8>) -> Vec<u8> {
        x.into_iter()
            .filter(|&x| {
                (b'0'..=b'9').contains(&x)
                    || (b'A'..=b'Z').contains(&x)
                    || (b'a'..=b'z').contains(&x)
            })
            .collect()
    }
    pub fn try_default() -> Result<Tag> {
        std::env::current_exe() // :=> StdResult<PathBuf, std::io::Error>
            .map_err(|err| err.into())
            .and_then(|pbuf| match pbuf.file_name() {
                Some(os_str) => Tag::new(Tag::strip_non_compliant(bytes_from_os_str(
                    os_str.to_os_string(),
                ))),
                None => Err(Error::NoTag {
                    pathb: pbuf.clone(),
                    back: Backtrace::new(),
                }),
            })
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> StdResult<(), std::fmt::Error> {
        write!(f, "{}", std::str::from_utf8(&self.0).unwrap())
    }
}

impl std::convert::TryFrom<String> for Tag {
    type Error = Error;
    fn try_from(x: String) -> StdResult<Self, Self::Error> {
        Tag::new(x.into_bytes())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_hostname() {
        let _x = Rfc3164Hostname::try_default(); // At least _exercise_ `Default`

        // <https://users.rust-lang.org/t/difference-of-u8-n-and-u8/30695>
        assert!(Rfc3164Hostname::new("not valid".as_bytes().into()).is_err());

        assert!(
            Rfc3164Hostname::strip_domain("staging.pwpinfra.com".as_bytes().into()) == b"staging"
        );

        let x = Rfc3164Hostname::try_from(String::from("bree"));
        assert!(x.is_ok());
    }

    #[test]
    fn test_tag() {
        let _x = Tag::try_default(); // At least exercise it

        let x = Tag::new(b"tracingrfc".to_vec());
        assert!(x.is_ok());

        let x = Tag::new(b"012345678901234567890123456789012".to_vec()); // 33 chars-- no go
        assert!(x.is_err());

        let x = Tag::new("🩡".as_bytes().to_vec()); // Non-ASCII-- no go
        assert!(x.is_err());
    }
}

/// A syslog formatter that produces RFC [3164]-conformant syslog messages.
///
/// [3164]: https://datatracker.ietf.org/doc/html/rfc3164
///
/// # Character encoding
///
/// Per the spec: "The code set traditionally and most often used has also been seven-bit ASCII in
/// an eight-bit field", but in practice UTF-8 seems to be accepted. Therefore, callers may ask
/// instances to [escape] unicode, but by default they will not.
///
/// [escape]: str::escape_unicode
pub struct Rfc3164 {
    facility: Facility,
    hostname: Rfc3164Hostname,
    tag: Tag,
    add_pid: Option<u32>,
    escape_unicode: bool,
}

impl Rfc3164 {
    pub fn try_default() -> Result<Rfc3164> {
        Ok(Rfc3164 {
            facility: Facility::LOG_USER,
            hostname: Rfc3164Hostname::try_default()?,
            tag: Tag::try_default()?,
            add_pid: Some(std::process::id()),
            escape_unicode: false,
        })
    }
    pub fn builder() -> Result<Rfc3164Builder> {
        Ok(Rfc3164Builder {
            imp: Rfc3164::try_default()?,
        })
    }
}

pub struct Rfc3164Builder {
    imp: Rfc3164,
}

impl Rfc3164Builder {
    pub fn facility(mut self, facility: Facility) -> Self {
        self.imp.facility = facility;
        self
    }
    pub fn hostname(mut self, hostname: Rfc3164Hostname) -> Self {
        self.imp.hostname = hostname;
        self
    }
    pub fn hostname_as_string(mut self, hostname: String) -> Result<Self> {
        self.imp.hostname = Rfc3164Hostname::try_from(hostname)?;
        Ok(self)
    }
    pub fn tag_as_string(mut self, tag: String) -> Result<Self> {
        self.imp.tag = Tag::try_from(tag)?;
        Ok(self)
    }
    pub fn escape_unicode(mut self, escape_unicode: bool) -> Self {
        self.imp.escape_unicode = escape_unicode;
        self
    }
    pub fn build(self) -> Rfc3164 {
        self.imp
    }
}

impl SyslogFormatter for Rfc3164 {
    type Error = Error;
    type Output = Vec<u8>;
    fn format(
        &self,
        level: Level,
        msg: &str,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<Self::Output> {
        let mut buf = format!(
            "<{}>{} ",
            self.facility as u8 | level as u8,
            timestamp.map(|d| d.with_timezone(&Local))
                .or_else(|| Some(Local::now()))
                .unwrap()
                .format("%b %_d %H:%M:%S"),
        )
        .into_bytes();

        use bytes::BufMut;
        buf.put_slice(&self.hostname.0);

        // The MSG part has two fields known as the TAG field and the CONTENT field.  The value in
        // the TAG field will be the name of the program or process that generated the message.  The
        // CONTENT contains the details of the message.  This has traditionally been a freeform
        // message that gives some detailed information of the event.  The TAG is a string of ABNF
        // alphanumeric characters that MUST NOT exceed 32 characters.  Any non-alphanumeric
        // character will terminate the TAG field and will be assumed to be the starting character
        // of the CONTENT field.  Most commonly, the first character of the CONTENT field that
        // signifies the conclusion of the TAG field has been seen to be the left square bracket
        // character ("["), a colon character (":"), or a space character.
        buf.put_slice(b" ");
        buf.put_slice(&self.tag.0);
        if let Some(pid) = self.add_pid {
            buf.put_slice(format!("[{}]: ", pid).as_bytes());
        }

        if self.escape_unicode {
            buf.put_slice(msg.escape_unicode().to_string().as_bytes())
        } else {
            buf.put_slice(msg.as_bytes())
        }

        Ok(buf)
    }
}
