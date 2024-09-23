#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    cell::UnsafeCell,
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    hint,
    marker::PhantomData,
    num::NonZero,
    ops::{Index, IndexMut},
};

// TODO:
// - Make the u32 index a trait.

/// An arena of values, identified by ID.
///
/// # Safety
///
/// It is undefined behavior to use an `Id` in any arena other than the one
/// which created it.
pub struct Arena<T> {
    values: UnsafeCell<StableVec<T>>,
    #[cfg(debug_assertions)]
    arena_id: u32,
}

/// A growable vector, which does not move contained elements.
struct StableVec<T> {
    data: Vec<Vec<T>>,
    len: u32,
}

/// The ID for a value in an arena.
pub struct Id<T> {
    /// The 1-based ID of the node, i.e., the index plus 1.
    value_id: NonZero<u32>,
    /// The ID of the arena which contains the node, used for ensuring an `Id`
    /// is not used with a different arena when debug assertions are enabled.
    #[cfg(debug_assertions)]
    arena_id: u32,
    ty: PhantomData<fn() -> T>,
}

impl<T> Arena<T> {
    #[inline]
    pub fn new() -> Self {
        Arena {
            values: UnsafeCell::new(StableVec::new()),
            #[cfg(debug_assertions)]
            arena_id: {
                static ARENA_ID: AtomicU32 = AtomicU32::new(0);
                ARENA_ID.fetch_add(1, Ordering::Relaxed)
            },
        }
    }

    #[inline]
    pub fn push(&self, value: T) -> Id<T> {
        let value_id = self.values().push(value);
        Id {
            value_id,
            #[cfg(debug_assertions)]
            arena_id: self.arena_id,
            ty: PhantomData,
        }
    }

    #[inline(always)]
    pub(super) unsafe fn get_unchecked(&self, index: u32) -> &T {
        self.values().get_unchecked(index)
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(super) unsafe fn get_unchecked_mut(&self, index: u32) -> &mut T {
        self.values().get_unchecked_mut(index)
    }

    /// Returns the number of values in this arena.
    #[inline]
    pub fn len(&self) -> usize {
        self.values().len.try_into().unwrap_or(usize::MAX)
    }

    /// Returns whether this arena contains no values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values().data.iter().flatten()
    }

    fn values(&self) -> &mut StableVec<T> {
        // SAFETY: `Arena<T>` is `!Sync` and values remain at stable addresses
        // once pushed.
        unsafe { &mut *self.values.get() }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: Id<T>) -> &Self::Output {
        let values = self.values();
        #[cfg(debug_assertions)]
        debug_assert!(
            index.arena_id == self.arena_id,
            "index used in different arena",
        );
        debug_assert!(index.value_id.get() <= values.len, "index out of bounds");
        unsafe { values.get_unchecked(index.value_id.get() - 1) }
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, index: Id<T>) -> &mut Self::Output {
        let values = self.values();
        #[cfg(debug_assertions)]
        debug_assert!(
            index.arena_id == self.arena_id,
            "index used in different arena",
        );
        debug_assert!(index.value_id.get() <= values.len, "index out of bounds");
        unsafe { values.get_unchecked_mut(index.value_id.get() - 1) }
    }
}

impl<T: Debug> Debug for Arena<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Arena ")?;
        f.debug_map().entries(self.iter().enumerate()).finish()
    }
}

impl<T> StableVec<T> {
    const CHUNK_SIZE: usize = 1024;

    #[inline]
    fn new() -> Self {
        StableVec {
            data: Vec::new(),
            len: 0,
        }
    }

    #[inline(always)]
    fn push(&mut self, value: T) -> NonZero<u32> {
        if self.len % Self::CHUNK_SIZE as u32 == 0 {
            self.grow();
        }
        let Some(len) = self.len.checked_add(1) else {
            Self::size_overflow();
        };
        self.len = len;
        // SAFETY: Guaranteed by addition.
        let id = unsafe { NonZero::new_unchecked(len) };

        // SAFETY: There is always at least one element, guaranteed by the grow
        // above.
        let chunk = unsafe { self.data.last_mut().unwrap_unchecked() };
        #[cfg(debug_assertions)]
        let chunk_ptr = chunk.as_ptr();
        // Avoids unreachable codegen for growing in `push`.
        // SAFETY: Dynamically assured with `StableVec::len`.
        unsafe { hint::assert_unchecked(chunk.len() < chunk.capacity()) };
        chunk.push(value);
        #[cfg(debug_assertions)]
        debug_assert!(chunk_ptr == chunk.as_ptr(), "push reallocated");
        id
    }

    #[inline(always)]
    unsafe fn get_unchecked(&self, index: u32) -> &T {
        debug_assert!(index < self.len, "index out of bounds");
        // SAFETY: Guaranteed by caller.
        unsafe {
            let chunk = self.data.get_unchecked(index as usize / Self::CHUNK_SIZE);
            chunk.get_unchecked(index as usize % Self::CHUNK_SIZE)
        }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, index: u32) -> &mut T {
        debug_assert!(index < self.len, "index out of bounds");
        // SAFETY: Guaranteed by caller.
        unsafe {
            let chunk = self
                .data
                .get_unchecked_mut(index as usize / Self::CHUNK_SIZE);
            chunk.get_unchecked_mut(index as usize % Self::CHUNK_SIZE)
        }
    }

    #[inline(never)]
    #[cold]
    fn grow(&mut self) {
        self.data.push(Vec::with_capacity(Self::CHUNK_SIZE));
    }

    #[inline(never)]
    #[cold]
    fn size_overflow() -> ! {
        panic!("arena size too large for index");
    }
}

impl<T> Id<T> {
    #[inline]
    pub(super) fn from_index(index: u32, _arena: &Arena<T>) -> Self {
        Id {
            value_id: unsafe { NonZero::new_unchecked(index + 1) },
            #[cfg(debug_assertions)]
            arena_id: _arena.arena_id,
            ty: PhantomData,
        }
    }

    /// Returns the 0-based index of this ID.
    #[inline]
    pub fn index(self) -> u32 {
        self.value_id.get() - 1
    }

    /// Sets the type of the identified value.
    #[inline]
    pub(super) unsafe fn transmute<U>(self) -> Id<U> {
        Id {
            value_id: self.value_id,
            #[cfg(debug_assertions)]
            arena_id: self.arena_id,
            ty: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    #[inline]
    fn clone(&self) -> Self {
        Id {
            value_id: self.value_id,
            #[cfg(debug_assertions)]
            arena_id: self.arena_id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[cfg(debug_assertions)]
        if f.alternate() {
            return f
                .debug_struct("Id")
                .field("index", &self.index())
                .field("arena_id", &self.arena_id)
                .finish();
        }
        f.debug_tuple("Id").field(&self.index()).finish()
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.value_id != other.value_id {
            return false;
        }
        #[cfg(debug_assertions)]
        if self.arena_id != other.arena_id {
            return false;
        }
        true
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value_id.hash(state);
        #[cfg(debug_assertions)]
        self.arena_id.hash(state);
    }
}
