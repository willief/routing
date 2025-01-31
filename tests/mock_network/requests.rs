// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::{create_connected_clients, create_connected_nodes, gen_immutable_data, poll_all};
use routing::{
    mock::Network, Authority, ClientError, Event, EventStream, MessageId, Request, Response,
    QUORUM_DENOMINATOR, QUORUM_NUMERATOR,
};

#[test]
fn successful_put_request() {
    let min_section_size = 8;
    let quorum = 1 + (min_section_size * QUORUM_NUMERATOR) / QUORUM_DENOMINATOR;
    let network = Network::new(min_section_size, None);
    let mut rng = network.new_rng();
    let mut nodes = create_connected_nodes(&network, min_section_size + 1);
    let mut clients = create_connected_clients(&network, &mut nodes, 1);

    let dst = Authority::ClientManager(clients[0].name());
    let data = gen_immutable_data(&mut rng, 1024);
    let message_id = MessageId::new();

    assert!(clients[0]
        .inner
        .put_idata(dst, data.clone(), message_id)
        .is_ok());

    let _ = poll_all(&mut nodes, &mut clients);

    let mut request_received_count = 0;
    for node in nodes.iter_mut().filter(|n| n.is_recipient(&dst)) {
        loop {
            match node.try_next_ev() {
                Ok(Event::RequestReceived {
                    request:
                        Request::PutIData {
                            data: ref req_data,
                            msg_id: ref req_message_id,
                        },
                    ..
                }) => {
                    request_received_count += 1;
                    if data == *req_data && message_id == *req_message_id {
                        break;
                    }
                }
                Ok(_) => (),
                _ => panic!("Event::RequestReceived not received"),
            }
        }
    }

    assert!(request_received_count >= quorum);
}

#[test]
fn successful_get_request() {
    let min_section_size = 8;
    let quorum = 1 + (min_section_size * QUORUM_NUMERATOR) / QUORUM_DENOMINATOR;
    let network = Network::new(min_section_size, None);
    let mut rng = network.new_rng();
    let mut nodes = create_connected_nodes(&network, min_section_size + 1);
    let mut clients = create_connected_clients(&network, &mut nodes, 1);

    let data = gen_immutable_data(&mut rng, 1024);
    let dst = Authority::NaeManager(*data.name());
    let message_id = MessageId::new();

    assert!(clients[0]
        .inner
        .get_idata(dst, *data.name(), message_id)
        .is_ok());

    let _ = poll_all(&mut nodes, &mut clients);

    let mut request_received_count = 0;

    for node in nodes.iter_mut().filter(|n| n.is_recipient(&dst)) {
        loop {
            match node.try_next_ev() {
                Ok(Event::RequestReceived {
                    request:
                        Request::GetIData {
                            name: ref req_name,
                            msg_id: req_message_id,
                        },
                    src,
                    dst,
                }) => {
                    request_received_count += 1;
                    if data.name() == req_name && message_id == req_message_id {
                        if let Err(err) = node.inner.send_get_idata_response(
                            dst,
                            src,
                            Ok(data.clone()),
                            req_message_id,
                        ) {
                            trace!("Failed to send GetIData success response: {:?}", err);
                        }
                        break;
                    }
                }
                Ok(_) => (),
                _ => panic!("Event::RequestReceived not received"),
            }
        }
    }

    assert!(request_received_count >= quorum);

    let _ = poll_all(&mut nodes, &mut clients);

    let mut response_received_count = 0;

    for client in &mut clients {
        loop {
            match client.inner.try_next_ev() {
                Ok(Event::ResponseReceived {
                    response:
                        Response::GetIData {
                            res: Ok(ref res_data),
                            msg_id: ref res_message_id,
                        },
                    ..
                }) => {
                    response_received_count += 1;
                    if data == *res_data && message_id == *res_message_id {
                        break;
                    }
                }
                Ok(_) => (),
                _ => panic!("Event::ResponseReceived not received"),
            }
        }
    }

    assert_eq!(response_received_count, 1);
}

#[test]
fn failed_get_request() {
    let min_section_size = 8;
    let quorum = 1 + (min_section_size * QUORUM_NUMERATOR) / QUORUM_DENOMINATOR;
    let network = Network::new(min_section_size, None);
    let mut rng = network.new_rng();
    let mut nodes = create_connected_nodes(&network, min_section_size + 1);
    let mut clients = create_connected_clients(&network, &mut nodes, 1);

    let data = gen_immutable_data(&mut rng, 1024);
    let dst = Authority::NaeManager(*data.name());
    let message_id = MessageId::new();

    assert!(clients[0]
        .inner
        .get_idata(dst, *data.name(), message_id)
        .is_ok());

    let _ = poll_all(&mut nodes, &mut clients);

    let mut request_received_count = 0;

    for node in nodes.iter_mut().filter(|n| n.is_recipient(&dst)) {
        loop {
            match node.try_next_ev() {
                Ok(Event::RequestReceived {
                    request:
                        Request::GetIData {
                            name: ref req_name,
                            msg_id: ref req_message_id,
                        },
                    src,
                    dst,
                }) => {
                    request_received_count += 1;
                    if data.name() == req_name && message_id == *req_message_id {
                        if let Err(err) = node.inner.send_get_idata_response(
                            dst,
                            src,
                            Err(ClientError::NoSuchData),
                            *req_message_id,
                        ) {
                            trace!("Failed to send GetIData failure response: {:?}", err);
                        }
                        break;
                    }
                }
                Ok(_) => (),
                _ => panic!("Event::RequestReceived not received"),
            }
        }
    }

    assert!(request_received_count >= quorum);

    let _ = poll_all(&mut nodes, &mut clients);

    let mut response_received_count = 0;

    for client in &mut clients {
        loop {
            match client.inner.try_next_ev() {
                Ok(Event::ResponseReceived {
                    response:
                        Response::GetIData {
                            res: Err(_),
                            msg_id: ref res_message_id,
                        },
                    ..
                }) => {
                    response_received_count += 1;
                    if message_id == *res_message_id {
                        break;
                    }
                }
                Ok(_) => (),
                _ => panic!("Event::ResponseReceived not received"),
            }
        }
    }

    assert_eq!(response_received_count, 1);
}
