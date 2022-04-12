extern crate byteorder;
extern crate clap;
extern crate serde_json;

use serde::{Deserialize, Serialize};

use clap::{App, Arg, SubCommand, crate_version, crate_authors};
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

fn get_stream() -> UnixStream {
    let socket_path = match env::var("I3SOCK") {
        Ok(val) => val,
        Err(_e) => {
            panic!("couldn't find i3/sway socket");
        }
    };

    let socket = Path::new(&socket_path);

    match UnixStream::connect(&socket) {
        Err(_) => panic!("couldn't connect to i3/sway socket"),
        Ok(stream) => stream,
    }
}

fn send_msg(mut stream: &UnixStream, msg_type: u32, payload: &str) {
    let payload_length = payload.len() as u32;

    let mut msg_prefix: [u8; 6 * mem::size_of::<u8>() + 2 * mem::size_of::<u32>()] =
        *b"i3-ipc00000000";

    msg_prefix[6..]
        .as_mut()
        .write_u32::<LittleEndian>(payload_length)
        .expect("Unable to write");

    msg_prefix[10..]
        .as_mut()
        .write_u32::<LittleEndian>(msg_type)
        .expect("Unable to write");

    let mut msg: Vec<u8> = msg_prefix[..].to_vec();
    msg.extend(payload.as_bytes());

    match stream.write_all(&msg[..]) {
        Err(_) => panic!("couldn't send message"),
        Ok(_) => {}
    }
}

fn send_command(stream: &UnixStream, command: &str) {
    eprint!("Sending command: '{}' - ", &command);
    send_msg(&stream, RUN_COMMAND, &command);
    check_success(&stream);
}

fn read_msg(mut stream: &UnixStream) -> Result<String, &str> {
    let mut response_header: [u8; 14] = *b"uninitialized.";
    stream.read_exact(&mut response_header).unwrap();

    if &response_header[0..6] == b"i3-ipc" {
        let mut v = Cursor::new(vec![
            response_header[6],
            response_header[7],
            response_header[8],
            response_header[9],
        ]);
        let payload_length = v.read_u32::<LittleEndian>().unwrap();

        let mut payload = vec![0; payload_length as usize];
        stream.read_exact(&mut payload[..]).unwrap();
        let payload_str = String::from_utf8(payload).unwrap();
        Ok(payload_str)
    } else {
        eprint!("Not an i3-icp packet, emptying the buffer: ");
        let mut v = vec![];
        stream.read_to_end(&mut v).unwrap();
        eprintln!("{:?}", v);
        Err("Unable to read i3-ipc packet")
    }
}

fn check_success(stream: &UnixStream) {
    match read_msg(&stream) {
        Ok(msg) => {
            let r: Vec<serde_json::Value> = serde_json::from_str(&msg).unwrap();
            match r[0]["success"] {
                serde_json::Value::Bool(true) => eprintln!("Command successful"),
                _ => panic!("Command failed: {:#?}", r),
            }
        }
        Err(_) => panic!("Unable to read response"),
    };
}

#[derive(Serialize, Deserialize)]
struct Output {
    name: String,
    focused: bool,
    active: bool,
}

fn get_outputs(stream: &UnixStream) -> Vec<Output> {
    send_msg(&stream, GET_OUTPUTS, "");
    let o = match read_msg(&stream) {
        Ok(msg) => msg,
        Err(_) => panic!("Unable to get outputs"),
    };
    let mut outputs: Vec<Output> = serde_json::from_str(&o).unwrap();
    outputs.sort_by(|x, y| x.name.cmp(&y.name));  // sort_by_key doesn't work here (https://stackoverflow.com/a/47126516)
    outputs
}

#[derive(Serialize, Deserialize)]
struct Workspace {
    num: u32,
    output: String,
    visible: bool,
}

fn get_workspaces(stream: &UnixStream) -> Vec<Workspace> {
    send_msg(&stream, GET_WORKSPACES, "");
    let ws = match read_msg(&stream) {
        Ok(msg) => msg,
        Err(_) => panic!("Unable to get current workspace"),
    };
    let mut workspaces: Vec<Workspace> = serde_json::from_str(&ws).unwrap();
    workspaces.sort_by_key(|x| x.num);
    workspaces
}

fn get_current_output_index(stream: &UnixStream) -> String {
    let outputs = get_outputs(&stream);

    let focused_output_index = match outputs
        .iter()
        .position(|x| x.focused)
    {
        Some(i) => i,
        None => panic!("WTF! No focused output???"),
    };

    format!("{}", focused_output_index)
}

fn get_current_output_name(stream: &UnixStream) -> String {
    let outputs = get_outputs(&stream);

    let focused_output_index = match outputs
        .iter()
        .find(|x| x.focused)
    {
        Some(i) => i.name.as_str(),
        None => panic!("WTF! No focused output???"),
    };

    format!("{}", focused_output_index)
}

