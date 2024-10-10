//! An arena of values, identified by ID.

use std::{
    cell::{Cell, UnsafeCell},
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    iter::{self, FusedIterator},
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZero,
    ops::{Index, IndexMut, Range},
    ptr,
};

// TODO:
// - Drop

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

/// An iterator over values in an arena.
pub struct Iter<'a, T> {
    arena: &'a Arena<T>,
    index: Range<u32>,
}

/// An iterator over values and their IDs in an arena.
pub struct IdIter<'a, T> {
    arena: &'a Arena<T>,
    index: Range<u32>,
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
            // SAFETY: Guaranteed non-zero by checked add.
            Id::from_id(new_len)
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

    /// Returns an iterator over values in this arena.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            arena: self,
            index: 0..self.len.get(),
        }
    }

    /// Returns an iterator over values and their IDs in this arena.
    #[inline]
    pub fn iter_ids(&self) -> IdIter<'_, T> {
        IdIter {
            arena: self,
            index: 0..self.len.get(),
        }
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

impl<T: Debug> Debug for Arena<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Arena ")?;
        f.debug_map().entries(self.iter().enumerate()).finish()
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

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> Id<T> {
    /// Constructs an `Id` from its 1-based ID.
    ///
    /// # Safety
    ///
    /// The ID must not be 0 and be in bounds for the arena.
    #[inline]
    unsafe fn from_id(id: u32) -> Self {
        Id {
            id: unsafe { NonZero::new_unchecked(id) },
            marker: PhantomData,
        }
    }

    /// Constructs an `Id` from its 0-based index.
    ///
    /// # Safety
    ///
    /// The index must not be `u32::MAX` and be in bounds for the arena.
    #[inline]
    unsafe fn from_index(index: u32) -> Self {
        Id::from_id(index + 1)
    }

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

macro_rules! impl_iter(($Iter:ident<$a:lifetime, $T:ident> yields $Item:ty) => {
    impl<$a, $T> Iterator for $Iter<$a, $T> {
        type Item = $Item;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.get(|index| index.next())
        }

        #[inline]
        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            self.get(|index| index.nth(n))
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.index.size_hint()
        }
    }

    impl<$T> DoubleEndedIterator for $Iter<'_, $T> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.get(|index| index.next_back())
        }

        #[inline]
        fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
            self.get(|index| index.nth_back(n))
        }
    }

    impl<$T> ExactSizeIterator for $Iter<'_, $T> {
        #[inline]
        fn len(&self) -> usize {
            self.index.len()
        }
    }

    impl<$T> FusedIterator for $Iter<'_, $T> {}

    impl<$T> Clone for $Iter<'_, $T> {
        #[inline]
        fn clone(&self) -> Self {
            $Iter {
                arena: self.arena,
                index: self.index.clone(),
            }
        }
    }

    impl<$T> Debug for $Iter<'_, $T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.debug_tuple(stringify!($Iter)).field(&self.index).finish()
        }
    }

    impl<$T> PartialEq for $Iter<'_, $T> {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            ptr::eq(self.arena, other.arena) && self.index == other.index
        }
    }

    impl<$T> Eq for $Iter<'_, $T> {}
});

impl<'a, T> Iter<'a, T> {
    #[inline]
    fn get(&mut self, get_index: impl FnOnce(&mut Range<u32>) -> Option<u32>) -> Option<&'a T> {
        let index = get_index(&mut self.index)?;
        unsafe { Some(self.arena.get_unchecked(index as usize)) }
    }
}

impl_iter!(Iter<'a, T> yields &'a T);

impl<'a, T> IdIter<'a, T> {
    #[inline]
    fn get(
        &mut self,
        get_index: impl FnOnce(&mut Range<u32>) -> Option<u32>,
    ) -> Option<(Id<T>, &'a T)> {
        let index = get_index(&mut self.index)?;
        unsafe {
            Some((
                Id::from_index(index),
                self.arena.get_unchecked(index as usize),
            ))
        }
    }
}

impl_iter!(IdIter<'a, T> yields (Id<T>, &'a T));
