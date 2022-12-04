extern crate clap;
extern crate serde_json;

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use swayless::Swayless;

use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread;
use std::sync::{Arc, Mutex, MutexGuard};
use swayipc::{Connection, Event, EventType, WorkspaceChange};

mod swayless;
mod swayless_output;
mod swayless_connection;

static SOCKET_PATH: &str = "/tmp/swayless.sock";

#[derive(Parser, Debug)]
#[clap(author, version, about = "Better multimonitor handling for sway", long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
enum Command {
    #[clap(about = "Initialize the workspaces for all the outputs")]
    Init,

    #[clap(about = "Move the focused container to another workspace on the same output")]
    Move(MoveAction),

    #[clap(about = "Focus to another workspace on the same output")]
    Focus(FocusAction),

    #[clap(about = "Move the focused container to the next output")]
    NextOutput,

    #[clap(about = "Move the focused container to the previous output")]
    PrevOutput,

    #[clap(about = "Move all containers in workspace to current workspace")]
    MoveWorkspaceHere(MoveHereAction),

    #[clap(about = "Go to the previous tag on the current container")]
    AltTab,
}

#[derive(Args, Debug, Serialize, Deserialize)]
struct FocusAction {
    #[clap(value_name = "index", help = "The index to focus on")]
    name: String,
}

#[derive(Args, Debug, Serialize, Deserialize)]
struct MoveAction {
    #[clap(value_name = "index", help = "The index to move the container to")]
    name: String,
}

#[derive(Args, Debug, Serialize, Deserialize)]
struct MoveHereAction {
    #[clap(value_name = "index", help = "The index to move the containers from")]
    name: String,
}

fn handle_focus_events(swayless_mutex: Arc<Mutex<Swayless>>) {
    let workspace_sub = Connection::new().unwrap().subscribe([EventType::Workspace]).unwrap();
    for ev in workspace_sub {
        let event = match ev {
            Ok(event) => event,
            Err(_err) => {
                eprintln!("Failure while watching events.");
                continue;
            }
        };
        match event {
            Event::Workspace(w) => {
                if w.change != WorkspaceChange::Focus {
                    continue;
                }
                let name = w.current.unwrap().name.unwrap();
                println!("Detected tag focus: [tag={}]", name);
                let mut swayless = swayless_mutex.lock().unwrap();
                swayless.update_focused(&name);
            }
            _ => unreachable!(),
        }
    }
}

fn handle_incoming_requests(swayless_mutex: Arc<Mutex<Swayless>>) {
    let socket = Path::new(SOCKET_PATH);
    if socket.exists() {
        eprintln!("Socket exists. Destroying it...");
        fs::remove_file(&socket).unwrap();
    }

    let listener = match UnixListener::bind(&socket) {
        Err(_) => panic!("failed to bind socket"),
        Ok(listener) => listener,
    };
    println!("Server started, waiting for clients.");

    // iterate over clients, blocks if no client available
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(err) => {
                eprintln!("Failed handling client request: [err={}]", err);
                continue;
            }
        };
        match serde_json::from_reader::<_, Command>(stream) {
            Ok(cmd) => {
                let mut swayless = swayless_mutex.lock().unwrap();
                handle_cmd(&mut swayless, &cmd)
            }
            Err(err) => {
                eprintln!("Failed reading request to command: [err={}]", err);
                continue;
            }
        };
    }
}

fn init() {
    let swayless_mutex = Arc::new(Mutex::new(Swayless::new("1")));
    let mut handles = vec![];

    let swayless_mutex_clone = Arc::clone(&swayless_mutex);
    handles.push(thread::spawn(move || handle_incoming_requests(swayless_mutex_clone)));
    let swayless_mutex_clone = Arc::clone(&swayless_mutex);
    handles.push(thread::spawn(move || handle_focus_events(swayless_mutex_clone)));

    for handle in handles {
        handle.join().unwrap();
    }
}

fn handle_cmd(swayless: &mut MutexGuard<Swayless>, cmd: &Command) {
    match cmd {
        Command::Init => {
            eprintln!("Shouldn't have received init command. Ignoring.");
        }
        Command::Move(action) => {
            swayless.move_container_to_workspace(&action.name);
        }
        Command::Focus(action) => {
            swayless.focus_to_workspace(&action.name);
        }
        Command::NextOutput => {
            swayless.move_container_to_next_output();
        }
        Command::PrevOutput => {
            swayless.move_container_to_prev_output();
        }
        Command::MoveWorkspaceHere(action) => {
            swayless.move_workspace_containers_to_here(&action.name);
        }
        Command::AltTab => {
            swayless.alt_tab_tag();
        }
    }
}

fn send_cmd(cmd: &Command) {
    let socket = Path::new(SOCKET_PATH);
    if !socket.exists() {
        panic!("Socket doesn't exist. Run init command first.");
    }

    let stream = match UnixStream::connect(&socket) {
        Ok(stream) => stream,
        Err(_) => panic!("Failed to connect to socket. Run init command first."),
    };

    serde_json::to_writer(stream, cmd).unwrap();
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Command::Init => {
            init();
        }
        _ => {
            send_cmd(&cli.command);
        }
    }
}
