mod message;
use self::message::*;

use std::env;
use std::sync::mpsc::{Sender, channel};
use std::thread;

use rustc_serialize::json::ToJson;

use slack;
use slack_api;
use slack_api::chat::{post_message, PostMessageRequest};
use regex::Regex;

use adapter::ChatAdapter;
use message::AdapterMsg;
use message::IncomingMessage;

/// SlackAdapter sends and receives messages from the Slack chat service. Until actualy
/// configuration is added, the slack token should be placed in the environment variable
/// `SLACK_BOT_TOKEN`
pub struct SlackAdapter {
    api_token: String,
    client: Option<slack::RtmClient>,
    addresser_regex: Regex
}

impl SlackAdapter {
    pub fn new(bot_name: &str) -> SlackAdapter {
        let token = env::var("SLACK_BOT_TOKEN").expect("Failed to get SLACK_BOT_TOKEN from env");
        let cli = slack::RtmClient::login(&token[..]).expect("login to slack");

        SlackAdapter {
            api_token: token,
            client: Some(cli),
            addresser_regex: Regex::new(format!(r"^<@{}>", bot_name).as_str()).unwrap()
        }
    }
}

struct MyHandler {
  count: i64,
  tx_incoming: Sender<IncomingMessage>,
  tx_outgoing: Sender<AdapterMsg>,
}

#[allow(unused_variables)]
impl slack::EventHandler for MyHandler {
    fn on_event(&mut self,
                cli: &slack::RtmClient,
                event: slack::Event)
    {
        println!("Received[{}]: {:?}", self.count, event);
        self.count = self.count + 1;

        if let slack::Event::Message(msg) = event {
            if let slack::Message::Standard(msg) = *msg {
                let incoming = IncomingMessage::new("SlackAdapter".to_owned(), None,
                    msg.channel, msg.user,
                    msg.text.unwrap(), self.tx_outgoing.clone());

                self.tx_incoming.send(incoming)
                                .ok().expect("Bot unable to process messages");
            }
        }
    }

    fn on_close(&mut self, cli: &slack::RtmClient) { }

    fn on_connect(&mut self, cli: &slack::RtmClient) { }
}

impl ChatAdapter for SlackAdapter {
    /// SlackAdapter name
    fn get_name(&self) -> &str {
        "SlackAdapter"
    }

    /// Check whether this adapter was addressed
    fn addresser(&self) -> &Regex {
        &self.addresser_regex
    }

    fn process_events(&mut self, tx_incoming: Sender<IncomingMessage>) {
        println!("SlackAdapter: process_events");
        let (tx_outgoing, rx_outgoing) = channel();

        let cli = self.client.take().unwrap();

        thread::Builder::new().name("Chatbot Slack Receiver".to_owned()).spawn(move || {
            let mut handler = MyHandler {
                count: 0,
                tx_incoming: tx_incoming,
                tx_outgoing: tx_outgoing,
            };
            cli.run(&mut handler).expect("run connector ok");
        }).ok().expect("failed to create thread for slack receiver");

        let api_token = self.api_token.clone();
        thread::Builder::new().name("Chatbot Slack Sender".to_owned()).spawn(move || {
            let webclient = slack_api::default_client().expect("initialize default slack http client");
            loop {
                match rx_outgoing.recv() {
                    Ok(msg) => {
                        match msg {
                            AdapterMsg::Outgoing(m) => {
                                let mut req = PostMessageRequest::default();
                                req.channel = m.get_incoming().channel().unwrap();
                                req.text = m.get_incoming().get_contents();
                                post_message(&webclient, &api_token, &req)
                                    .expect("slack_api chat post message");
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
        }).ok().expect("failed to create thread for slack sender");
    }
}
