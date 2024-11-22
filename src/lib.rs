use crate::task::CSTaskGroupIndex;
use broadsword::dll;
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    collections::VecDeque,
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Receiver},
        OnceLock,
        Mutex,
    },
    thread::spawn,
};
use std::time::Duration;
use tracing::instrument::WithSubscriber;
use tungstenite::{accept, client, Message};
use widestring::U16CStr;
use lazy_static::lazy_static;

const WS_PORT: &str = "10001";

/// Bindings to the bloodmessage system
mod bloodmessage;
mod difficulty;
/// Bindings to the player
mod player;
/// Service locator using FS's DLRF system
mod reflection;
mod task;
mod util;

// Mod starts here
#[dll::entrypoint]
pub fn entry(_hmodule: usize) -> bool {
    broadsword::logging::init("bloodmessage-mod.log");

    bloodmessage::init_hooks();


    spawn(|| {
        let server =
            TcpListener::bind(format!("127.0.0.1:{}", WS_PORT)).expect("Could not bind to port");

        for stream in server.incoming() {
            let client =
                spawn(|| handle_client(stream.expect("Could not acquire incoming stream")));

            client.join().expect("Client failed to join");
        }
    });

    log::info!("Spawned websocket server");

    true
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum IncomingMessage {
    SpawnBloodMessage { text: String },
    IncreaseDifficulty,
    DecreaseDifficulty,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum OutgoingMessage {
    BloodMessageEvent { text: String },
}

lazy_static! {
    static ref RECVIN: Mutex<Option<Receiver<IncomingMessage>>> = Mutex::new(None);
}

fn handle_client_task() {
    if let Some(recv_in) = RECVIN.lock().unwrap().as_ref() {
        while let Ok(msg) = recv_in.try_recv() {
            match msg {
                IncomingMessage::SpawnBloodMessage { text } => bloodmessage::spawn_message(&text),
                IncomingMessage::IncreaseDifficulty => difficulty::increase_difficulty(),
                IncomingMessage::DecreaseDifficulty => difficulty::decrease_difficulty(),
            }
        }
    }
}

pub fn handle_client(stream: TcpStream) {
    log::info!("Serving new client...");

    // Setup Outgoing message hook
    let (send_out, recv_out) = channel();
    if let Ok(mut guard) = bloodmessage::SEND.write() {
        guard.replace(send_out);
    }

    // Setup IncomingMessage handler. This is a task that runs in the games task system.
    let (send_in, recv_in) = channel();
    *RECVIN.lock().unwrap() = Some(recv_in);
    let task = task::run_task(
        handle_client_task, //this can't be a closure that takes local args, otherise it breaks
        CSTaskGroupIndex::WorldChrMan_PostPhysics,
    );

    //loop listening for data from the client, and pass it to the IncomingMessage handler
    let mut websocket = accept(stream).expect("Could not accept stream");
    loop {
        while let Ok(msg) = recv_out.try_recv() {
            websocket
                .send(Message::Text(
                    serde_json::to_string(&OutgoingMessage::BloodMessageEvent { text: msg })
                        .unwrap(),
                ))
                .unwrap()
        }

        match websocket.read() {
            Ok(msg) => {
                log::info!("Received websocket message. {msg:?}");

                if let Message::Text(content) = msg {
                    log::info!("Received text: {content}");

                    let deserialized: IncomingMessage =
                        serde_json::from_str(&content).expect("Could not parse incoming message");

                    log::info!("Deserialized incoming message {deserialized:?}");
                    send_in.send(deserialized).expect("Could not send");
                }
            }
            Err(e) => match e {
                tungstenite::Error::AlreadyClosed => {
                    log::info!("Client dropped connection");
                    break;
                }
                _ => log::error!("Error while handling message: {e:?}"),
            },
        }
    }

    if let Ok(mut guard) = bloodmessage::SEND.write() {
        guard.take();
    }
    *RECVIN.lock().unwrap() = None;
    drop(task);
}
