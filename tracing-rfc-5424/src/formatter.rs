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

//! syslog formatting primitives.
//!
//! This module defines the [`SyslogFormatter`] trait.

use crate::facility::Level;

use chrono::prelude::*;

use std::ops::Deref;

/// Operations all formatters must support
/// ======================================
///
/// # Introduction
///
/// The translation from [`tracing`] events to syslog messages occurs in three parts:
///
/// [`tracing`]: https://docs.rs/tracing/latest/tracing/index.html
///
/// 1. formatting the event to a textual message
///
/// 2. incorporating that message into a syslog packet compliant with your daemon's implementation
///
/// 3. transporting that packet to your daemon
///
/// [`SyslogFormatter`] implements step 2 in this process: given the [`Level`], a textual message
/// field, and an optional timestamp, produce a compliant syslog packet.
///
/// # Design
///
/// The associated type `Output` is designed to make illegal states unrepresentable. If the
/// [`Transport`] trait simply took, say, a slice of `u8` then callers could mistakenly pass
/// _anything_ to it (a little endian binary representation of a `u32`, `[0; 1204]` or any silly
/// thing). I would like to enforce the rule that "The thing passed to the [`Transport`] trait must
/// have been returned from a [`SyslogFormatter`] implementation." Hence the associated type, and
/// the constraint that it be dereferenceable to a slice of `u8` (to enable the [`Transport`]
/// implementation to deal with it). This _does_ mean making the [`SyslogFormatter`] implementation
/// type a generic parameter to the [`Transport`] type.
///
/// [`Transport`]: crate::transport::Transport
pub trait SyslogFormatter {
    type Error: std::error::Error;
    type Output: Deref<Target = [u8]>;
    fn format(
        &self,
        level: Level,
        msg: &str,
        timestamp: Option<DateTime<Utc>>,
    ) -> std::result::Result<Self::Output, Self::Error>;
}
