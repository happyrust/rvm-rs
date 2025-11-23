use std::any::Any;

pub struct Arena {
    storage: Vec<Box<dyn Any>>,
    _chunk_size: usize,
}

impl Arena {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            storage: Vec::new(),
            _chunk_size: chunk_size,
        }
    }

    /// Safe, type-erased bump allocation. The returned reference
    /// remains valid for the lifetime of the arena.
    pub fn alloc<T: 'static>(&mut self, value: T) -> &mut T {
        self.storage.push(Box::new(value));
        let idx = self.storage.len() - 1;
        // Downcast is guaranteed to succeed because we just inserted this value.
        self.storage[idx]
            .downcast_mut::<T>()
            .expect("type mismatch in arena")
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new(1024)
    }
}
