use std::collections::{HashMap, HashSet};

use crate::swayless_connection::{run_command};

struct SwaylessTag {
    pub name: String,
    pub borrowed_tags: HashMap<String, HashSet<i64>>,
}

pub struct SwaylessOutput {
    pub name: String,
    tags: [SwaylessTag; 2],
}

impl SwaylessOutput {
    pub fn new(name: &str, initial_tag: &str) -> Self {
        return Self {
            name: name.to_string(),
            tags: [
                SwaylessTag {
                    name: initial_tag.to_string(),
                    borrowed_tags: HashMap::new(),
                },
                SwaylessTag {
                    name: initial_tag.to_string(),
                    borrowed_tags: HashMap::new(),
                },
            ],
        };
    }

    pub fn focused_tag(&self) -> &str {
        &self.tags[0].name
    }

    pub fn is_borrowing_tag(&self, tag: &str) -> bool {
        self.tags[0].borrowed_tags.contains_key(tag)
    }

    pub fn borrow_tag_container(&mut self, tag: &str, container: i64) -> bool {
        let borrowed_tags = &mut self.tags[0].borrowed_tags;

        if borrowed_tags.contains_key(tag) {
            let borrowed = borrowed_tags.get_mut(tag).unwrap();
            borrowed.insert(container)
        } else {
            let mut borrowed = HashSet::new();
            borrowed.insert(container);
            borrowed_tags.insert(tag.to_string(), borrowed);
            return true;
        }
    }

    pub fn borrow_tag_containers(&mut self, tag: &str, containers: &Vec<i64>) {
        let borrowed_tags = &mut self.tags[0].borrowed_tags;

        if borrowed_tags.contains_key(tag) {
            let borrowed = borrowed_tags.get_mut(tag).unwrap();
            borrowed.extend(containers);
        } else {
            let mut borrowed = HashSet::new();
            borrowed.extend(containers);
            borrowed_tags.insert(tag.to_string(), borrowed);
        }
    }

    pub fn unborrow_container(&mut self, borrowed_container: i64) -> bool {
        for (_tag, containers) in self.tags[0].borrowed_tags.iter_mut() {
            if containers.remove(&borrowed_container) {
                return true;
            }
        }
        return false;
    }

    pub fn change_focused_tag(&mut self, new_tag: &str) {
        self.return_all_containers();

        if self.focused_tag() != new_tag {
            self.tags.reverse();
            self.tags[0].name = new_tag.to_string();
            self.tags[0].borrowed_tags.clear();

            unsafe { run_command(&format!("workspace {}", new_tag)); }
        } else {
            self.tags[0].borrowed_tags.clear();
        }
    }

    pub fn alt_tab(&mut self) {
        self.return_all_containers();

        self.tags.reverse();

        let current_tag = &self.tags[0];
        unsafe { run_command(&format!("workspace {}", current_tag.name)); }
        for tag in current_tag.borrowed_tags.keys() {
            unsafe {
                run_command(&format!("[ workspace={}$ ] move to workspace {}",
                                     tag, current_tag.name
                ));
            }
        }
    }

    pub fn return_containers(&mut self, borrowed_tag: &str) -> bool {
        let borrowed_tags = &mut self.tags[0].borrowed_tags;
        let containers = borrowed_tags.remove(borrowed_tag);
        return match containers {
            None => false,
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
                return true;
            }
        };
    }

    fn return_all_containers(&mut self) {
        for (tag, containers) in self.tags[0].borrowed_tags.iter_mut() {
            for id in containers.iter() {
                unsafe {
                    run_command(&format!("[ con_id={} ] move container to workspace {}", id, tag))
                }
            }
        }
    }
}