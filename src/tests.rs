use crate::SmallVec;
use alloc::{borrow::ToOwned, boxed::Box, rc::Rc, vec, vec::Vec};
use core::iter::FromIterator;

macro_rules! create_smallvec {
    (let $var_name:ident: SmallVec($data_ty:ty, $data_value:expr) = $smallvec:expr) => {
        #[cfg(feature = "const_generics")]
        let $var_name: SmallVec<$data_ty, $data_value> = $smallvec;
        #[cfg(not(feature = "const_generics"))]
        let $var_name: SmallVec<[$data_ty; $data_value]> = $smallvec;
    };
    (let mut $var_name:ident: SmallVec($data_ty:ty, $data_value:expr) = $smallvec:expr) => {
        #[cfg(feature = "const_generics")]
        let mut $var_name: SmallVec<$data_ty, $data_value> = $smallvec;
        #[cfg(not(feature = "const_generics"))]
        let mut $var_name: SmallVec<[$data_ty; $data_value]> = $smallvec;
    };
}

#[test]
pub fn test_zero() {
    create_smallvec!(let mut v: SmallVec(_, 0) = SmallVec::new());
    assert!(!v.spilled());
    v.push(0usize);
    assert!(v.spilled());
    assert_eq!(&*v, &[0]);
}

// We heap allocate all these strings so that double frees will show up under valgrind.

#[test]
pub fn test_inline() {
    create_smallvec!(let mut v: SmallVec(_, 16) = SmallVec::new());
    v.push("hello".to_owned());
    v.push("there".to_owned());
    assert_eq!(&*v, &["hello".to_owned(), "there".to_owned()][..]);
}

#[test]
pub fn test_spill() {
    create_smallvec!(let mut v: SmallVec(_, 2) = SmallVec::new());
    v.push("hello".to_owned());
    assert_eq!(v[0], "hello");
    v.push("there".to_owned());
    v.push("burma".to_owned());
    assert_eq!(v[0], "hello");
    v.push("shave".to_owned());
    assert_eq!(
        &*v,
        &[
            "hello".to_owned(),
            "there".to_owned(),
            "burma".to_owned(),
            "shave".to_owned()
        ][..]
    );
}

#[test]
pub fn test_double_spill() {
    create_smallvec!(let mut v: SmallVec(_, 2) = SmallVec::new());
    v.push("hello".to_owned());
    v.push("there".to_owned());
    v.push("burma".to_owned());
    v.push("shave".to_owned());
    v.push("hello".to_owned());
    v.push("there".to_owned());
    v.push("burma".to_owned());
    v.push("shave".to_owned());
    assert_eq!(
        &*v,
        &[
            "hello".to_owned(),
            "there".to_owned(),
            "burma".to_owned(),
            "shave".to_owned(),
            "hello".to_owned(),
            "there".to_owned(),
            "burma".to_owned(),
            "shave".to_owned()
        ][..]
    );
}

/// https://github.com/servo/rust-smallvec/issues/4
#[test]
fn issue_4() {
    create_smallvec!(let _v: SmallVec(Box<u32>, 2) = SmallVec::new());
}

/// https://github.com/servo/rust-smallvec/issues/5
#[test]
fn issue_5() {
    create_smallvec!(let v: SmallVec(&u32, 2) = SmallVec::new());
    assert!(Some(v).is_some());
}

#[test]
fn test_with_capacity() {
    create_smallvec!(let v: SmallVec(u8, 3) = SmallVec::with_capacity(1));
    assert!(v.is_empty());
    assert!(!v.spilled());
    assert_eq!(v.capacity(), 3);

    create_smallvec!(let v: SmallVec(u8, 3) = SmallVec::with_capacity(10));
    assert!(v.is_empty());
    assert!(v.spilled());
    assert_eq!(v.capacity(), 10);
}

#[test]
fn drain() {
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.push(3);
    assert_eq!(v.drain().collect::<Vec<_>>(), &[3]);

    // spilling the vec
    v.push(3);
    v.push(4);
    v.push(5);
    assert_eq!(v.drain().collect::<Vec<_>>(), &[3, 4, 5]);
}

