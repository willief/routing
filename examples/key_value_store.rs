// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

//! usage example (using default methods of connecting to the network):
//!      starting the first node:       `key_value_store --first`
//!      starting a passive node:       `key_value_store --node`
//!      starting an interactive node:  `key_value_store`

// For explanation of lint checks, run `rustc -W help` or see
// https://github.com/maidsafe/QA/blob/master/Documentation/Rust%20Lint%20Checks.md
#![forbid(
    exceeding_bitshifts,
    mutable_transmutes,
    no_mangle_const_items,
    unknown_crate_types,
    warnings
)]
#![deny(
    bad_style,
    deprecated,
    improper_ctypes,
    missing_docs,
    non_shorthand_field_patterns,
    overflowing_literals,
    plugin_as_library,
    stable_features,
    unconditional_recursion,
    unknown_lints,
    unsafe_code,
    unused,
    unused_allocation,
    unused_attributes,
    unused_comparisons,
    unused_features,
    unused_parens,
    while_true
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results
)]
#![allow(
    box_pointers,
    missing_copy_implementations,
    missing_debug_implementations,
    variant_size_differences,
    non_camel_case_types
)]
#![cfg_attr(feature = "mock_base", allow(unused_extern_crates, unused_imports))]

#[macro_use]
extern crate log;
#[macro_use]
extern crate unwrap;
#[macro_use]
extern crate serde_derive;
#[cfg(not(feature = "mock_base"))]
extern crate safe_crypto;

mod utils;

#[cfg(feature = "mock_base")]
fn main() {
    println!("This example should be built without `--features=mock`.");
    // Return Linux sysexit code for "configuration error"
    ::std::process::exit(78);
}

#[cfg(not(feature = "mock_base"))]
mod unnamed {
    use crate::utils::{ExampleClient, ExampleNode};
    use docopt::Docopt;
    use maidsafe_utilities::log;
    use maidsafe_utilities::serialisation::{deserialise, serialise};
    use maidsafe_utilities::thread::{self, Joiner};
    use routing::{MutableData, Value, XorName};
    use safe_crypto;
    use std::io::{self, Write};
    use std::iter;
    use std::sync::mpsc;
    use std::sync::mpsc::{Receiver, Sender};
    use std::thread as std_thread;
    use std::time::Duration;

    // ==========================   Program Options   =================================
    #[rustfmt::skip]
    static USAGE: &str = "
Usage:
  key_value_store
  key_value_store --node
  key_value_store --first [--node]
  key_value_store --help

Options:
  -n, --node   Run as a non-interactive routing node in the network.
  -f, --first  Start a new network as the first node.
  -h, --help   Display this help message.

  Running without the --node option will start an interactive node.
  Such a node can be used to send requests such as 'put' and 'get' to the network.

  A passive node is one that simply reacts on received requests. Such nodes are
  the workers; they route messages and store and provide data.

  The network configuration file can be used to provide information on what
  network discovery patterns to use, or which seed nodes to use.
";

    const TAG: u64 = 10_000;
    const KEY: &[u8] = &[];

    #[derive(Debug, Deserialize)]
    struct Args {
        flag_first: bool,
        flag_node: bool,
        flag_help: bool,
    }

    #[derive(PartialEq, Eq, Debug, Clone)]
    enum UserCommand {
        Exit,
        Get(String),
        Put(String, String),
    }

    fn read_user_commands(command_sender: &Sender<UserCommand>) {
        loop {
            let mut command = String::new();
            let stdin = io::stdin();

            print!("Enter command (exit | put <key> <value> | get <key>)\n> ");
            let _ = io::stdout().flush();
            let _ = stdin.read_line(&mut command);

            let parts = command
                .trim_end_matches(|c| c == '\r' || c == '\n')
                .split(' ')
                .collect::<Vec<_>>();

            if parts.len() == 1 && parts[0] == "exit" {
                let _ = command_sender.send(UserCommand::Exit);
                return;
            } else if parts.len() == 2 && parts[0] == "get" {
                let _ = command_sender.send(UserCommand::Get(parts[1].to_string()));
            } else if parts.len() == 3 && parts[0] == "put" {
                let _ = command_sender
                    .send(UserCommand::Put(parts[1].to_string(), parts[2].to_string()));
            } else {
                println!("Unrecognised command");
            }
        }
    }

    struct KeyValueStore {
        example_client: ExampleClient,
        command_receiver: Receiver<UserCommand>,
        exit: bool,
        _joiner: Joiner,
    }

    impl KeyValueStore {
        fn new() -> KeyValueStore {
            let example_client = ExampleClient::new();
            let (command_sender, command_receiver) = mpsc::channel::<UserCommand>();
            KeyValueStore {
                example_client,
                command_receiver,
                exit: false,
                _joiner: thread::named("Command reader", move || {
                    read_user_commands(&command_sender)
                }),
            }
        }

        fn run(&mut self) {
            // Need to do poll as Select is not yet stable in the current rust implementation.
            loop {
                while let Ok(command) = self.command_receiver.try_recv() {
                    self.handle_user_command(command);
                }

                if self.exit {
                    break;
                }

                let interval = Duration::from_millis(10);
                std_thread::sleep(interval);
            }
        }

        fn handle_user_command(&mut self, cmd: UserCommand) {
            match cmd {
                UserCommand::Exit => {
                    self.exit = true;
                }
                UserCommand::Get(what) => {
                    self.get(&what);
                }
                UserCommand::Put(put_where, put_what) => {
                    self.put(put_where, put_what);
                }
            }
        }

        /// Get data from the network.
        pub fn get(&mut self, what: &str) {
            let name = Self::calculate_key_name(what);
            match self.example_client.get_mdata_value(name, TAG, KEY.to_vec()) {
                Ok(value) => {
                    let content = unwrap!(deserialise::<String>(&value.content));
                    println!("Got value {:?} on key {:?}", content, what);
                }
                Err(error) => println!("Failed to get {:?} ({:?})", what, error),
            }
        }

        /// Put data onto the network.
        pub fn put<S: AsRef<str>>(&mut self, put_where: S, put_what: S) {
            let name = Self::calculate_key_name(put_where.as_ref());

            let value = Value {
                content: unwrap!(serialise(&put_what.as_ref())),
                entry_version: 0,
            };
            let entries = iter::once((KEY.to_vec(), value)).collect();
            let owners = iter::once(*self.example_client.signing_public_key()).collect();

            let data = unwrap!(MutableData::new(
                name,
                TAG,
                Default::default(),
                entries,
                owners,
            ));
            if let Err(error) = self.example_client.put_mdata(data) {
                error!("Failed to put data ({:?})", error);
            }
        }

        fn calculate_key_name(key: &str) -> XorName {
            XorName(safe_crypto::hash(key.as_bytes()))
        }
    }

    impl Default for KeyValueStore {
        fn default() -> KeyValueStore {
            KeyValueStore::new()
        }
    }

    pub fn run_main() {
        unwrap!(log::init(false));

        let args: Args = Docopt::new(USAGE)
            .and_then(|docopt| docopt.deserialize())
            .unwrap_or_else(|error| error.exit());

        if args.flag_first {
            ExampleNode::new(true).run();
        } else if args.flag_node {
            ExampleNode::new(false).run();
        } else {
            KeyValueStore::new().run();
        }
    }
}

#[cfg(not(feature = "mock_base"))]
fn main() {
    unnamed::run_main()
}
