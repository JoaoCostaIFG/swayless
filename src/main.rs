extern crate byteorder;
extern crate clap;
extern crate serde_json;

use serde::{Deserialize, Serialize};

use swayipc::{Connection, Fallible};

use clap::{Args, Parser, Subcommand};
use std::env;
use std::io::Cursor;
use std::io::{Read, Write};
use std::mem;
use std::os::unix::net::UnixStream;
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

const RUN_COMMAND: u32 = 0;
const GET_WORKSPACES: u32 = 1;
// const SUBSCRIBE: u32 = 2;
const GET_OUTPUTS: u32 = 3;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Better multimonitor handling for sway", long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[clap(about = "Initialize the workspaces for all the outputs")]
    Init(InitAction),

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

#[derive(Args, Debug)]
struct InitAction {
    #[clap(value_name = "index", help = "The index to initialize with")]
    name: String,
}

#[derive(Args, Debug)]
struct FocusAction {
    #[clap(value_name = "index", help = "The index to focus on")]
    name: String,
}

#[derive(Args, Debug)]
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

fn send_command(connection: &Connection, command: &str) -> Vec<Result<(), swayipc::Error>> {
    eprint!("Sending command: '{}' - ", &command);
    let result = connection.run_command(&command).unwrap();
    result
}

#[derive(Serialize, Deserialize)]
struct Output {
    name: String,
    #[serde(default)]
    focused: bool,
    active: bool,
}

fn get_outputs(connection: &Connection) -> Vec<Output> {
    send_msg(stream, GET_OUTPUTS, "");
    let o = match read_msg(stream) {
        Ok(msg) => msg,
        Err(_) => panic!("Unable to get outputs"),
    };
    let mut outputs: Vec<Output> = serde_json::from_str(&o).unwrap();
    outputs.sort_by(|x, y| x.name.cmp(&y.name)); // sort_by_key doesn't work here (https://stackoverflow.com/a/47126516)
    outputs
}

#[derive(Serialize, Deserialize)]
struct Workspace {
    num: u32,
    output: String,
    visible: bool,
}

fn get_workspaces(connection: &Connection) -> Vec<Workspace> {
    send_msg(stream, GET_WORKSPACES, "");
    let ws = match read_msg(stream) {
        Ok(msg) => msg,
        Err(_) => panic!("Unable to get current workspace"),
    };
    let mut workspaces: Vec<Workspace> = serde_json::from_str(&ws).unwrap();
    workspaces.sort_by_key(|x| x.num);
    workspaces
}

fn get_current_output_index(connection: &Connection) -> usize {
    let outputs = get_outputs(stream);

    let focused_output_index = match outputs.iter().position(|x| x.focused) {
        Some(i) => i,
        None => panic!("WTF! No focused output???"),
    };

    focused_output_index
}

fn get_current_output_name(connection: &Connection) -> String {
    let outputs = get_outputs(stream);

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

fn move_container_to_workspace(connection: &Connection, workspace_name: &String) {
    let mut cmd: String = "move container to workspace ".to_string();
    let full_ws_name = get_container_name(workspace_name, get_current_output_index(stream));
    cmd.push_str(&full_ws_name);
    send_command(stream, &cmd);
}

fn focus_to_workspace(connection: &Connection, workspace_name: &String) {
    let mut cmd: String = "workspace ".to_string();
    let full_ws_name = get_container_name(workspace_name, get_current_output_index(stream));
    cmd.push_str(&full_ws_name);
    send_command(stream, &cmd);
}

fn focus_all_outputs_to_workspace(connection: &Connection, workspace_name: &String) {
    let current_output = get_current_output_name(stream);

    // Iterate on all outputs to focus on the given workspace
    let outputs = get_outputs(stream);
    for output in outputs.iter() {
        let mut cmd: String = "focus output ".to_string();
        cmd.push_str(output.name.as_str());
        send_command(stream, &cmd);

        focus_to_workspace(stream, workspace_name);
    }

    // Get back to currently focused output
    let mut cmd: String = "focus output ".to_string();
    cmd.push_str(&current_output);
    send_command(stream, &cmd);
}

fn move_container_to_next_or_prev_output(connection: &Connection, go_to_prev: bool) {
    let outputs = get_outputs(stream);
    let focused_output_index = match outputs.iter().position(|x| x.focused) {
        Some(i) => i,
        None => panic!("WTF! No focused output???"),
    };

    let target_output = if go_to_prev {
        &outputs[(focused_output_index + outputs.len() - 1) % outputs.len()]
    } else {
        &outputs[(focused_output_index + 1) % outputs.len()]
    };

    let workspaces = get_workspaces(stream);
    let target_workspace = workspaces
        .iter()
        .find(|x| x.output == target_output.name && x.visible)
        .unwrap();

    // Move container to target workspace
    let mut cmd: String = "move container to workspace ".to_string();
    cmd.push_str(&target_workspace.num.to_string());
    send_command(stream, &cmd);

    // Focus that workspace to follow the container
    let mut cmd: String = "workspace ".to_string();
    cmd.push_str(&target_workspace.num.to_string());
    send_command(stream, &cmd);
}

fn move_container_to_next_output(connection: &Connection) {
    move_container_to_next_or_prev_output(stream, false);
}

fn move_container_to_prev_output(connection: &Connection) {
    move_container_to_next_or_prev_output(stream, true);
}

fn init_workspaces(sway_conn: &Connection, workspace_name: &String) {
    let outputs = get_outputs(stream);

    let cmd_prefix: String = "focus output ".to_string();
    for output in outputs.iter().filter(|x| x.active).rev() {
        let mut cmd = cmd_prefix.clone();
        cmd.push_str(output.name.as_str());
        send_command(stream, &cmd);
        focus_to_workspace(stream, workspace_name);
    }
}

fn main() {
    let cli = Cli::parse();
    let sway_conn = get_sway_conn();

    match &cli.command {
        Command::Init(action) => {
            init_workspaces(&sway_conn, &action.name);
        }
        Command::Move(action) => {
            move_container_to_workspace(&sway_conn, &action.name);
        }
        Command::Focus(action) => {
            focus_to_workspace(&sway_conn, &action.name);
        }
        Command::FocusAllOutputs(action) => {
            focus_all_outputs_to_workspace(&sway_conn, &action.name);
        }
        Command::NextOutput => {
            move_container_to_next_output(&sway_conn);
        }
        Command::PrevOutput => {
            move_container_to_prev_output(&sway_conn);
        }
    }
}
