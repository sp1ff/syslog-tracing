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

//! syslog facility & level defintions.
//!
//! [`Facility`] and [`Level`] replicate the names used in `<syslog.h>`. The are (mostly) identical
//! in both RFC [3164] & [5424], and so [`tracing-rfc-5424`](crate) models both with the same
//! enumeration.
//!
//! [3164]: https://datatracker.ietf.org/doc/html/rfc3164
//! [5424]: https://datatracker.ietf.org/doc/html/rfc5424

type StdResult<T, E> = std::result::Result<T, E>;

/// Both RFCs [5424] & [3164] define twenty-four "facilities" for messages. The enumeration values
/// duplicate the constants defined in `<syslog.h>`, albeit multiplied by 8 for convenience in
/// forming syslog message headers (which again mirrors the `#define`s in `<syslog.h>`).
///
/// [5424]: https://datatracker.ietf.org/doc/html/rfc5424
/// [3164]: https://datatracker.ietf.org/doc/html/rfc3164
///
/// The "facility" is clearly designed to indicate the source of the log message, but regrettably
/// selected a pre-defined set of values, along with eight "local" values and a "user" value
/// (documented in the `<syslog.h>` header file as, I kid you not, "random user-level
/// messages"). Furthermore, the set of selected sources is, ahem, showing its age. Does anyone have
/// a line-printer anymore? An FTP site? A Usenet server? If I re-purpose one of those on my host,
/// can I count on it being used for the same purpose on a different host? All part of the joy of
/// coding to a legacy system, ably documented in Giles Orr's [note] on the topic.
///
/// [note]: https://www.gilesorr.com/blog/rsyslog-facility-severity.html
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Facility {
    /// kernel messages
    LOG_KERN = 0 << 3,
    /// random user-level messages
    LOG_USER = 1 << 3,
    /// mail system
    LOG_MAIL = 2 << 3,
    /// system daemons
    LOG_DAEMON = 3 << 3,
    /// security/authorization messages
    LOG_AUTH = 4 << 3,
    /// messages generated internally by syslogd
    LOG_SYSLOG = 5 << 3,
    /// line printer subsystem
    LOG_LPR = 6 << 3,
    /// network news subsystem
    LOG_NEWS = 7 << 3,
    /// UUCP subsystem
    LOG_UUCP = 8 << 3,
    /// clock daemon
    LOG_CRON = 9 << 3,
    /// security/authorization messages (private)
    LOG_AUTHPRIV = 10 << 3,
    /// ftp daemon
    LOG_FTP = 11 << 3,
    /// NTP subsystem
    LOG_NTP = 12 << 3,
    /// Log Audit: Various operating systems have been found to utilize Facilities 4, 10, 13 and 14
    /// for security/authorization, audit, and alert messages which seem to be
    /// similar. [1](https://datatracker.ietf.org/doc/html/rfc3164#section-5.3), pg. 9
    LOG_AUDIT = 13 << 3,
    /// Log Alert: Various operating systems have been found to utilize Facilities 4, 10, 13 and 14
    /// for security/authorization, audit, and alert messages which seem to be
    /// similar. [1](https://datatracker.ietf.org/doc/html/rfc3164#section-5.3), pg. 9
    LOG_ALERT = 14 << 3,
    /// clock daemon: Various operating systems have been found to utilize both Facilities 9 and 15
    /// for clock (cron/at)
    /// messages. [1](https://datatracker.ietf.org/doc/html/rfc3164#section-5.3), pg. 9
    LOG_CLOCK = 15 << 3,
    /// reserved for local use
    LOG_LOCAL0 = 16 << 3,
    /// reserved for local use
    LOG_LOCAL1 = 17 << 3,
    /// reserved for local use
    LOG_LOCAL2 = 18 << 3,
    /// reserved for local use
    LOG_LOCAL3 = 19 << 3,
    /// reserved for local use
    LOG_LOCAL4 = 20 << 3,
    /// reserved for local use
    LOG_LOCAL5 = 21 << 3,
    /// reserved for local use
    LOG_LOCAL6 = 22 << 3,
    /// reserved for local use
    LOG_LOCAL7 = 23 << 3,
}

