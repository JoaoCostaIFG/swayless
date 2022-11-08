use std::borrow::Borrow;
use std::collections::HashMap;

use swayipc::{Connection, Output, Workspace};

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
        let outputs = self.get_outputs();
        for output in outputs.iter().filter(|x| x.active).rev() {
            self.run_command(&format!("focus output {}", output.name));
            self.focus_to_workspace(initial_workspace);
        }
    }

    pub fn move_container_to_workspace(&mut self, workspace_name: &String) {
        let mut cmd: String = "move container to workspace ".to_string();
        let current_output_index = self.get_current_output_index();
        let full_ws_name = self.get_container_name(workspace_name, current_output_index);
        cmd.push_str(&full_ws_name);
        self.run_command(&cmd);
    }

    pub fn focus_to_workspace(&mut self, workspace_name: &str) {
        let mut cmd: String = "workspace ".to_string();
        let current_output_index = self.get_current_output_index();
        let full_ws_name = self.get_container_name(workspace_name, current_output_index);
        cmd.push_str(&full_ws_name);
        self.run_command(&cmd);
    }

    pub fn focus_all_outputs_to_workspace(&mut self, workspace_name: &str) {
        let current_output = self.get_current_output_name();

        // Iterate on all outputs to focus on the given workspace
        let outputs = self.get_outputs();
        for output in outputs.iter() {
            let mut cmd: String = "focus output ".to_string();
            cmd.push_str(output.name.as_str());
            self.run_command(&cmd);

            self.focus_to_workspace(workspace_name);
        }

        // Get back to currently focused output
        let mut cmd: String = "focus output ".to_string();
        cmd.push_str(&current_output);
        self.run_command(&cmd);
    }

    fn move_container_to_next_or_prev_output(&mut self, go_to_prev: bool) {
        let outputs = self.get_outputs();
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
        let mut cmd: String = "move container to workspace ".to_string();
        cmd.push_str(&target_workspace.num.to_string());
        self.run_command(&cmd);

        // Focus that workspace to follow the container
        let mut cmd: String = "workspace ".to_string();
        cmd.push_str(&target_workspace.num.to_string());
        self.run_command(&cmd);
    }

    pub fn move_container_to_next_output(&mut self) {
        self.move_container_to_next_or_prev_output(false);
    }

    pub fn move_container_to_prev_output(&mut self) {
        self.move_container_to_next_or_prev_output(true);
    }

    pub fn move_workspace_containers_to_here(&mut self, from_workspace_id: &str) {
        let from_workspace_name = self.get_container_name(from_workspace_id, 0);
        if self.return_containers(&from_workspace_name) {
            return
        }

        let current_output_name = self.get_current_output_name();

        let tree = self.sway_conn.get_tree().unwrap();
        let current_output_node =
            match tree
                .nodes
                .into_iter()
                .find(|node| match node.name.borrow() {
                    Some(name) => *name == current_output_name,
                    None => false,
                }) {
                Some(node) => node,
                None => {
                    eprintln!(
                        "Failed to find the output in the tree: [output_name={}]",
                        current_output_name
                    );
                    return;
                }
            };

        let workspaces = self.get_workspaces();
        let to_workspace = workspaces
            .iter()
            .find(|x| x.output == current_output_name && x.visible)
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
            self.run_command(&format!(
                "[ con_id={} ] move container to workspace {}",
                container.id, to_workspace.name
            ));
            tags.push(container.id);
        }
        self.tags.insert(from_workspace_name, tags);
    }

    pub fn run_command(&mut self, command: &str) {
        eprintln!("Running command: [cmd={}]", &command);

        let results = match self.sway_conn.run_command(&command) {
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

    fn return_containers(&mut self, workspace_name: &str) -> bool {
        let tags = self.tags.remove(workspace_name).unwrap_or_default();
        if !tags.is_empty() {
            for id in tags.iter() {
                self.run_command(&format!(
                    "[ con_id={} ] move container to workspace {}",
                    id, workspace_name
                ))
            }
            return true;
        }
        return false;
    }

    fn get_outputs(&mut self) -> Vec<Output> {
        let outputs = match self.sway_conn.get_outputs() {
            Ok(outputs) => outputs,
            Err(err) => panic!("Failed getting outputs: [error={}]", err),
        };
        outputs
    }

    fn get_workspaces(&mut self) -> Vec<Workspace> {
        let workspaces = match self.sway_conn.get_workspaces() {
            Ok(workspaces) => workspaces,
            Err(err) => panic!("Failed getting workspaces: [error={}]", err),
        };
        workspaces
    }

    fn get_current_output_index(&mut self) -> usize {
        let outputs = self.get_outputs();

        let focused_output_index = match outputs.iter().position(|x| x.focused) {
            Some(i) => i,
            None => panic!("WTF! No focused output???"),
        };

        focused_output_index
    }

    fn get_current_output_name(&mut self) -> String {
        let outputs = self.get_outputs();

        let focused_output_index = match outputs.iter().find(|x| x.focused) {
            Some(i) => i.name.as_str(),
            None => panic!("WTF! No focused output???"),
        };

        focused_output_index.to_string()
    }

    fn get_container_name(&self, workspace_name: &str, output_index: usize) -> String {
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
