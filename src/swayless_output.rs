use std::collections::{HashMap, HashSet};

use crate::swayless_connection::run_command;

pub struct SwaylessOutput {
    pub name: String,
    pub focused_tag: String,
    pub borrowed_tags: HashMap<String, HashSet<i64>>,
}

impl SwaylessOutput {
    pub fn new(name: &str, initial_tag: &str) -> Self {
        return Self {
            name: name.to_string(),
            focused_tag: initial_tag.to_string(),
            borrowed_tags: HashMap::new(),
        };
    }

    pub fn borrow_tag_container(&mut self, tag: &str, container: i64) -> bool {
        if self.borrowed_tags.contains_key(tag) {
            let borrowed = self.borrowed_tags.get_mut(tag).unwrap();
            borrowed.insert(container)
        } else {
            let mut borrowed = HashSet::new();
            borrowed.insert(container);
            self.borrowed_tags.insert(tag.to_string(), borrowed);
            return true;
        }
    }

    pub fn unborrow_tag_container(&mut self, tag: &str, borrowed_tag: i64) -> bool {
        if self.borrowed_tags.contains_key(tag) {
            let borrowed = self.borrowed_tags.get_mut(tag).unwrap();
            borrowed.remove(&borrowed_tag)
        } else {
            return false;
        }
    }

    pub fn return_containers(&mut self, borrowed_tag: &str) -> bool {
        let containers = self.borrowed_tags.get_mut(borrowed_tag);
        return match containers {
            None => {
                false
            }
            Some(containers) => {
                if containers.is_empty() {
                    return false;
                }
                for id in containers.iter() {
                    unsafe {
                        run_command(&format!(
                            "[ con_id={} ] move container to workspace {}",
                            id, borrowed_tag
                        ))
                    }
                }
                containers.clear();
                return true;
            }
        };
    }

    pub fn return_all_containers(&mut self) {
        for (tag, containers) in self.borrowed_tags.iter_mut() {
            if !containers.is_empty() {
                for id in containers.iter() {
                    unsafe {
                        run_command(&format!(
                            "[ con_id={} ] move container to workspace {}",
                            id, tag
                        ))
                    }
                }
            }
            containers.clear();
        }
    }
}