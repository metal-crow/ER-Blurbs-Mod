use crate::task::CSTaskGroupIndex;
use broadsword::dll;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Receiver},
        Mutex,
    },
    thread::spawn,
};
use tungstenite::{accept, Message};

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
    static ref TASK_ENQUEUE: Mutex<Option<Receiver<IncomingMessage>>> = Mutex::new(None);
}

fn handle_client_task() {
    if let Some(recv_in) = TASK_ENQUEUE.lock().unwrap().as_ref() {
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
    // We only support 1 client at a time. TODO check this by looking at TASK_ENQUEUE == None

    // Setup a channel for communicating with the in-game task
    let (task_send, task_recv) = channel();
    *TASK_ENQUEUE.lock().unwrap() = Some(task_recv);

    // Setup a channel for notification if a player reads a message
    let (msginfo_send, msginfo_recv) = channel();
    if let Ok(mut guard) = bloodmessage::SEND.write() {
        guard.replace(msginfo_send);
    }

    // Start the task. This serves this client until connection is closed
    let task = task::run_task(
        handle_client_task, //this can't be a closure that takes local args, otherise it breaks
        CSTaskGroupIndex::WorldChrMan_PostPhysics,
    );

    let mut websocket = accept(stream).expect("Could not accept stream");
    loop {
        //listen for data from the game for messages being read, and pass it back to the remote client
        while let Ok(msg) = msginfo_recv.try_recv() {
            websocket
                .send(Message::Text(
                    serde_json::to_string(&OutgoingMessage::BloodMessageEvent { text: msg })
                        .unwrap(),
                ))
                .unwrap()
        }

        //listen for data from the remote client, and pass it to the IncomingMessage handler
        match websocket.read() {
            Ok(msg) => {
                log::info!("Received websocket message. {msg:?}");

                if let Message::Text(content) = msg {
                    log::info!("Received text: {content}");

                    let deserialized: IncomingMessage =
                        serde_json::from_str(&content).expect("Could not parse incoming message");

                    log::info!("Deserialized incoming message {deserialized:?}");
                    task_send.send(deserialized).expect("Could not send");
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
    *TASK_ENQUEUE.lock().unwrap() = None;
    drop(task);
}
