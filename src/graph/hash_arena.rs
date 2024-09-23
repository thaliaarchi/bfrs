use std::{
    cell::{Ref, RefCell, RefMut, UnsafeCell},
    hash::{BuildHasher, Hash},
};

use hashbrown::{hash_map::DefaultHashBuilder, hash_table::Entry, HashTable};

use crate::graph::arena::{Arena, Id};

// TODO:
// - Refactor ID transmutes.

/// An arena of unique values, identified by ID.
///
/// # Safety
///
/// It is undefined behavior to use an `Id` in any arena other than the one
/// which created it.
pub struct HashArena<T: Eq + Hash> {
    arena: Arena<RefCell<T>>,
    /// A table for deduplicating values. The key is the 0-based index of the
    /// value in the arena.
    table: UnsafeCell<HashTable<u32>>,
    hash_builder: DefaultHashBuilder,
}

pub struct ArenaRefMut<'a, T: Eq + Hash> {
    value: RefMut<'a, T>,
    index: u32,
    arena: &'a HashArena<T>,
}

impl<T: Eq + Hash> HashArena<T> {
    /// Constructs an empty hashed arena.
    #[inline]
    pub fn new() -> Self {
        HashArena {
            arena: Arena::new(),
            table: UnsafeCell::new(HashTable::new()),
            hash_builder: DefaultHashBuilder::default(),
        }
    }

    /// Inserts a node without deduplicating and returns its ID.
    pub fn insert_unique(&self, value: T) -> Id<T> {
        let id = self.arena.push(RefCell::new(value));
        unsafe { id.transmute() }
    }

    /// Inserts a node and returns its ID or the ID of an equivalent node
    /// already in the arena.
    pub fn insert(&self, value: T) -> Id<T> {
        let index = *self
            .entry(&value)
            .or_insert_with(|| self.arena.push(RefCell::new(value)).index())
            .get();
        unsafe { Id::from_index(index, &self.arena).transmute() }
    }

    /// Borrows the identified value.
    pub fn get(&self, id: Id<T>) -> Ref<'_, T> {
        self.arena[unsafe { id.transmute() }].borrow()
    }

    /// Mutably borrows the identified value.
    pub fn get_mut(&self, id: Id<T>) -> ArenaRefMut<'_, T> {
        let value = self.arena[unsafe { id.transmute() }].borrow_mut();
        match self.entry(&*value) {
            Entry::Occupied(entry) => {
                entry.remove();
            }
            Entry::Vacant(_) => unreachable!(),
        }
        ArenaRefMut {
            value,
            index: id.index(),
            arena: self,
        }
    }

    #[inline]
    fn entry(&self, value: &T) -> Entry<'_, u32> {
        let table = unsafe { &mut *self.table.get() };
        let hash = self.hash_builder.hash_one(value);
        let eq = |&index: &u32| {
            // SAFETY:
            // - The length of the arena monotonically increases, so if an index
            //   was valid on insertion, it remains valid.
            // - Only values with no mutable handle returned to the user are in
            //   the table.
            let key = unsafe { &*self.arena.get_unchecked(index).as_ptr() };
            value == key
        };
        let hasher = |&index: &u32| {
            // SAFETY: Same as above.
            let key = unsafe { &*self.arena.get_unchecked(index).as_ptr() };
            self.hash_builder.hash_one(key)
        };
        table.entry(hash, eq, hasher)
    }

    /// Returns the number of values in this arena.
    #[inline]
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// Returns whether this arena contains no values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Eq + Hash> Default for HashArena<T> {
    fn default() -> Self {
        HashArena::new()
    }
}

impl<T: Eq + Hash> Drop for ArenaRefMut<'_, T> {
    fn drop(&mut self) {
        match self.arena.entry(&self.value) {
            Entry::Occupied(_) => unreachable!(),
            Entry::Vacant(entry) => {
                entry.insert(self.index);
            }
        }
    }
}
