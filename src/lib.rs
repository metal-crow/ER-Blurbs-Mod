use crate::task::CSTaskGroupIndex;
use broadsword::dll;
use lazy_static::lazy_static;
use minidump_writer::MinidumpType;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Receiver},
        Mutex,
    },
    thread::spawn,
};
use tungstenite::{accept, Message};
use windows::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, SetUnhandledExceptionFilter, EXCEPTION_POINTERS,
};

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

mod spiritash;

fn crash_handler(exception_info: *const EXCEPTION_POINTERS) {
    unsafe {
        let context = *(*exception_info).ContextRecord;
        log::info!(
            "Unhandled exception caught! {0:?}",
            (*(*exception_info).ExceptionRecord)
        );
        log::info!("Rax: 0x{:X}", context.Rax);
        log::info!("Rcx: 0x{:X}", context.Rcx);
        log::info!("Rdx: 0x{:X}", context.Rdx);
        log::info!("Rbx: 0x{:X}", context.Rbx);
        log::info!("Rsp: 0x{:X}", context.Rsp);
        log::info!("Rbp: 0x{:X}", context.Rbp);
        log::info!("Rsi: 0x{:X}", context.Rsi);
        log::info!("Rdi: 0x{:X}", context.Rdi);
        log::info!("R8:  0x{:X}", context.R8);
        log::info!("R9:  0x{:X}", context.R9);
        log::info!("R10: 0x{:X}", context.R10);
        log::info!("R11: 0x{:X}", context.R11);
        log::info!("R12: 0x{:X}", context.R12);
        log::info!("R13: 0x{:X}", context.R13);
        log::info!("R14: 0x{:X}", context.R14);
        log::info!("R15: 0x{:X}", context.R15);
        log::info!("Rip: 0x{:X}", context.Rip);
        log::info!("LastBranchToRip: 0x{:X}", context.LastBranchToRip);
        log::info!("LastBranchFromRip: 0x{:X}", context.LastBranchFromRip);
        log::info!("LastExceptionToRip: 0x{:X}", context.LastExceptionToRip);
        log::info!("LastExceptionFromRip: 0x{:X}", context.LastExceptionFromRip);
    }

    let mut minidump_file = std::fs::File::create("crash.dmp").expect("failed to create file");

    // Attempts to the write the minidump
    minidump_writer::minidump_writer::MinidumpWriter::dump_local_context(
        // The exception code, presumably one of STATUS_*. Defaults to STATUS_NONCONTINUABLE_EXCEPTION if not specified
        None,
        // If not specified, uses the current thread as the "crashing" thread,
        // so this is equivalent to passing `None`, but it could be any thread
        // in the process
        Some(unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }),
        Some(
            MinidumpType::Normal
                | MinidumpType::WithIndirectlyReferencedMemory
                | MinidumpType::WithProcessThreadData
                | MinidumpType::WithThreadInfo
                | MinidumpType::WithCodeSegs,
        ),
        &mut minidump_file,
    )
    .expect("failed to write minidump");
}

unsafe extern "system" fn my_exception_filter1(exception_info: *const EXCEPTION_POINTERS) -> i32 {
    if ((*(*exception_info).ExceptionRecord).ExceptionCode).0 as u32 & 0xFF000000 != 0xC0000000 {
        return 0;
    }

    crash_handler(exception_info);

    1
}

unsafe extern "system" fn my_exception_filter2(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    if ((*(*exception_info).ExceptionRecord).ExceptionCode).0 as u32 & 0xFF000000 != 0xC0000000 {
        return 0;
    }

    crash_handler(exception_info);

    1 // EXCEPTION_EXECUTE_HANDLER
}

// Mod starts here
#[dll::entrypoint]
pub fn entry(_hmodule: usize) -> bool {
    let _ = fs::remove_file("bloodmessage-mod.log");
    broadsword::logging::init("bloodmessage-mod.log");
    unsafe {
        // Set the unhandled exception filter
        SetUnhandledExceptionFilter(Some(my_exception_filter1));
        AddVectoredExceptionHandler(1, Some(my_exception_filter2));
    }

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
    SpawnBloodMessage { text: String, msg_visual: i32 },
    RemoveBloodMessage { text: String },
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
                IncomingMessage::SpawnBloodMessage { text, msg_visual } => {
                    bloodmessage::spawn_message(&text, msg_visual)
                }
                IncomingMessage::RemoveBloodMessage { text } => bloodmessage::delete_message(&text),
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
    *bloodmessage::MSGINFO_SEND.lock().unwrap() = Some(msginfo_send);

    // Start the task. This serves this client until connection is closed
    let task_msgs = task::run_task(
        handle_client_task, //this can't be a closure that takes local args, otherise it breaks
        CSTaskGroupIndex::WorldChrMan_PostPhysics,
    );

    // Start the task to handle scaling the enemies
    let task_scaling = task::run_task(
        difficulty::set_scaling,
        CSTaskGroupIndex::WorldChrMan_PostPhysics,
    );

    stream
        .set_nonblocking(true)
        .expect("set_nonblocking call failed");
    let mut peek_buf = [0; 1];
    let mut websocket = accept(stream.try_clone().expect("tcpstream clone failed..."))
        .expect("Could not accept stream");

    loop {
        //listen for data from the game for messages being read, and pass it back to the remote client
        if let Ok(msg) = msginfo_recv.try_recv() {
            log::info!("Sending player read message {msg:?}");

            websocket
                .send(Message::Text(
                    serde_json::to_string(&OutgoingMessage::BloodMessageEvent { text: msg })
                        .unwrap(),
                ))
                .unwrap()
        }

        //listen for data from the remote client, and pass it to the IncomingMessage handler
        //this is blocking, so we need to peek before we read
        match stream.peek(&mut peek_buf) {
            Ok(1) => match websocket.read() {
                Ok(msg) => {
                    log::info!("Received websocket message. {msg:?}");

                    if let Message::Text(content) = msg {
                        log::info!("Received text: {content}");

                        if let Ok(deserialized) = serde_json::from_str(&content) {
                            log::info!("Deserialized incoming message {deserialized:?}");
                            task_send.send(deserialized).expect("Could not send");
                        } else {
                            log::info!("Error reading incoming message {content:?}");
                        }
                    }
                }
                Err(e) => match e {
                    tungstenite::Error::AlreadyClosed => {
                        log::info!("Client dropped connection");
                        break;
                    }
                    _ => log::error!("Error while handling message: {e:?}"),
                },
            },
            _ => {}
        }
    }

    *bloodmessage::MSGINFO_SEND.lock().unwrap() = None;
    *TASK_ENQUEUE.lock().unwrap() = None;
    drop(task_scaling);
    drop(task_msgs);
}