impl std::default::Default for Facility {
    /// The default facility is `LOG_USER`.
    fn default() -> Self {
        Facility::LOG_USER
    }
}

impl std::fmt::Display for Facility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> StdResult<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Facility::LOG_KERN => "LOG_KERN",
                Facility::LOG_USER => "LOG_USER",
                Facility::LOG_MAIL => "LOG_MAIL",
                Facility::LOG_DAEMON => "LOG_DAEMON",
                Facility::LOG_AUTH => "LOG_AUTH",
                Facility::LOG_SYSLOG => "LOG_SYSLOG",
                Facility::LOG_LPR => "LOG_LPR",
                Facility::LOG_NEWS => "LOG_NEWS",
                Facility::LOG_UUCP => "LOG_UUCP",
                Facility::LOG_CRON => "LOG_CRON",
                Facility::LOG_AUTHPRIV => "LOG_AUTHPRIV",
                Facility::LOG_FTP => "LOG_FTP",
                Facility::LOG_NTP => "LOG_NTP",
                Facility::LOG_AUDIT => "LOG_AUDIT",
                Facility::LOG_ALERT => "LOG_ALERT",
                Facility::LOG_CLOCK => "LOG_CLOCK",
                Facility::LOG_LOCAL0 => "LOG_LOCAL0",
                Facility::LOG_LOCAL1 => "LOG_LOCAL1",
                Facility::LOG_LOCAL2 => "LOG_LOCAL2",
                Facility::LOG_LOCAL3 => "LOG_LOCAL3",
                Facility::LOG_LOCAL4 => "LOG_LOCAL4",
                Facility::LOG_LOCAL5 => "LOG_LOCAL5",
                Facility::LOG_LOCAL6 => "LOG_LOCAL6",
                Facility::LOG_LOCAL7 => "LOG_LOCAL7",
            }
        )
    }
}

/// Both RFCs [5424] & [3164] define eight severity levels for messages. The enumeration values
/// duplicate the constants documented as per the `syslog()` manual [page] & defined in
/// `<syslog.h>`.
///
/// [5424]: https://datatracker.ietf.org/doc/html/rfc5424
/// [3164]: https://datatracker.ietf.org/doc/html/rfc3164
/// [page]: https://man7.org/linux/man-pages/man3/syslog.3.html
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Level {
    /// system is unusable
    LOG_EMERG,
    /// action must be take immediately
    LOG_ALERT,
    /// critical conditions
    LOG_CRIT,
    /// error conditions
    LOG_ERR,
    /// warning conditions
    LOG_WARNING,
    /// normal, but significant condition
    LOG_NOTICE,
    /// informational message
    LOG_INFO,
    /// debug-level message
    LOG_DEBUG,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> StdResult<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Level::LOG_EMERG => "LOG_EMERG",
                Level::LOG_ALERT => "LOG_ALERT",
                Level::LOG_CRIT => "LOG_CRIT",
                Level::LOG_ERR => "LOG_ERR",
                Level::LOG_WARNING => "LOG_WARNING",
                Level::LOG_NOTICE => "LOG_NOTICE",
                Level::LOG_INFO => "LOG_INFO",
                Level::LOG_DEBUG => "LOG_DEBUG",
            }
        )
    }
}

#[cfg(test)]
mod facility_level_tests {
    use super::*;
    /// Test basic PRI formatting
    #[test]
    fn test_pri() {
        assert_eq!(14, (Facility::LOG_USER as u8) | (Level::LOG_INFO as u8));
        assert_eq!(format!("{}", Facility::LOG_FTP), "LOG_FTP".to_string());
        assert_eq!(format!("{:?}", Facility::LOG_FTP), "LOG_FTP".to_string());
    }
}
