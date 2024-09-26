#[cfg(not(debug_assertions))]
use std::hint::unreachable_unchecked;
use std::{
    cell::{Ref, RefCell, RefMut, UnsafeCell},
    fmt::{self, Debug, Formatter},
    hash::{BuildHasher, Hash},
    ops::{Deref, DerefMut},
    ptr,
};

use hashbrown::{
    hash_map::DefaultHashBuilder,
    hash_table::{Entry as TableEntry, OccupiedEntry},
    HashTable,
};

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
    arena: Arena<Entry<T>>,
    /// A table for deduplicating values. The key is the 0-based index of the
    /// value in the arena.
    table: UnsafeCell<HashTable<u32>>,
    hash_builder: DefaultHashBuilder,
}

enum Entry<T> {
    Occupied { value: RefCell<T>, unique: bool },
    Replaced(u32),
}

pub struct ArenaRef<'a, T: Eq + Hash> {
    value: Ref<'a, T>,
    arena: &'a HashArena<T>,
}

pub struct ArenaRefMut<'a, T: Eq + Hash> {
    value: RefMut<'a, T>,
    index: u32,
    unique: bool,
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
        let id = self.arena.push(Entry::Occupied {
            value: RefCell::new(value),
            unique: true,
        });
        unsafe { id.transmute() }
    }

    /// Inserts a node and returns its ID or the ID of an equivalent node
    /// already in the arena.
    pub fn insert(&self, value: T) -> Id<T> {
        let index = *self
            .table_entry(&value)
            .or_insert_with(|| {
                let entry = Entry::Occupied {
                    value: RefCell::new(value),
                    unique: false,
                };
                self.arena.push(entry).index()
            })
            .get();
        unsafe { Id::from_index(index, &self.arena).transmute() }
    }

    /// Gets the ID of a value.
    pub fn find(&self, value: &T) -> Option<Id<T>> {
        self.table_find(value)
            .map(|index| unsafe { Id::from_index(index, &self.arena).transmute() })
    }

    /// Borrows the identified value.
    pub fn get(&self, id: Id<T>) -> ArenaRef<'_, T> {
        let value = self.arena_entry(id).0.borrow();
        ArenaRef { value, arena: self }
    }

    /// Mutably borrows the identified value.
    pub fn get_mut(&self, id: Id<T>) -> ArenaRefMut<'_, T> {
        let (cell, unique) = self.arena_entry(id);
        let value = cell.borrow_mut();
        if !unique {
            self.table_find_entry(&*value).unwrap().remove();
        }
        ArenaRefMut {
            value,
            index: id.index(),
            unique,
            arena: self,
        }
    }

    /// Gets a reference to the identified node, without checking that it is
    /// not immutably borrowed.
    pub unsafe fn get_unchecked(&self, id: Id<T>) -> &T {
        unsafe { &*self.arena_entry(id).0.as_ptr() }
    }

    #[inline]
    fn arena_entry(&self, id: Id<T>) -> (&RefCell<T>, bool) {
        let mut index = id.index();
        loop {
            match unsafe { self.arena.get_unchecked(index) } {
                Entry::Occupied { value, unique } => break (value, *unique),
                &Entry::Replaced(replaced) => index = replaced,
            }
        }
    }

    #[inline]
    fn table_entry(&self, value: &T) -> TableEntry<'_, u32> {
        let table = unsafe { &mut *self.table.get() };
        let hash = self.hash_builder.hash_one(value);
        let eq = |&index: &u32| {
            // SAFETY:
            // - The length of the arena monotonically increases, so if an index
            //   was valid on insertion, it remains valid.
            // - Only values with no mutable handle returned to the user are in
            //   the table.
            let key = unsafe { self.arena.get_unchecked(index).unwrap_unchecked() };
            key == value
        };
        let hasher = |&index: &u32| {
            // SAFETY: Same as above.
            let key = unsafe { self.arena.get_unchecked(index).unwrap_unchecked() };
            self.hash_builder.hash_one(key)
        };
        table.entry(hash, eq, hasher)
    }

    #[inline]
    fn table_find(&self, value: &T) -> Option<u32> {
        let table = unsafe { &mut *self.table.get() };
        let hash = self.hash_builder.hash_one(value);
        let eq = |&index: &u32| {
            // SAFETY: Same as above.
            let key = unsafe { self.arena.get_unchecked(index).unwrap_unchecked() };
            key == value
        };
        table.find(hash, eq).copied()
    }

    #[inline]
    fn table_find_entry(&self, value: &T) -> Option<OccupiedEntry<'_, u32>> {
        let table = unsafe { &mut *self.table.get() };
        let hash = self.hash_builder.hash_one(value);
        let eq = |&index: &u32| {
            // SAFETY: Same as above.
            let key = unsafe { self.arena.get_unchecked(index).unwrap_unchecked() };
            key == value
        };
        table.find_entry(hash, eq).ok()
    }

    #[cfg(debug_assertions)]
    #[inline(always)]
    pub(super) fn assert_id(&self, id: Id<T>) {
        self.arena.assert_id(unsafe { id.transmute() });
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

impl<T> Entry<T> {
    /// Gets a reference to the contained data.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the entry is `Entry::Occupied` and that
    /// no mutable reference to its `RefCell` has been returned to the user.
    #[inline]
    unsafe fn unwrap_unchecked(&self) -> &T {
        match self {
            Entry::Occupied { value, .. } => unsafe { &*value.as_ptr() },
            Entry::Replaced(_) => {
                #[cfg(debug_assertions)]
                unreachable!();
                #[cfg(not(debug_assertions))]
                unsafe {
                    unreachable_unchecked()
                };
            }
        }
    }
}

impl<'a, T: Eq + Hash> ArenaRef<'a, T> {
    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn arena(&self) -> &'a HashArena<T> {
        self.arena
    }

    pub fn get(&self, id: Id<T>) -> ArenaRef<'a, T> {
        self.arena.get(id)
    }

    pub fn get_mut(&self, id: Id<T>) -> ArenaRefMut<'a, T> {
        self.arena.get_mut(id)
    }
}

