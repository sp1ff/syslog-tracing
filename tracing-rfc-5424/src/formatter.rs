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
//! This module defines the [`Formatter`] trait.

use crate::{facility::Level, tracing::TracingFormatter};

use chrono::prelude::*;

/// Operations all formatters must support.
pub trait Formatter {
    type Error: std::error::Error;
    fn format_event(
        &self,
        level: Level,
        event: &tracing::Event,
        fmtr: &impl TracingFormatter,
        timestamp: Option<DateTime<Utc>>,
    ) -> std::result::Result<Vec<u8>, Self::Error>;
}
