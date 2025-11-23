use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringId(pub u32);

pub struct StringInterner {
    strings: HashMap<String, StringId>,
    storage: Vec<String>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            storage: Vec::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.strings.get(s) {
            return id;
        }

        let id = StringId(self.storage.len() as u32);
        self.storage.push(s.to_string());
        self.strings.insert(s.to_string(), id);
        id
    }

    pub fn get(&self, id: StringId) -> &str {
        &self.storage[id.0 as usize]
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}
