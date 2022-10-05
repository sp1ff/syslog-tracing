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

//! Test writing to port 514 on the local host.

use tracing::{debug, error, info, trace, warn};
use tracing_rfc_5424::{layer::Layer, transport::TcpTransport};
use tracing_subscriber::{
    layer::SubscriberExt, // Needed to get `with()`
    registry::Registry,
};

pub fn main() {
    // Setup the real subsriber...
    let subscriber =
        Registry::default().with(Layer::with_transport(TcpTransport::try_default().unwrap()));
    // and install it.
    let _guard = tracing::subscriber::set_default(subscriber);

    trace!("你好, TCP socket.");
    debug!("你好, TCP socket.");
    info!("你好, TCP socket.");
    warn!("你好, TCP socket.");
    error!("你好, TCP socket.");
}
