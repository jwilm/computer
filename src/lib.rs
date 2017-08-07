#![deny(unused_must_use)]

//!
//! An extensible platform for writing chatbots.
//!
//! Chatbot is extensible in both adapters (services it connects to like IRC or Slack) and handlers
//! (the bits of code processing messages). Several adapters are provided out of the box including a
//! [Command-line Adapter][], an [IRC Adapter][], and a [Slack Adapter][]. There's also several
//! handlers built in which can be viewed in the [implementors][Messagehandler implementors] section
//! of the handler module.
//!
//! `cargo run` will get you a working command line bot immediately after cloning. The `main.rs`
//! file is the default binary and is a good starting place to hack on your own bot.
//!
//! [Command-line Adapter]: adapter/struct.CliAdapter.html
//! [IRC Adapter]: adapter/struct.CliAdapter.html
//! [Slack Adapter]: adapter/struct.SlackAdapter.html
//! [MessageHandler implementors]: handler/trait.MessageHandler.html#implementors
//!
//! ## Adapters
//!
//! An adapter is a wrapper around some service like Slack, IRC, or just the command line. When the
//! bot starts up, it passes a Sender<IncomingMessage> into the handler `process_events` method.
//! The main loop owns the receiver and thus gets messages from all of the adapters. When the
//! adapter gets a message from the underlying service, it must create an IncomingMessage and `send`
//! it using that Sender. The IncomingMessage must be populated with a `Sender<OutgoingMessage>`
//! which the adapter calls `recv` on. It is up to the adapter what to do with these
//! OutgoingMessages. Generally they should be directed to whence they came.
//!
//! ## Handlers
//!
//! Handlers provide a `Regex` which the main loop uses to check whether the handler is interested.
//! If the regex matches, `handle` is called on the handler with the IncomingMessage. The handler
//! can then do some work and call `reply` on the incoming message to send its response. The adapter
//! which created the incoming message will decide how to route the message back to the service.
//!
//! Handlers are not sandboxed and can thus bring the bot down in flames if they decide to panic. It
//! may be worth sandboxing in threads in the future (maybe make handlers `Runnable` and send to a
//! worker pool). The built in handlers are written with care as to not panic the bot.
//!
//! For very simple handlers, there is a `handler!` macro which lets you pass a regex and a closure
//! without having to implement the MessageHandler trait. The example below contains an example of
//! this.
//!
//! ## Chatbot
//!
//! The Chatbot is the central data structure of the chatbot platform. It contains a `run` method
//! which listens for messages from adapters and routes them to handlers. Any program which uses
//! chatbot will need to minimally create a Chatbot, add an adapter, add a handler, and call Chatbot
//! [`run`](chatbot/struct.Chatbot.html#method.run).
//!
//! For example, setting up a simple echo server that responds to CLI input:
//!
//! # Examples
//!
//! ```no_run
//! # #[macro_use(handler)]
//! # extern crate chatbot;
//! # fn main() {
//! use chatbot::Chatbot;
//! use chatbot::adapter::CliAdapter;
//!
//! let mut bot = Chatbot::new("echobot");
//!
//! let echo = handler!("EchoHandler", r"echo .+", |_, msg| {
//!     Some(msg.to_owned())
//! });
//!
//! bot.add_handler(echo);
//! bot.add_adapter(CliAdapter::new("echobot"));
//!
//! bot.run();
//! # }
//! ```
//!
//! Sometimes you might want the bot to react only when it is addressed,
//! this is what
//! [`add_addressed_handler`](chatbot/struct.Chatbot.html#method.add_addressed_handler) is for.
//!
//! An example would be a bot that responses to pings, only you don't want the bot to respond
//! everytime there is any form of "ping" in a sentence.
//!
//! ```no_run
//! # #[macro_use(handler)]
//! # extern crate chatbot;
//! # fn main() {
//! use chatbot::Chatbot;
//! use chatbot::adapter::CliAdapter;
//!
//! let mut bot = Chatbot::new("pingbot");
//!
//! let ping = handler!("PingHandler", r"ping", |_, _| Some("pong".to_owned()));
//!
//! bot.add_addressed_handler(ping);
//! bot.add_adapter(CliAdapter::new("pingbot"));
//!
//! bot.run();
//! # }
//! ```
//!

extern crate regex;
extern crate rustc_serialize;
#[cfg(feature = "slack-adapter")]
extern crate slack;
#[cfg(feature = "slack-adapter")]
extern crate slack_api;
#[cfg(feature = "irc-adapter")]
extern crate irc;

/// Shorthand for creating a `Regex` as suggested by the regex crate. You probably don't need to
/// `macro_use` this unless you're creating handlers in an external module.
#[macro_export]
macro_rules! regex(
    ($s:expr) => (regex::Regex::new($s).unwrap());
);

/// The `handler!` macro is shorthand for creating simple chat handlers. It
/// accepts a name, a string used to build a regex for testing the incoming
/// message and for collecting captures, and a closure which should return a
/// `String` to be sent as the outgoing message.
///
/// # Examples
///
/// ```
/// # #[macro_use(handler)]
/// # extern crate chatbot;
/// # fn main() {
/// let ping = handler!("Ping", r"ping", |_, _| Some("pong".to_owned()) );
/// # }
/// ```
///
#[macro_export]
macro_rules! handler {
    ( $name:expr, $rstr:expr, $lambda:expr ) => {
        $crate::handler::BasicResponseHandler::new($name, $rstr, $lambda)
    }
}


pub mod adapter;
pub mod handler;
pub mod message;

mod chatbot;
pub use chatbot::Chatbot;

pub use handler::HandlerResult;
pub use handler::MessageHandler;
pub use message::IncomingMessage;
