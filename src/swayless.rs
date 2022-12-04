use std::collections::{HashMap};

use crate::swayless_output::SwaylessOutput;
use crate::swayless_connection::{get_containers, get_current_container, get_current_output, get_outputs, get_visible_workspace, run_command};

pub struct Swayless {
    /// The swayless outputs
    sway_outputs: HashMap<String, SwaylessOutput>,
}

impl Swayless {
    pub fn new(initial_workspace: &str) -> Self {
        let mut selff = Self {
            sway_outputs: HashMap::new(),
        };
        unsafe { selff.init_outputs(initial_workspace); }
        return selff;
    }

    /// Make each output focus its initial workspace
    unsafe fn init_outputs(&mut self, initial_workspace: &str) {
        for output in get_outputs().iter().filter(|x| x.active).rev() {
            run_command(&format!("focus output {}", output.name));
            self.sway_outputs.insert(
                output.name.clone(),
                SwaylessOutput::new(&output.name, initial_workspace),
            );
            self.focus_to_workspace(initial_workspace);
        }
    }

    /// Change focus to the given tag in the current output
    pub fn focus_to_workspace(&mut self, tag: &str) {
        let (current_output_idx, current_output) = unsafe { get_current_output() };
        let workspace_name = self.get_workspace_name(tag, current_output_idx);

        let sway_output = self.sway_outputs.get_mut(&current_output.name).unwrap();
        sway_output.return_all_containers();
        unsafe { run_command(&format!("workspace {}", workspace_name)); }
        sway_output.change_focused_tag(&workspace_name);
    }

    /// Move a container to another workspace
    pub fn move_container_to_workspace(&mut self, tag: &str) {
        let (current_output_idx, current_output) = unsafe { get_current_output() };
        let workspace_name = self.get_workspace_name(tag, current_output_idx);

        let sway_output = self.sway_outputs.get_mut(&current_output.name).unwrap();
        unsafe {
            if sway_output.is_borrowing_tag(&workspace_name) {
                sway_output.borrow_tag_container(&tag, get_current_container(&current_output));
                run_command(&format!("move container to workspace {}", sway_output.focused_tag));
            } else {
                sway_output.unborrow_container(get_current_container(&current_output));
                run_command(&format!("move container to workspace {}", workspace_name));
            }
        }
    }

    /// Move a container to next/previous output. It wraps around
    unsafe fn move_container_to_next_or_prev_output(&mut self, go_to_prev: bool) {
        let outputs = get_outputs();
        let focused_output_index = match outputs.iter().position(|x| x.focused) {
            Some(i) => i,
            None => panic!("WTF! No focused output???"),
        };

        let target_output = if go_to_prev {
            &outputs[(focused_output_index + outputs.len() - 1) % outputs.len()]
        } else {
            &outputs[(focused_output_index + 1) % outputs.len()]
        };

        let target_workspace = get_visible_workspace(&target_output);
        // Move container to target workspace
        run_command(&format!("move container to workspace {}", target_workspace.name));
        // Focus that workspace to follow the container
        run_command(&format!("workspace {}", target_workspace.name));
    }

    pub fn move_container_to_next_output(&mut self) {
        unsafe { self.move_container_to_next_or_prev_output(false); }
    }

    pub fn move_container_to_prev_output(&mut self) {
        unsafe { self.move_container_to_next_or_prev_output(true); }
    }

    /// Move containers on a given tag to the current tag. They are borrowed
    pub fn move_workspace_containers_to_here(&mut self, from_tag: &str) {
        let (current_output_idx, current_output) = unsafe { get_current_output() };
        let from_workspace_name = self.get_workspace_name(from_tag, current_output_idx);
        let sway_output = self.sway_outputs.get_mut(&current_output.name).unwrap();
        if sway_output.return_containers(&from_workspace_name) {
            return;
        }

        let containers = unsafe { get_containers(&current_output, &from_workspace_name) };
        sway_output.borrow_tag_containers(&from_workspace_name, &containers);
        unsafe {
            run_command(&format!("[ workspace={} ] move container to workspace {}",
                                 from_workspace_name, sway_output.focused_tag
            ));
        }
    }

    pub fn alt_tab_tag(&mut self) {
        let (_, current_output) = unsafe { get_current_output() };
        let sway_output = self.sway_outputs.get_mut(&current_output.name).unwrap();
        sway_output.alt_tab();
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
