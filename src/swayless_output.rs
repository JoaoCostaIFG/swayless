use std::collections::{HashMap, HashSet};
use serde::__private::de::IdentifierDeserializer;

use crate::swayless_connection::{run_command};

pub struct SwaylessOutput {
    pub name: String,
    pub focused_tag: String,
    pub borrowed_tags: HashMap<String, HashSet<i64>>,
    pub prev_tag: String,
    pub prev_borrowed_tags: HashMap<String, HashSet<i64>>,
}

impl SwaylessOutput {
    pub fn new(name: &str, initial_tag: &str) -> Self {
        return Self {
            name: name.to_string(),
            focused_tag: initial_tag.to_string(),
            borrowed_tags: HashMap::new(),
            prev_tag: initial_tag.to_string(),
            prev_borrowed_tags: HashMap::new(),
        };
    }

    pub fn is_borrowing_tag(&self, tag: &str) -> bool {
        self.borrowed_tags.contains_key(tag)
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

    pub fn borrow_tag_containers(&mut self, tag: &str, containers: &Vec<i64>) {
        if self.borrowed_tags.contains_key(tag) {
            let borrowed = self.borrowed_tags.get_mut(tag).unwrap();
            borrowed.extend(containers);
        } else {
            let mut borrowed = HashSet::new();
            borrowed.extend(containers);
            self.borrowed_tags.insert(tag.to_string(), borrowed);
        }
    }

    pub fn unborrow_container(&mut self, borrowed_container: i64) -> bool {
        for (_tag, containers) in self.borrowed_tags.iter_mut() {
            if containers.remove(&borrowed_container) {
                return true;
            }
        }
        return false;
    }

    pub fn change_focused_tag(&mut self, new_tag: &str) {
        self.return_all_containers();

        self.prev_tag = self.focused_tag.to_string();
        self.prev_borrowed_tags = self.borrowed_tags.clone();

        self.focused_tag = new_tag.to_string();
        self.borrowed_tags = HashMap::new();

        unsafe { run_command(&format!("workspace {}", self.focused_tag)); }
    }

    pub fn alt_tab(&mut self) {
        self.return_all_containers();

        let prev_tag = self.prev_tag.clone();
        let prev_borrow = self.prev_borrowed_tags.clone();
        self.prev_tag = self.focused_tag.clone();
        self.prev_borrowed_tags = self.borrowed_tags.clone();
        self.focused_tag = prev_tag;
        self.borrowed_tags = prev_borrow;

        unsafe { run_command(&format!("workspace {}", self.focused_tag)); }
        for tag in self.borrowed_tags.keys() {
            unsafe {
                run_command(&format!("[ workspace={}$ ] move to workspace {}",
                                     tag, self.focused_tag
                ));
            }
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
                self.borrowed_tags.remove(borrowed_tag);
                return true;
            }
        };
    }

    fn return_all_containers(&mut self) {
        for (tag, containers) in self.borrowed_tags.iter_mut() {
            if containers.is_empty() {
                continue;
            }
            for id in containers.iter() {
                unsafe {
                    run_command(&format!("[ con_id={} ] move container to workspace {}", id, tag))
                }
            }
        }
    }
}