#[test]
fn drain_rev() {
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.push(3);
    assert_eq!(v.drain().rev().collect::<Vec<_>>(), &[3]);

    // spilling the vec
    v.push(3);
    v.push(4);
    v.push(5);
    assert_eq!(v.drain().rev().collect::<Vec<_>>(), &[5, 4, 3]);
}

#[test]
fn into_iter() {
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.push(3);
    assert_eq!(v.into_iter().collect::<Vec<_>>(), &[3]);

    // spilling the vec
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.push(3);
    v.push(4);
    v.push(5);
    assert_eq!(v.into_iter().collect::<Vec<_>>(), &[3, 4, 5]);
}

#[test]
fn into_iter_rev() {
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.push(3);
    assert_eq!(v.into_iter().rev().collect::<Vec<_>>(), &[3]);

    // spilling the vec
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.push(3);
    v.push(4);
    v.push(5);
    assert_eq!(v.into_iter().rev().collect::<Vec<_>>(), &[5, 4, 3]);
}

#[test]
fn into_iter_drop() {
    use core::cell::Cell;

    struct DropCounter<'a>(&'a Cell<i32>);

    impl<'a> Drop for DropCounter<'a> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    {
        let cell = Cell::new(0);
        create_smallvec!(let mut v: SmallVec(DropCounter, 2) = SmallVec::new());
        v.push(DropCounter(&cell));
        v.into_iter();
        assert_eq!(cell.get(), 1);
    }

    {
        let cell = Cell::new(0);
        create_smallvec!(let mut v: SmallVec(DropCounter, 2) = SmallVec::new());
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        assert!(v.into_iter().next().is_some());
        assert_eq!(cell.get(), 2);
    }

    {
        let cell = Cell::new(0);
        create_smallvec!(let mut v: SmallVec(DropCounter, 2) = SmallVec::new());
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        assert!(v.into_iter().next().is_some());
        assert_eq!(cell.get(), 3);
    }
    {
        let cell = Cell::new(0);
        create_smallvec!(let mut v: SmallVec(DropCounter, 2) = SmallVec::new());
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        {
            let mut it = v.into_iter();
            assert!(it.next().is_some());
            assert!(it.next_back().is_some());
        }
        assert_eq!(cell.get(), 3);
    }
}

#[test]
fn test_capacity() {
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.reserve(1);
    assert_eq!(v.capacity(), 2);
    assert!(!v.spilled());

    v.reserve_exact(0x100);
    assert!(v.capacity() >= 0x100);

    v.push(0);
    v.push(1);
    v.push(2);
    v.push(3);

    v.shrink_to_fit();
    assert!(v.capacity() < 0x100);
}

#[test]
fn test_truncate() {
    create_smallvec!(let mut v: SmallVec(Box<u8>, 8) = SmallVec::new());

    for x in 0..8 {
        v.push(Box::new(x));
    }
    v.truncate(4);

    assert_eq!(v.len(), 4);
    assert!(!v.spilled());

    assert_eq!(*v.swap_remove(1), 1);
    assert_eq!(*v.remove(1), 3);
    v.insert(1, Box::new(3));

    assert_eq!(&v.iter().map(|v| **v).collect::<Vec<_>>(), &[0, 3, 2]);
}

#[test]
fn test_insert_many() {
    create_smallvec!(let mut v: SmallVec(u8, 8) = SmallVec::new());
    for x in 0..4 {
        v.push(x);
    }
    assert_eq!(v.len(), 4);
    v.insert_many(1, [5, 6].iter().cloned());
    assert_eq!(
        &v.iter().map(|v| *v).collect::<Vec<_>>(),
        &[0, 5, 6, 1, 2, 3]
    );
}

struct MockHintIter<T: Iterator> {
    x: T,
    hint: usize,
}
impl<T: Iterator> Iterator for MockHintIter<T> {
    type Item = T::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.x.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.hint, None)
    }
}

#[test]
fn test_insert_many_short_hint() {
    create_smallvec!(let mut v: SmallVec(u8, 8) = SmallVec::new());
    for x in 0..4 {
        v.push(x);
    }
    assert_eq!(v.len(), 4);
    v.insert_many(
        1,
        MockHintIter {
            x: [5, 6].iter().cloned(),
            hint: 5,
        },
    );
    assert_eq!(
        &v.iter().map(|v| *v).collect::<Vec<_>>(),
        &[0, 5, 6, 1, 2, 3]
    );
}