fn move_container_to_workspace(stream: &UnixStream, workspace_name: &String) {
    let mut cmd: String = "move container to workspace number ".to_string();
    let full_ws_name = format!("{}{}", get_current_output_index(stream), &workspace_name)
        .parse::<i32>()
        .unwrap();
    cmd.push_str(&full_ws_name.to_string());
    send_command(&stream, &cmd);
}

fn focus_to_workspace(stream: &UnixStream, workspace_name: &String) {
    let mut cmd: String = "workspace number ".to_string();
    let full_ws_name = format!("{}{}", get_current_output_index(stream), &workspace_name)
        .parse::<i32>()
        .unwrap();
    cmd.push_str(&full_ws_name.to_string());
    send_command(&stream, &cmd);
}

fn focus_all_outputs_to_workspace(stream: &UnixStream, workspace_name: &String) {
    let current_output = get_current_output_name(stream);

    // Iterate on all outputs to focus on the given workspace
    let outputs = get_outputs(&stream);
    for output in outputs.iter() {
        let mut cmd: String = "focus output ".to_string();
        cmd.push_str(&output.name.as_str());
        send_command(&stream, &cmd);

        focus_to_workspace(&stream, &workspace_name);
    }

    // Get back to currently focused output
    let mut cmd: String = "focus output ".to_string();
    cmd.push_str(&current_output);
    send_command(&stream, &cmd);
}

fn move_container_to_next_output(stream: &UnixStream) {
    move_container_to_next_or_prev_output(&stream, false);
}

fn move_container_to_prev_output(stream: &UnixStream) {
    move_container_to_next_or_prev_output(&stream, true);
}

fn move_container_to_next_or_prev_output(stream: &UnixStream, go_to_prev: bool) {
    let outputs = get_outputs(&stream);
    let focused_output_index = match outputs
        .iter()
        .position(|x| x.focused)
    {
        Some(i) => i,
        None => panic!("WTF! No focused output???"),
    };

    let target_output;
    if go_to_prev {
        target_output = &outputs[(focused_output_index - 1 + &outputs.len()) % &outputs.len()];
    } else {
        target_output = &outputs[(focused_output_index + 1) % &outputs.len()];
    }

    let workspaces = get_workspaces(&stream);
    let target_workspace = workspaces
        .iter()
        .filter(|x| {
            x.output == target_output.name && x.visible
        })
        .next()
        .unwrap();

    // Move container to target workspace
    let mut cmd: String = "move container to workspace number ".to_string();
    cmd.push_str(&target_workspace.num.to_string());
    send_command(&stream, &cmd);

    // Focus that workspace to follow the container
    let mut cmd: String = "workspace number ".to_string();
    cmd.push_str(&target_workspace.num.to_string());
    send_command(&stream, &cmd);
}

fn init_workspaces(stream: &UnixStream, workspace_name: &String) {
    let outputs = get_outputs(&stream);

    let cmd_prefix: String = "focus output ".to_string();
    for output in outputs.iter().filter(|x| x.active).rev() {
        let mut cmd = cmd_prefix.clone();
        cmd.push_str(&output.name.as_str());
        send_command(&stream, &cmd);
        focus_to_workspace(&stream, &workspace_name);
    }
}

fn main() {
    let matches = App::new("swaysome")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Better multimonitor handling for sway")
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize the workspaces for all the outputs")
                .arg(
                    Arg::with_name("index")
                        .help("The index to initialize with")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("focus")
                .about("Focus to another workspace on the same output")
                .arg(
                    Arg::with_name("index")
                        .help("The index to focus on")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("focus_all_outputs")
                .about("Focus to another workspace on all the outputs")
                .arg(
                    Arg::with_name("index")
                        .help("The index to focus on")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("move")
                .about("Move the focused container to another workspace on the same output")
                .arg(
                    Arg::with_name("index")
                        .help("The index to move the container to")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("next_output")
                .about("Move the focused container to the next output"),
        )
        .subcommand(
            SubCommand::with_name("prev_output")
                .about("Move the focused container to the previous output"),
        )
        .get_matches();

    let stream = get_stream();

    if let Some(matches) = matches.subcommand_matches("init") {
        init_workspaces(&stream, &matches.value_of("index").unwrap().to_string());
    } else if let Some(matches) = matches.subcommand_matches("move") {
        move_container_to_workspace(&stream, &matches.value_of("index").unwrap().to_string());
    } else if let Some(matches) = matches.subcommand_matches("focus") {
        focus_to_workspace(&stream, &matches.value_of("index").unwrap().to_string());
    } else if let Some(matches) = matches.subcommand_matches("focus_all_outputs") {
        focus_all_outputs_to_workspace(&stream, &matches.value_of("index").unwrap().to_string());
    } else if let Some(_) = matches.subcommand_matches("next_output") {
        move_container_to_next_output(&stream);
    } else if let Some(_) = matches.subcommand_matches("prev_output") {
        move_container_to_prev_output(&stream);
    }
}
