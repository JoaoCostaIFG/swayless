use std::borrow::Borrow;
use std::collections::HashMap;

use swayipc::{Connection, Output, Workspace};

fn run_command(sway_conn: &mut Connection, command: &str) {
    eprintln!("Running command: [cmd={}]", &command);

    let results = match sway_conn.run_command(&command) {
        Ok(results) => results,
        Err(err) => panic!(
            "Failed running command: [command={}], [error={}]",
            command, err
        ),
    };

    for res in results {
        if res.is_err() {
            eprintln!("Failed running command: [command={}]", command);
        }
    }
}

fn get_outputs(sway_conn: &mut Connection) -> Vec<Output> {
    let outputs = match sway_conn.get_outputs() {
        Ok(outputs) => outputs,
        Err(err) => panic!("Failed getting outputs: [error={}]", err),
    };
    outputs
}

fn get_current_output(sway_conn: &mut Connection) -> (usize, Output) {
    let mut outputs = get_outputs(sway_conn);

    let focused_output_index = match outputs.iter().position(|x| x.focused) {
        Some(i) => i,
        None => panic!("WTF! No focused output???"),
    };

    (focused_output_index, outputs.remove(focused_output_index))
}

pub struct Swayless {
    sway_conn: Connection,
    tags: HashMap<String, Vec<i64>>,
}

impl Swayless {
    pub fn new(initial_workspace: &str) -> Self {
        let connection = match Connection::new() {
            Ok(connection) => connection,
            Err(_e) => {
                panic!("couldn't find i3/sway socket");
            }
        };

        let mut selff = Self {
            sway_conn: connection,
            tags: HashMap::new(),
        };
        selff.init_outputs(initial_workspace);

        return selff;
    }

    fn init_outputs(&mut self, initial_workspace: &str) {
        let outputs = get_outputs(&mut self.sway_conn);
        for output in outputs.iter().filter(|x| x.active).rev() {
            run_command(&mut self.sway_conn, &format!("focus output {}", output.name));
            self.focus_to_workspace(initial_workspace);
        }
    }

    pub fn move_container_to_workspace(&mut self, workspace_name: &String) {
        let mut cmd: String = "move container to workspace ".to_string();
        let current_output_index = get_current_output(&mut self.sway_conn).0;
        let full_ws_name = self.get_workspace_name(workspace_name, current_output_index);
        cmd.push_str(&full_ws_name);
        run_command(&mut self.sway_conn, &cmd);
    }

    pub fn focus_to_workspace(&mut self, workspace_name: &str) {
        let mut cmd: String = "workspace ".to_string();
        let current_output_index = get_current_output(&mut self.sway_conn).0;
        let full_ws_name = self.get_workspace_name(workspace_name, current_output_index);
        cmd.push_str(&full_ws_name);
        run_command(&mut self.sway_conn, &cmd);
    }

    fn move_container_to_next_or_prev_output(&mut self, go_to_prev: bool) {
        let outputs = get_outputs(&mut self.sway_conn);
        let focused_output_index = match outputs.iter().position(|x| x.focused) {
            Some(i) => i,
            None => panic!("WTF! No focused output???"),
        };

        let target_output = if go_to_prev {
            &outputs[(focused_output_index + outputs.len() - 1) % outputs.len()]
        } else {
            &outputs[(focused_output_index + 1) % outputs.len()]
        };

        let workspaces = self.get_workspaces();
        let target_workspace = workspaces
            .iter()
            .find(|x| x.output == target_output.name && x.visible)
            .unwrap();

        // Move container to target workspace
        run_command(&mut self.sway_conn, format!("move container to workspace {}", target_workspace.name).as_str());
        // Focus that workspace to follow the container
        run_command(&mut self.sway_conn, format!("workspace {}", target_workspace.name).as_str());
    }

    pub fn move_container_to_next_output(&mut self) {
        self.move_container_to_next_or_prev_output(false);
    }

    pub fn move_container_to_prev_output(&mut self) {
        self.move_container_to_next_or_prev_output(true);
    }

    pub fn move_workspace_containers_to_here(&mut self, from_workspace_id: &str) {
        let (current_output_idx, current_output) = get_current_output(&mut self.sway_conn);
        let from_workspace_name = self.get_workspace_name(from_workspace_id, current_output_idx);
        if self.return_containers(&from_workspace_name) {
            return;
        }

        let tree = self.sway_conn.get_tree().unwrap();
        let current_output_node =
            match tree
                .nodes
                .into_iter()
                .find(|node| match node.name.borrow() {
                    Some(name) => *name == current_output.name,
                    None => false,
                }) {
                Some(node) => node,
                None => {
                    eprintln!(
                        "Failed to find the output in the tree: [output_name={}]",
                        current_output.name
                    );
                    return;
                }
            };

        let workspaces = self.get_workspaces();
        let to_workspace = workspaces
            .iter()
            .find(|x| x.output == current_output.name && x.visible)
            .unwrap();

        let from_workspace = match current_output_node.nodes.into_iter().find(|workspace| {
            match workspace.name.borrow() {
                Some(name) => *name == from_workspace_name,
                None => false,
            }
        }) {
            Some(workspace) => workspace,
            None => {
                eprintln!(
                    "From workspace doesn't exist: [workspace_name={}]",
                    from_workspace_name
                );
                return;
            }
        };

        let mut tags = self.tags.remove(&from_workspace_name).unwrap_or_default();
        for container in from_workspace.nodes.iter() {
            run_command(&mut self.sway_conn, &format!(
                "[ con_id={} ] move container to workspace {}",
                container.id, to_workspace.name
            ));
            tags.push(container.id);
        }
        self.tags.insert(from_workspace_name, tags);
    }

    fn return_containers(&mut self, workspace_name: &str) -> bool {
        let tags = self.tags.remove(workspace_name).unwrap_or_default();
        if !tags.is_empty() {
            for id in tags.iter() {
                run_command(&mut self.sway_conn, &format!(
                    "[ con_id={} ] move container to workspace {}",
                    id, workspace_name
                ))
            }
            return true;
        }
        return false;
    }

    fn return_all_containers(&mut self) {
        for (_key, tags) in self.tags.iter_mut() {
            if !tags.is_empty() {
                for id in tags.iter() {
                    run_command(&mut self.sway_conn, &format!(
                        "[ con_id={} ] move container to workspace {}",
                        id, "1"
                    ))
                }
            }
            tags.clear();
        }
    }

    fn get_workspaces(&mut self) -> Vec<Workspace> {
        let workspaces = match self.sway_conn.get_workspaces() {
            Ok(workspaces) => workspaces,
            Err(err) => panic!("Failed getting workspaces: [error={}]", err),
        };
        workspaces
    }

    fn get_workspace_name(&self, workspace_name: &str, output_index: usize) -> String {
        if output_index == 0 {
            format!("{}", workspace_name)
        } else {
            const SUPERSCRIPT_DIGITS: [&str; 10] =
                ["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
            let output_index_superscript: String = (output_index + 1)
                .to_string()
                .chars()
                .map(|c| SUPERSCRIPT_DIGITS[c.to_digit(10).unwrap() as usize])
                .collect();

            format!("{}{}", &workspace_name, output_index_superscript)
        }
    }
}