#[test]
fn test_insert_many_long_hint() {
    create_smallvec!(let mut v: SmallVec(u8, 8) = SmallVec::new());
    for x in 0..4 {
        v.push(x);
    }
    assert_eq!(v.len(), 4);
    v.insert_many(
        1,
        MockHintIter {
            x: [5, 6].iter().cloned(),
            hint: 1,
        },
    );
    assert_eq!(
        &v.iter().map(|v| *v).collect::<Vec<_>>(),
        &[0, 5, 6, 1, 2, 3]
    );
}

#[cfg(feature = "std")]
#[test]
// https://github.com/servo/rust-smallvec/issues/96
fn test_insert_many_panic() {
    struct PanicOnDoubleDrop {
        dropped: Box<bool>,
    }

    impl Drop for PanicOnDoubleDrop {
        fn drop(&mut self) {
            assert!(!*self.dropped, "already dropped");
            *self.dropped = true;
        }
    }

    struct BadIter;
    impl Iterator for BadIter {
        type Item = PanicOnDoubleDrop;
        fn size_hint(&self) -> (usize, Option<usize>) {
            (1, None)
        }
        fn next(&mut self) -> Option<Self::Item> {
            panic!()
        }
    }

    create_smallvec!(let mut v: SmallVec(PanicOnDoubleDrop, 0) = vec![
        PanicOnDoubleDrop {
            dropped: Box::new(false),
        },
        PanicOnDoubleDrop {
            dropped: Box::new(false),
        },
    ].into());
    let result = std::panic::catch_unwind(move || {
        v.insert_many(0, BadIter);
    });
    assert!(result.is_err());
}

#[test]
#[should_panic]
fn test_invalid_grow() {
    create_smallvec!(let mut v: SmallVec(u8, 8) = SmallVec::new());
    v.extend(0..8);
    v.grow(5);
}

#[test]
fn test_insert_from_slice() {
    create_smallvec!(let mut v: SmallVec(u8, 8) = SmallVec::new());
    for x in 0..4 {
        v.push(x);
    }
    assert_eq!(v.len(), 4);
    v.insert_from_slice(1, &[5, 6]);
    assert_eq!(
        &v.iter().map(|v| *v).collect::<Vec<_>>(),
        &[0, 5, 6, 1, 2, 3]
    );
}

#[test]
fn test_extend_from_slice() {
    create_smallvec!(let mut v: SmallVec(u8, 8) = SmallVec::new());
    for x in 0..4 {
        v.push(x);
    }
    assert_eq!(v.len(), 4);
    v.extend_from_slice(&[5, 6]);
    assert_eq!(
        &v.iter().map(|v| *v).collect::<Vec<_>>(),
        &[0, 1, 2, 3, 5, 6]
    );
}

#[test]
#[should_panic]
fn test_drop_panic_smallvec() {
    // This test should only panic once, and not double panic,
    // which would mean a double drop
    struct DropPanic;

    impl Drop for DropPanic {
        fn drop(&mut self) {
            panic!("drop");
        }
    }

    create_smallvec!(let mut v: SmallVec(_, 1) = SmallVec::new());
    v.push(DropPanic);
}

#[test]
fn test_eq() {
    create_smallvec!(let mut a: SmallVec(u32, 2) = SmallVec::new());
    create_smallvec!(let mut b: SmallVec(u32, 2) = SmallVec::new());
    create_smallvec!(let mut c: SmallVec(u32, 2) = SmallVec::new());
    // a = [1, 2]
    a.push(1);
    a.push(2);
    // b = [1, 2]
    b.push(1);
    b.push(2);
    // c = [3, 4]
    c.push(3);
    c.push(4);

    assert!(a == b);
    assert!(a != c);
}

#[test]
fn test_ord() {
    create_smallvec!(let mut a: SmallVec(u32, 2) = SmallVec::new());
    create_smallvec!(let mut b: SmallVec(u32, 2) = SmallVec::new());
    create_smallvec!(let mut c: SmallVec(u32, 2) = SmallVec::new());
    // a = [1]
    a.push(1);
    // b = [1, 1]
    b.push(1);
    b.push(1);
    // c = [1, 2]
    c.push(1);
    c.push(2);

    assert!(a < b);
    assert!(b > a);
    assert!(b < c);
    assert!(c > b);
}

