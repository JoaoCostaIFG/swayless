use clap::__macro_refs::once_cell::sync::Lazy;
use swayipc::{Connection, Output, Workspace};

pub static mut SWAY_CONN: Lazy<Connection> = Lazy::new(||Connection::new().unwrap());

pub unsafe fn run_command(command: &str) {
    eprintln!("Running command: [cmd={}]", &command);

    let results = match SWAY_CONN.run_command(&command) {
        Ok(results) => results,
        Err(_err) => panic!("Failed running command: [command={}]", command),
    };

    for res in results {
        if res.is_err() {
            eprintln!("Failed running command: [command={}]", command);
        }
    }
}

pub unsafe fn get_outputs() -> Vec<Output> {
    let outputs = match SWAY_CONN.get_outputs() {
        Ok(outputs) => outputs,
        Err(_err) => panic!("Failed getting outputs"),
    };
    outputs
}

pub unsafe fn get_current_output() -> (usize, Output) {
    let mut outputs = get_outputs();

    let focused_output_index = match outputs.iter().position(|x| x.focused) {
        Some(i) => i,
        None => panic!("WTF! No focused output???"),
    };

    (focused_output_index, outputs.remove(focused_output_index))
}

pub unsafe fn get_workspaces() -> Vec<Workspace> {
    let workspaces = match SWAY_CONN.get_workspaces() {
        Ok(workspaces) => workspaces,
        Err(_err) => panic!("Failed getting workspaces"),
    };
    workspaces
}
