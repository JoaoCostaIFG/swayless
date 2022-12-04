use clap::__macro_refs::once_cell::sync::Lazy;
use swayipc::{Connection, Node, Output, Workspace};

pub static mut SWAY_CONN: Lazy<Connection> = Lazy::new(|| Connection::new().unwrap());

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

pub unsafe fn get_current_workspace() -> Workspace {
    let mut workspaces = get_workspaces();
    let workspace_idx = workspaces
        .iter()
        .position(|x| x.focused)
        .unwrap();
    workspaces.remove(workspace_idx)
}

pub unsafe fn get_visible_workspace(output: &Output) -> Workspace {
    let mut workspaces = get_workspaces();
    let workspace_idx = workspaces
        .iter()
        .position(|x| x.output == output.name && x.visible)
        .unwrap();
    workspaces.remove(workspace_idx)
}

pub unsafe fn get_containers(current_output: &Output, workspace_name: &str) -> Vec<i64> {
    let tree = SWAY_CONN.get_tree().unwrap();
    let current_output_node = tree.nodes.iter()
        .find(|node| *node.name.as_ref().unwrap() == current_output.name).unwrap();

    let workspace_node = current_output_node.nodes.iter()
        .find(|workspace| *workspace.name.as_ref().unwrap() == workspace_name).unwrap();

    return workspace_node.nodes.iter().map(|node| node.id).collect();
}

pub unsafe fn get_current_container(current_output: &Output) -> i64 {
    let tree = SWAY_CONN.get_tree().unwrap();
    let current_output_node = tree.nodes.iter()
        .find(|node| *node.name.as_ref().unwrap() == current_output.name).unwrap();

    // note: at the time of writting there is a problem with the get_tree command where all
    // workspace nodes report not being focused. This also happens with swaymsg on the command line.
    // The get_workspaces command works correctly
    let current_workspace = get_current_workspace();
    let workspace_node = current_output_node.nodes.iter()
        .find(|node| *node.name.as_ref().unwrap() == current_workspace.name).unwrap();

    match workspace_node.nodes.iter().find(|container| container.focused) {
        None => -1,
        Some(container) => container.id,
    }
}