#[cfg(feature = "std")]
#[test]
fn test_hash() {
    use std::{collections::hash_map::DefaultHasher, hash::Hash};

    {
        create_smallvec!(let mut a: SmallVec(u32, 2) = SmallVec::new());
        let b = [1, 2];
        a.extend(b.iter().cloned());
        let mut hasher = DefaultHasher::new();
        assert_eq!(a.hash(&mut hasher), b.hash(&mut hasher));
    }
    {
        create_smallvec!(let mut a: SmallVec(u32, 2) = SmallVec::new());
        let b = [1, 2, 11, 12];
        a.extend(b.iter().cloned());
        let mut hasher = DefaultHasher::new();
        assert_eq!(a.hash(&mut hasher), b.hash(&mut hasher));
    }
}

#[test]
fn test_as_ref() {
    create_smallvec!(let mut a: SmallVec(u32, 2) = SmallVec::new());
    a.push(1);
    assert_eq!(a.as_ref(), [1]);
    a.push(2);
    assert_eq!(a.as_ref(), [1, 2]);
    a.push(3);
    assert_eq!(a.as_ref(), [1, 2, 3]);
}

#[test]
fn test_as_mut() {
    create_smallvec!(let mut a: SmallVec(u32, 2) = SmallVec::new());
    a.push(1);
    assert_eq!(a.as_mut(), [1]);
    a.push(2);
    assert_eq!(a.as_mut(), [1, 2]);
    a.push(3);
    assert_eq!(a.as_mut(), [1, 2, 3]);
    a.as_mut()[1] = 4;
    assert_eq!(a.as_mut(), [1, 4, 3]);
}

#[test]
fn test_borrow() {
    use core::borrow::Borrow;

    create_smallvec!(let mut a: SmallVec(u32, 2) = SmallVec::new());
    a.push(1);
    assert_eq!(a.borrow(), [1]);
    a.push(2);
    assert_eq!(a.borrow(), [1, 2]);
    a.push(3);
    assert_eq!(a.borrow(), [1, 2, 3]);
}

#[test]
fn test_borrow_mut() {
    use core::borrow::BorrowMut;

    create_smallvec!(let mut a: SmallVec(u32, 2) = SmallVec::new());
    a.push(1);
    assert_eq!(a.borrow_mut(), [1]);
    a.push(2);
    assert_eq!(a.borrow_mut(), [1, 2]);
    a.push(3);
    assert_eq!(a.borrow_mut(), [1, 2, 3]);
    BorrowMut::<[u32]>::borrow_mut(&mut a)[1] = 4;
    assert_eq!(a.borrow_mut(), [1, 4, 3]);
}

#[test]
fn test_from() {
    create_smallvec!(let a: SmallVec(u32, 2) = SmallVec::from_slice(&[1][..]));
    create_smallvec!(let b: SmallVec(u32, 2) = SmallVec::from_slice(&[1, 2, 3][..]));
    assert_eq!(&a[..], [1]);
    assert_eq!(&b[..], [1, 2, 3]);

    let vec = vec![];
    create_smallvec!(let v: SmallVec(u8, 3) = SmallVec::from(vec));
    assert_eq!(&*v, &[]);
    drop(v);

    let vec = vec![1, 2, 3, 4, 5];
    create_smallvec!(let v: SmallVec(u8, 3) = SmallVec::from(vec));
    assert_eq!(&*v, &[1, 2, 3, 4, 5]);
    drop(v);

    let vec = vec![1, 2, 3, 4, 5];
    create_smallvec!(let v: SmallVec(u8, 1) = SmallVec::from(vec));
    assert_eq!(&*v, &[1, 2, 3, 4, 5]);
    drop(v);

    let array = [1];
    create_smallvec!(let v: SmallVec(u8, 1) = SmallVec::from(array));
    assert_eq!(&*v, &[1]);
    drop(v);

    let array = [99; 128];
    create_smallvec!(let v: SmallVec(u8, 128) = SmallVec::from(array));
    assert_eq!(&*v, vec![99u8; 128].as_slice());
    drop(v);
}

