// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use connection;
use connection::Endpoint;
use connection::LogFileNames;
use endpoint_ip::EndpointIP;
use typedispatcher::HandlerResult;
use typedispatcher::TypeDispatcher;
use types::HandlerParams;

pub struct ConnectionIP {
    dispatcher: TypeDispatcher,
    remote_log_names: LogFileNames,
    local_log_names: LogFileNames,
    endpoints: Vec<Option<EndpointIP>>,
}

impl ConnectionIP {
    /// Common initialization
    fn init(&mut self) -> HandlerResult<()> {
        let handle_udp_message = |params: HandlerParams| -> HandlerResult<()> { Ok(()) };
        /*
        self.dispatcher
            .set_system_handler(constants::UDP_DESCRIPTION, handle_udp_message)
            */
        Ok(())
    }

    /// Create a new ConnectionIP that is a server.
    pub fn new_server(local_log_names: Option<LogFileNames>) -> HandlerResult<ConnectionIP> {
        let disp = TypeDispatcher::create()?;
        let mut ret = ConnectionIP {
            dispatcher: disp,
            remote_log_names: connection::make_none_log_names(),
            local_log_names: connection::make_log_names(local_log_names),
            endpoints: Vec::new(),
        };
        ret.init()?;
        Ok(ret)
    }

    /// Create a new ConnectionIP that is a client.
    pub fn new_client(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
    ) -> HandlerResult<ConnectionIP> {
        let disp = TypeDispatcher::create()?;
        let mut ret = ConnectionIP {
            dispatcher: disp,
            remote_log_names: connection::make_log_names(remote_log_names),
            local_log_names: connection::make_log_names(local_log_names),
            endpoints: Vec::new(),
        };
        // Create our single endpoint
        ret.endpoints.push(Some(EndpointIP::new()));
        ret.init()?;
        Ok(ret)
    }
}

impl connection::Connection for ConnectionIP {
    fn dispatcher_mut(&mut self) -> &mut TypeDispatcher {
        &mut self.dispatcher
    }

    fn dispatcher(&self) -> &TypeDispatcher {
        &self.dispatcher
    }

    fn call_on_each_mut_endpoint<'a, F: 'a + FnMut(&mut dyn Endpoint)>(&'a mut self, mut f: F) {
        for ref mut e in self.endpoints.iter_mut() {
            match e {
                Some(ref mut endpoint) => (f)(endpoint),
                _ => {}
            }
        }
    }
    fn call_on_each_endpoint<'a, F: 'a + Fn(&dyn Endpoint)>(&self,  f: F) {
        for ref e in self.endpoints.iter() {
            match e {
                Some(ref endpoint) => (f)(endpoint),
                _ => {}
            }
        }
    }
}
