extern crate byteorder;
extern crate clap;
extern crate serde_json;

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use swayipc::{Connection, Output, Workspace};

use std::fmt::format;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

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

    #[clap(about = "Focus to another workspace on all the outputs")]
    FocusAllOutputs(FocusAction),

    #[clap(about = "Move the focused container to the next output")]
    NextOutput,

    #[clap(about = "Move the focused container to the previous output")]
    PrevOutput,
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

fn get_sway_conn() -> Connection {
    let mut connection = match Connection::new() {
        Ok(connection) => connection,
        Err(_e) => {
            panic!("couldn't find i3/sway socket");
        }
    };
    connection
}

fn run_command(connection: &mut Connection, command: &str) {
    eprintln!("Running command: [cmd={}]", &command);
    let results = match connection.run_command(&command) {
        Ok(results) => results,
        Err(err) => panic!(
            "Failed running command: [command={}], [error={}]",
            command, err
        ),
    };

    for res in results {
        if res.is_err() {
            panic!("Failed running command: [command={}]", command)
        }
    }
}

fn get_outputs(connection: &mut Connection) -> Vec<Output> {
    let outputs = match connection.get_outputs() {
        Ok(outputs) => outputs,
        Err(err) => panic!("Failed getting outputs: [error={}]", err),
    };
    outputs
}

fn get_workspaces(connection: &mut Connection) -> Vec<Workspace> {
    let workspaces = match connection.get_workspaces() {
        Ok(workspaces) => workspaces,
        Err(err) => panic!("Failed getting workspaces: [error={}]", err),
    };
    workspaces
}

fn get_current_output_index(connection: &mut Connection) -> usize {
    let outputs = get_outputs(connection);

    let focused_output_index = match outputs.iter().position(|x| x.focused) {
        Some(i) => i,
        None => panic!("WTF! No focused output???"),
    };

    focused_output_index
}

fn get_current_output_name(connection: &mut Connection) -> String {
    let outputs = get_outputs(connection);

    let focused_output_index = match outputs.iter().find(|x| x.focused) {
        Some(i) => i.name.as_str(),
        None => panic!("WTF! No focused output???"),
    };

    focused_output_index.to_string()
}

fn get_container_name(workspace_name: &String, output_index: usize) -> String {
    if output_index == 0 {
        format!("{}", workspace_name)
    } else {
        const SUPERSCRIPT_DIGITS: [&str; 10] = ["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
        let output_index_superscript: String = (output_index + 1)
            .to_string()
            .chars()
            .map(|c| SUPERSCRIPT_DIGITS[c.to_digit(10).unwrap() as usize])
            .collect();

        format!("{}{}", &workspace_name, output_index_superscript)
    }
}

fn move_container_to_workspace(connection: &mut Connection, workspace_name: &String) {
    let mut cmd: String = "move container to workspace ".to_string();
    let full_ws_name = get_container_name(workspace_name, get_current_output_index(connection));
    cmd.push_str(&full_ws_name);
    run_command(connection, &cmd);
}

fn focus_to_workspace(connection: &mut Connection, workspace_name: &String) {
    let mut cmd: String = "workspace ".to_string();
    let full_ws_name = get_container_name(workspace_name, get_current_output_index(connection));
    cmd.push_str(&full_ws_name);
    run_command(connection, &cmd);
}

fn focus_all_outputs_to_workspace(connection: &mut Connection, workspace_name: &String) {
    let current_output = get_current_output_name(connection);

    // Iterate on all outputs to focus on the given workspace
    let outputs = get_outputs(connection);
    for output in outputs.iter() {
        let mut cmd: String = "focus output ".to_string();
        cmd.push_str(output.name.as_str());
        run_command(connection, &cmd);

        focus_to_workspace(connection, workspace_name);
    }

    // Get back to currently focused output
    let mut cmd: String = "focus output ".to_string();
    cmd.push_str(&current_output);
    run_command(connection, &cmd);
}

fn move_container_to_next_or_prev_output(connection: &mut Connection, go_to_prev: bool) {
    let outputs = get_outputs(connection);
    let focused_output_index = match outputs.iter().position(|x| x.focused) {
        Some(i) => i,
        None => panic!("WTF! No focused output???"),
    };

    let target_output = if go_to_prev {
        &outputs[(focused_output_index + outputs.len() - 1) % outputs.len()]
    } else {
        &outputs[(focused_output_index + 1) % outputs.len()]
    };

    let workspaces = get_workspaces(connection);
    let target_workspace = workspaces
        .iter()
        .find(|x| x.output == target_output.name && x.visible)
        .unwrap();

    // Move container to target workspace
    let mut cmd: String = "move container to workspace ".to_string();
    cmd.push_str(&target_workspace.num.to_string());
    run_command(connection, &cmd);

    // Focus that workspace to follow the container
    let mut cmd: String = "workspace ".to_string();
    cmd.push_str(&target_workspace.num.to_string());
    run_command(connection, &cmd);
}

fn move_container_to_next_output(connection: &mut Connection) {
    move_container_to_next_or_prev_output(connection, false);
}

fn move_container_to_prev_output(connection: &mut Connection) {
    move_container_to_next_or_prev_output(connection, true);
}

fn init() {
    let mut sway_conn = get_sway_conn();

    let outputs = get_outputs(&mut sway_conn);

    let cmd_prefix: String = "focus output ".to_string();
    for output in outputs.iter().filter(|x| x.active).rev() {
        let mut cmd = cmd_prefix.clone();
        cmd.push_str(output.name.as_str());
        //run_command(&mut sway_conn, &cmd);
        //focus_to_workspace(&mut sway_conn, &1.to_string());
    }

    listen_to_cmds();
}

fn listen_to_cmds() {
    let socket = Path::new(SOCKET_PATH);
    if socket.exists() {
        eprintln!("Socket exists. Destroying it...");
        fs::remove_file(&socket).unwrap();
    }

    let listener = match UnixListener::bind(&socket) {
        Err(_) => panic!("failed to bind socket"),
        Ok(listener) => listener,
    };

    println!("Server started, waiting for clients");

    // iterate over clients, blocks if no client available
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // connection succeeded
                let cmd: Command = serde_json::from_reader(stream).unwrap();
                handle_cmd(&cmd);
            }
            Err(err) => eprintln!("Failed handling client request: [err={}]", err),
        };
    }
}

fn handle_cmd(cmd: &Command) {
    match cmd {
        Command::Init => {
            eprintln!("Shouldn't have received init command. Ignoring.");
        }
        Command::Move(action) => {
            println!("{}", action.name);
        }
        Command::Focus(action) => {
            println!("{}", action.name);
        }
        Command::FocusAllOutputs(action) => {
            println!("{}", action.name);
        }
        Command::NextOutput => {
            println!("next");
        }
        Command::PrevOutput => {
            println!("prev");
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
        Err(_) => panic!("Failed to bind socket."),
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