#[test]
fn test_from_slice() {
    create_smallvec!(let a: SmallVec(u32, 2) = SmallVec::from_slice(&[1][..]));
    create_smallvec!(let b: SmallVec(u32, 2) = SmallVec::from_slice(&[1, 2, 3][..]));

    assert_eq!(&a[..], [1]);
    assert_eq!(&b[..], [1, 2, 3]);
}

#[test]
fn test_exact_size_iterator() {
    create_smallvec!(let mut v: SmallVec(u32, 2) = SmallVec::from(&[1, 2, 3][..]));
    assert_eq!(v.clone().into_iter().len(), 3);
    assert_eq!(v.drain().len(), 3);
}

#[test]
fn shrink_to_fit_unspill() {
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::from_iter(0..3));
    v.pop();
    assert!(v.spilled());
    v.shrink_to_fit();
    assert!(!v.spilled(), "shrink_to_fit will un-spill if possible");
}

#[test]
fn test_into_vec() {
    create_smallvec!(let v: SmallVec(u8, 2) = SmallVec::from_iter(0..2));
    assert_eq!(v.into_vec(), vec![0, 1]);

    create_smallvec!(let v: SmallVec(u8, 2) = SmallVec::from_iter(0..3));
    assert_eq!(v.into_vec(), vec![0, 1, 2]);
}

#[test]
fn test_into_inner() {
    create_smallvec!(let v: SmallVec(u8, 2) = SmallVec::from_iter(0..2));
    assert_eq!(v.into_inner(), Ok([0, 1]));

    create_smallvec!(let v: SmallVec(u8, 2) = SmallVec::from_iter(0..1));
    assert_eq!(v.clone().into_inner(), Err(v));

    create_smallvec!(let v: SmallVec(u8, 2) = SmallVec::from_iter(0..3));
    assert_eq!(v.clone().into_inner(), Err(v));
}

#[test]
fn test_from_vec() {
    let vec = vec![];
    create_smallvec!(let v: SmallVec(u8, 3) = SmallVec::from_vec(vec));
    assert_eq!(&*v, &[]);
    drop(v);

    let vec = vec![];
    create_smallvec!(let v: SmallVec(u8, 1) = SmallVec::from_vec(vec));
    assert_eq!(&*v, &[]);
    drop(v);

    let vec = vec![1];
    create_smallvec!(let v: SmallVec(u8, 3) = SmallVec::from_vec(vec));
    assert_eq!(&*v, &[1]);
    drop(v);

    let vec = vec![1, 2, 3];
    create_smallvec!(let v: SmallVec(u8, 3) = SmallVec::from_vec(vec));
    assert_eq!(&*v, &[1, 2, 3]);
    drop(v);

    let vec = vec![1, 2, 3, 4, 5];
    create_smallvec!(let v: SmallVec(u8, 3) = SmallVec::from_vec(vec));
    assert_eq!(&*v, &[1, 2, 3, 4, 5]);
    drop(v);

    let vec = vec![1, 2, 3, 4, 5];
    create_smallvec!(let v: SmallVec(u8, 1) = SmallVec::from_vec(vec));
    assert_eq!(&*v, &[1, 2, 3, 4, 5]);
    drop(v);
}

#[test]
fn test_retain() {
    // Test inline data storate
    create_smallvec!(let mut v: SmallVec(i32, 5) = SmallVec::from_slice(&[1, 2, 3, 3, 4]));
    v.retain(|&mut i| i != 3);
    assert_eq!(v.pop(), Some(4));
    assert_eq!(v.pop(), Some(2));
    assert_eq!(v.pop(), Some(1));
    assert_eq!(v.pop(), None);

    // Test spilled data storage
    create_smallvec!(let mut v: SmallVec(i32, 3) = SmallVec::from_slice(&[1, 2, 3, 3, 4]));
    v.retain(|&mut i| i != 3);
    assert_eq!(v.pop(), Some(4));
    assert_eq!(v.pop(), Some(2));
    assert_eq!(v.pop(), Some(1));
    assert_eq!(v.pop(), None);

    // Test that drop implementations are called for inline.
    let one = Rc::new(1);
    create_smallvec!(let mut v: SmallVec(Rc<i32>, 3) = SmallVec::new());
    v.push(Rc::clone(&one));
    assert_eq!(Rc::strong_count(&one), 2);
    v.retain(|_| false);
    assert_eq!(Rc::strong_count(&one), 1);

    // Test that drop implementations are called for spilled data.
    create_smallvec!(let mut v: SmallVec(Rc<i32>, 1) = SmallVec::new());
    v.push(Rc::clone(&one));
    v.push(Rc::new(2));
    assert_eq!(Rc::strong_count(&one), 2);
    v.retain(|_| false);
    assert_eq!(Rc::strong_count(&one), 1);
}

