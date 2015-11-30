extern crate slack;

mod message;
use self::message::*;

use std::env;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;
use std::thread;

use rustc_serialize::json::ToJson;

use slack::Message;

use chatbot::Chatbot;
use adapter::ChatAdapter;
use message::AdapterMsg;
use message::IncomingMessage;

/// SlackAdapter sends and receives messages from the Slack chat service. Until actualy
/// configuration is added, the slack token should be placed in the environment variable
/// `SLACK_BOT_TOKEN`
pub struct SlackAdapter {
    token: String
}

impl SlackAdapter {
    pub fn new() -> SlackAdapter {
        SlackAdapter {
            token: match env::var("SLACK_BOT_TOKEN") {
                Ok(t) => t,
                Err(_) => panic!("Failed to get SLACK_BOT_TOKEN from env")
            }
        }
    }
}

struct MyHandler {
  count: i64,
  tx_incoming: Sender<IncomingMessage>,
  tx_outgoing: Sender<AdapterMsg>
}

#[allow(unused_variables)]
impl slack::MessageHandler for MyHandler {
    fn on_receive(&mut self, cli: &mut slack::RtmClient, raw: &str) {
        debug!("Received[{}]: {}", self.count, raw.to_string());
        self.count = self.count + 1;

        match string_to_slack_msg(raw) {
            Ok(slack_msg) => {
                match slack_msg {
                    Event::Message(Msg::Plain(msg)) => {
                        let incoming = IncomingMessage::new("SlackAdapter".to_owned(), None,
                            Some(msg.channel().to_owned()), Some(msg.user().to_owned()),
                            msg.text().to_owned(), self.tx_outgoing.clone());

                        self.tx_incoming.send(incoming)
                                        .ok().expect("Bot unable to process messages");
                    },
                    _ => ()
                }
            },
            Err(e) => {
                println!("error decoding slack message: {:?}", e);
                println!("please consider reporting this to jwilm/chatbot as it is probably a bug");
            }
        }
    }

    fn on_ping(&mut self, cli: &mut slack::RtmClient) { }

    fn on_close(&mut self, cli: &mut slack::RtmClient) { }

    fn on_connect(&mut self, cli: &mut slack::RtmClient) { }
}

impl ChatAdapter for SlackAdapter {
    /// SlackAdapter name
    fn get_name(&self) -> &str {
        "SlackAdapter"
    }

    fn process_events(&self, _: &Chatbot, tx_incoming: Sender<IncomingMessage>) {
        println!("SlackAdapter: process_events");
        let (tx_outgoing, rx_outgoing) = channel();

        let uid = AtomicIsize::new(0);

        let mut cli = slack::RtmClient::new();
        let (client, slack_rx) = cli.login(self.token.as_ref()).unwrap();
        let slack_tx = cli.get_outs().unwrap();

        thread::Builder::new().name("Chatbot Slack Receiver".to_owned()).spawn(move || {
            abort_on_panic!("Chatbot Slack Receiver aborting", {
                let mut handler = MyHandler {
                    count: 0,
                    tx_incoming: tx_incoming,
                    tx_outgoing: tx_outgoing
                };
                cli.run::<MyHandler>(&mut handler, client, slack_rx).unwrap();
            });
        }).ok().expect("failed to create thread for slack receiver");

        thread::Builder::new().name("Chatbot Slack Sender".to_owned()).spawn(move || {
            abort_on_panic!("Chatbot Slack Sender", {
                loop {
                    match rx_outgoing.recv() {
                        Ok(msg) => {
                            match msg {
                                AdapterMsg::Outgoing(m) => {
                                    let id = uid.fetch_add(1, Ordering::SeqCst) as i64;
                                    let out = OutgoingEvent::new(id, m);
                                    slack_tx.send(Message::Text(out.to_json().to_string())).unwrap();
                                }
                                // Not implemented for now
                                AdapterMsg::Private(_) => {
                                    println!("SlackAdaptor: Private messages not implemented");
                                }
                                AdapterMsg::Shutdown => {
                                    break
                                }
                            }
                        },
                        Err(e) => {
                            println!("error receiving outgoing messages: {}", e);
                            break
                        }
                    }
                }
            });
        }).ok().expect("failed to create thread for slack sender");
    }
}
