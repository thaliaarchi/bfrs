//! An arena of values, identified by ID.

use std::{
    cell::{Cell, UnsafeCell},
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    iter,
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZero,
    ops::{Index, IndexMut},
};

// TODO:
// - Drop, Debug, and Iterator

/// An arena of values, identified by ID.
pub struct Arena<T> {
    chunks: UnsafeCell<Vec<*mut MaybeUninit<T>>>,
    len: Cell<u32>,
    marker: PhantomData<Vec<Vec<T>>>,
}

/// The ID for a value in an arena.
pub struct Id<T> {
    /// The 1-based ID of the node, i.e., the index plus 1.
    id: NonZero<u32>,
    marker: PhantomData<fn() -> T>,
}

impl<T> Arena<T> {
    const CHUNK_SIZE: usize = 1024;

    /// Constructs a new, empty arena.
    #[inline]
    pub fn new() -> Self {
        Arena {
            chunks: UnsafeCell::new(Vec::new()),
            len: Cell::new(0),
            marker: PhantomData::default(),
        }
    }

    /// Inserts a value into the arena.
    #[inline(always)]
    pub fn insert(&self, value: T) -> Id<T> {
        let len = self.len.get();
        if len as usize % Self::CHUNK_SIZE == 0 {
            self.grow();
        }
        let Some(new_len) = len.checked_add(1) else {
            Self::size_overflow();
        };
        self.len.set(new_len);

        unsafe {
            let chunks = &mut *self.chunks.get();
            let chunk = *chunks.last().unwrap_unchecked();
            chunk
                .add(len as usize % Self::CHUNK_SIZE)
                .write(MaybeUninit::new(value));
        }

        Id {
            // SAFETY: Guaranteed non-zero by checked add.
            id: unsafe { NonZero::new_unchecked(new_len) },
            marker: PhantomData,
        }
    }

    /// Gets a value without checking that the index is in bounds.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        #[cfg(debug_assertions)]
        if index >= self.len() {
            Self::out_of_bounds();
        }
        unsafe {
            let chunk = (*self.chunks.get()).get_unchecked(index / Self::CHUNK_SIZE);
            (&*chunk.add(index % Self::CHUNK_SIZE)).assume_init_ref()
        }
    }

    /// Gets a mutable reference to a value without checking that the index is
    /// in bounds.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        #[cfg(debug_assertions)]
        if index >= self.len() {
            Self::out_of_bounds();
        }
        unsafe {
            let chunk = (*self.chunks.get()).get_unchecked_mut(index / Self::CHUNK_SIZE);
            (&mut *chunk.add(index % Self::CHUNK_SIZE)).assume_init_mut()
        }
    }

    /// Returns the number of values in this arena.
    #[inline]
    pub fn len(&self) -> usize {
        self.len.get() as usize
    }

    /// Returns whether this arena contains no values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[cold]
    #[inline(never)]
    fn grow(&self) {
        let chunks = unsafe { &mut *self.chunks.get() };
        // TODO: Use `Box::new_uninit_slice(CHUNK_SIZE)`, once Rust 1.82 is
        // stable.
        let chunk = iter::repeat_with(|| MaybeUninit::<T>::uninit())
            .take(Self::CHUNK_SIZE)
            .collect::<Box<[_]>>();
        chunks.push(Box::leak(chunk).as_mut_ptr());
    }

    #[cold]
    #[inline(never)]
    fn out_of_bounds() -> ! {
        panic!("index out of bounds");
    }

    #[cold]
    #[inline(never)]
    fn size_overflow() -> ! {
        panic!("arena has too many values for u32 index");
    }
}

impl<T> Index<usize> for Arena<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.len() {
            Self::out_of_bounds();
        }
        unsafe { self.get_unchecked(index) }
    }
}

impl<T> IndexMut<usize> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.len() {
            Self::out_of_bounds();
        }
        unsafe { self.get_unchecked_mut(index) }
    }
}

impl<T> Id<T> {
    /// Returns the 0-based index of this ID.
    #[inline]
    pub fn index(self) -> usize {
        self.id.get() as usize
    }
}

impl<T> Clone for Id<T> {
    #[inline]
    fn clone(&self) -> Self {
        Id {
            id: self.id,
            marker: PhantomData,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArenaId").field(&self.index()).finish()
    }
}

impl<T> PartialEq for Id<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