#[test]
fn test_dedup() {
    create_smallvec!(let mut dupes: SmallVec(i32, 5) = SmallVec::from_slice(&[1, 1, 2, 3, 3]));
    dupes.dedup();
    assert_eq!(&*dupes, &[1, 2, 3]);

    create_smallvec!(let mut empty: SmallVec(i32, 5) = SmallVec::new());
    empty.dedup();
    assert!(empty.is_empty());

    create_smallvec!(let mut all_ones: SmallVec(i32, 5) = SmallVec::from_slice(&[1, 1, 1, 1, 1]));
    all_ones.dedup();
    assert_eq!(all_ones.len(), 1);

    create_smallvec!(let mut no_dupes: SmallVec(i32, 5) = SmallVec::from_slice(&[1, 2, 3, 4, 5]));
    no_dupes.dedup();
    assert_eq!(no_dupes.len(), 5);
}

#[test]
fn test_resize() {
    create_smallvec!(let mut v: SmallVec(i32, 8) = SmallVec::new());
    v.push(1);
    v.resize(5, 0);
    assert_eq!(v[..], [1, 0, 0, 0, 0][..]);

    v.resize(2, -1);
    assert_eq!(v[..], [1, 0][..]);
}

#[cfg(feature = "std")]
#[test]
fn test_write() {
    use std::io::Write;

    let data = [1, 2, 3, 4, 5];

    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    let len = v.write(&data[..]).unwrap();
    assert_eq!(len, 5);
    assert_eq!(v.as_ref(), data.as_ref());

    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.write_all(&data[..]).unwrap();
    assert_eq!(v.as_ref(), data.as_ref());
}

#[cfg(feature = "serde")]
extern crate bincode;

#[cfg(feature = "serde")]
#[test]
fn test_serde() {
    use self::bincode::{config, deserialize};
    create_smallvec!(let mut v: SmallVec(i32, 2) = SmallVec::new());
    v.push(1);
    let encoded = config().limit(100).serialize(&v).unwrap();
    create_smallvec!(let decoded: SmallVec(i32, 2) = deserialize(&encoded).unwrap());
    assert_eq!(v, decoded);
    v.push(2);
    // Spill the vec
    v.push(3);
    v.push(4);
    // Check again after spilling.
    let encoded = config().limit(100).serialize(&v).unwrap();

    create_smallvec!(let decoded: SmallVec(i32, 2) = deserialize(&encoded).unwrap());
    assert_eq!(v, decoded);
}

#[test]
fn grow_to_shrink() {
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.push(1);
    v.push(2);
    v.push(3);
    assert!(v.spilled());
    v.clear();
    // Shrink to inline.
    v.grow(2);
    assert!(!v.spilled());
    assert_eq!(v.capacity(), 2);
    assert_eq!(v.len(), 0);
    v.push(4);
    assert_eq!(v[..], [4]);
}

#[test]
fn resumable_extend() {
    let s = "a b c";
    // This iterator yields: (Some('a'), None, Some('b'), None, Some('c')), None
    let it = s
        .chars()
        .scan(0, |_, ch| if ch.is_whitespace() { None } else { Some(ch) });
    create_smallvec!(let mut v: SmallVec(char, 4) = SmallVec::new());
    v.extend(it);
    assert_eq!(v[..], ['a']);
}

#[test]
fn grow_spilled_same_size() {
    create_smallvec!(let mut v: SmallVec(u8, 2) = SmallVec::new());
    v.push(0);
    v.push(1);
    v.push(2);
    assert!(v.spilled());
    assert_eq!(v.capacity(), 4);
    // grow with the same capacity
    v.grow(4);
    assert_eq!(v.capacity(), 4);
    assert_eq!(v[..], [0, 1, 2]);
}