impl<T: Eq + Hash> Deref for ArenaRef<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.value
    }
}

impl<T: Eq + Hash> PartialEq for ArenaRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        *self.value == *other.value && ptr::eq(self.arena, other.arena)
    }
}

impl<T: Eq + Hash> Eq for ArenaRef<'_, T> {}

impl<'a, T: Eq + Hash> ArenaRefMut<'a, T> {
    pub fn value(&self) -> &T {
        &*self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut *self.value
    }

    pub fn replace(mut self, replacement: Id<T>) {
        let arena = self.arena;
        let index = self.index;
        self.unique = true;
        drop(self);
        let entry = unsafe { arena.arena.get_unchecked_mut(index) };
        *entry = Entry::Replaced(replacement.index());
    }

    pub fn arena(&self) -> &'a HashArena<T> {
        self.arena
    }

    pub fn id(&self) -> Id<T> {
        unsafe { Id::from_index(self.index, &self.arena.arena).transmute() }
    }

    pub fn unique(&mut self) -> bool {
        self.unique
    }

    pub fn set_unique(&mut self, is_unique: bool) {
        self.unique = is_unique;
    }

    pub fn get(&self, id: Id<T>) -> ArenaRef<'a, T> {
        self.arena.get(id)
    }

    pub fn get_mut(&self, id: Id<T>) -> ArenaRefMut<'a, T> {
        self.arena.get_mut(id)
    }
}

impl<T: Eq + Hash> Drop for ArenaRefMut<'_, T> {
    fn drop(&mut self) {
        if !self.unique {
            match self.arena.table_entry(&self.value) {
                TableEntry::Occupied(replaced) => {
                    let entry = unsafe { self.arena.arena.get_unchecked_mut(self.index) };
                    *entry = Entry::Replaced(*replaced.get());
                }
                TableEntry::Vacant(entry) => {
                    entry.insert(self.index);
                }
            }
        }
    }
}

impl<T: Eq + Hash> Deref for ArenaRefMut<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.value
    }
}

impl<T: Eq + Hash> DerefMut for ArenaRefMut<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.value
    }
}

impl<T: Debug + Eq + Hash> Debug for HashArena<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "HashArena {{")?;
        for (i, entry) in self.arena.iter().enumerate() {
            if f.alternate() {
                write!(f, "\n    ")?;
            } else {
                if i != 0 {
                    write!(f, ",")?;
                }
                write!(f, " ")?;
            }
            match entry {
                Entry::Occupied { value, unique } => {
                    if *unique {
                        write!(f, "{i} (unique): ")?;
                    } else {
                        write!(f, "{i}: ")?;
                    }
                    let value = unsafe { &*value.as_ptr() };
                    Debug::fmt(value, f)?;
                }
                Entry::Replaced(replaced) => {
                    write!(f, "{i} (replaced) -> {replaced}")?;
                }
            }
            if f.alternate() {
                write!(f, ",")?;
            }
        }
        if !self.arena.is_empty() {
            if f.alternate() {
                write!(f, "\n")?;
            } else {
                write!(f, " ")?;
            }
        }
        write!(f, "}}")
    }
}

impl<T: Debug + Eq + Hash> Debug for ArenaRef<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.value(), f)
    }
}

impl<T: Debug + Eq + Hash> Debug for ArenaRefMut<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.value(), f)
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::hash_arena::HashArena;

    #[test]
    fn mut_rehash() {
        let arena = HashArena::new();
        let h = arena.insert("hello".to_owned());
        let h_unique = arena.insert_unique("hello".to_owned());
        assert_ne!(h, h_unique);

        let w = arena.insert("world".to_owned());
        assert_eq!(arena.find(&"world".to_owned()), Some(w));
        let w_mut = arena.get_mut(w);
        assert!(arena.find(&"world".to_owned()).is_none());
        drop(w_mut);
        assert_eq!(arena.find(&"world".to_owned()), Some(w));

        let mut w_mut = arena.get_mut(w);
        *w_mut = "hello".to_owned();
        drop(w_mut);
        assert!(arena.find(&"world".to_owned()).is_none());

        assert_eq!(*arena.get(w), "hello".to_owned());
    }
}
