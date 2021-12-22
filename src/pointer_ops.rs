// Copyright 2020 Amari Robinson
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#[cfg(feature = "alloc")]
use crate::alloc::boxed::Box;
#[cfg(feature = "alloc")]
use crate::alloc::rc::Rc;
#[cfg(feature = "alloc")]
use crate::alloc::sync::Arc;
use crate::{UnsafeMut, UnsafeRef};
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::Deref;
use core::pin::Pin;

/// Trait for pointer conversion operations.
///
/// `Value` is the actual object type managed by the collection. This type will
/// typically have a link as a struct field.
///
/// `Pointer` is a pointer type which "owns" an object of type `Value`.
/// Operations which insert an element into an intrusive collection will accept
/// such a pointer and operations which remove an element will return this type.
pub unsafe trait PointerOps {
    /// Object type which is inserted into an intrusive collection.
    type Value: ?Sized;
    /// Pointer type which owns an instance of a value.
    type Pointer;

    /// Constructs an owned pointer from a raw pointer.
    ///
    /// # Safety
    /// The raw pointer must have been previously returned by `into_raw`.
    ///
    /// An implementation of `from_raw` must not panic.
    unsafe fn from_raw(&self, value: *const Self::Value) -> Self::Pointer;

    /// Consumes the owned pointer and returns a raw pointer to the owned object.
    fn into_raw(&self, ptr: Self::Pointer) -> *const Self::Value;
}

/// Trait for conditional exclusive access to the object managed by the collection.
pub unsafe trait TryExclusivePointerOps: PointerOps {
    /// Try to get unique access to the value.
    /// Returns Some if the value is uniquely owned by the collection. None if not.
    unsafe fn try_get_mut(&self, value: *const Self::Value) -> Option<*mut Self::Value>;
}

/// Trait for exclusive access to the object managed by the collection.
///
/// Implementing this type automatically implements [TryExclusivePointerOps]
/// 
/// The default implementation of [get_mut](Self::get_mut) simply casts `*const Self::Value` to `*mut Self::Value`
pub unsafe trait ExclusivePointerOps: TryExclusivePointerOps {
    /// Get unique access to the value
    /// This function always succeeds, if you want to conditionally allow exclusive access, implement [TryExclusivePointerOps] instead.
    #[inline]
    unsafe fn get_mut(&self, value: *const Self::Value) -> *mut Self::Value {
        value as *mut Self::Value
    }
}
unsafe impl<E: ExclusivePointerOps> TryExclusivePointerOps for E {
    unsafe fn try_get_mut(&self, value: *const Self::Value) -> Option<*mut Self::Value> {
        Some(self.get_mut(value))
    }
}

/// The `PointerOps` type used by an `Adapter` generated by `intrusive_adapter!`.
pub struct DefaultPointerOps<Pointer>(PhantomData<Pointer>);

impl<Pointer> DefaultPointerOps<Pointer> {
    /// Constructs an instance of `DefaultPointerOps`.
    #[inline]
    pub const fn new() -> DefaultPointerOps<Pointer> {
        DefaultPointerOps(PhantomData)
    }
}

impl<Pointer> Clone for DefaultPointerOps<Pointer> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<Pointer> Copy for DefaultPointerOps<Pointer> {}

impl<Pointer> Default for DefaultPointerOps<Pointer> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<'a, T: ?Sized> PointerOps for DefaultPointerOps<&'a T> {
    type Value = T;
    type Pointer = &'a T;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> &'a T {
        &*raw
    }

    #[inline]
    fn into_raw(&self, ptr: &'a T) -> *const T {
        ptr
    }
}

unsafe impl<'a, T: ?Sized> PointerOps for DefaultPointerOps<Pin<&'a T>> {
    type Value = T;
    type Pointer = Pin<&'a T>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Pin<&'a T> {
        Pin::new_unchecked(&*raw)
    }

    #[inline]
    fn into_raw(&self, ptr: Pin<&'a T>) -> *const T {
        unsafe { Pin::into_inner_unchecked(ptr) as *const T }
    }
}

unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<UnsafeRef<T>> {
    type Value = T;
    type Pointer = UnsafeRef<T>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> UnsafeRef<T> {
        UnsafeRef::from_raw(raw as *mut T)
    }

    #[inline]
    fn into_raw(&self, ptr: UnsafeRef<T>) -> *const T {
        UnsafeRef::into_raw(ptr) as *const T
    }
}

unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<Pin<UnsafeRef<T>>> {
    type Value = T;
    type Pointer = Pin<UnsafeRef<T>>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Pin<UnsafeRef<T>> {
        Pin::new_unchecked(UnsafeRef::from_raw(raw as *mut T))
    }

    #[inline]
    fn into_raw(&self, ptr: Pin<UnsafeRef<T>>) -> *const T {
        UnsafeRef::into_raw(unsafe { Pin::into_inner_unchecked(ptr) }) as *const T
    }
}

unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<UnsafeMut<T>> {
    type Value = T;
    type Pointer = UnsafeMut<T>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> UnsafeMut<T> {
        UnsafeMut::from_raw(raw as *mut T)
    }

    #[inline]
    fn into_raw(&self, ptr: UnsafeMut<T>) -> *const T {
        UnsafeMut::into_raw(ptr) as *const T
    }
}

unsafe impl<T: ?Sized> ExclusivePointerOps for DefaultPointerOps<UnsafeMut<T>> {}

unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<Pin<UnsafeMut<T>>> {
    type Value = T;
    type Pointer = Pin<UnsafeMut<T>>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Pin<UnsafeMut<T>> {
        Pin::new_unchecked(UnsafeMut::from_raw(raw as *mut T))
    }

    #[inline]
    fn into_raw(&self, ptr: Pin<UnsafeMut<T>>) -> *const T {
        UnsafeMut::into_raw(unsafe { Pin::into_inner_unchecked(ptr) }) as *const T
    }
}

unsafe impl<T: ?Sized> ExclusivePointerOps for DefaultPointerOps<Pin<UnsafeMut<T>>> {}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<Box<T>> {
    type Value = T;
    type Pointer = Box<T>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Box<T> {
        Box::from_raw(raw as *mut T)
    }

    #[inline]
    fn into_raw(&self, ptr: Box<T>) -> *const T {
        Box::into_raw(ptr) as *const T
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> ExclusivePointerOps for DefaultPointerOps<Box<T>> {}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<Pin<Box<T>>> {
    type Value = T;
    type Pointer = Pin<Box<T>>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Pin<Box<T>> {
        Pin::new_unchecked(Box::from_raw(raw as *mut T))
    }

    #[inline]
    fn into_raw(&self, ptr: Pin<Box<T>>) -> *const T {
        Box::into_raw(unsafe { Pin::into_inner_unchecked(ptr) }) as *const T
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> ExclusivePointerOps for DefaultPointerOps<Pin<Box<T>>> {}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<Rc<T>> {
    type Value = T;
    type Pointer = Rc<T>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Rc<T> {
        Rc::from_raw(raw)
    }

    #[inline]
    fn into_raw(&self, ptr: Rc<T>) -> *const T {
        Rc::into_raw(ptr)
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> TryExclusivePointerOps for DefaultPointerOps<Rc<T>> {
    #[inline]
    unsafe fn try_get_mut(&self, value: *const Self::Value) -> Option<*mut Self::Value> {
        let mut rc = Rc::from_raw(value);
        let ptr = Rc::get_mut(&mut rc).map(|ptr| ptr as *mut Self::Value);
        drop(Rc::into_raw(rc));
        ptr
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<Pin<Rc<T>>> {
    type Value = T;
    type Pointer = Pin<Rc<T>>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Pin<Rc<T>> {
        Pin::new_unchecked(Rc::from_raw(raw))
    }

    #[inline]
    fn into_raw(&self, ptr: Pin<Rc<T>>) -> *const T {
        Rc::into_raw(unsafe { Pin::into_inner_unchecked(ptr) })
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> TryExclusivePointerOps for DefaultPointerOps<Pin<Rc<T>>> {
    #[inline]
    unsafe fn try_get_mut(&self, value: *const Self::Value) -> Option<*mut Self::Value> {
        let mut rc = Rc::from_raw(value);
        let ptr = Rc::get_mut(&mut rc).map(|ptr| ptr as *mut Self::Value);
        drop(Rc::into_raw(rc));
        ptr
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<Arc<T>> {
    type Value = T;
    type Pointer = Arc<T>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Arc<T> {
        Arc::from_raw(raw)
    }

    #[inline]
    fn into_raw(&self, ptr: Arc<T>) -> *const T {
        Arc::into_raw(ptr)
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> TryExclusivePointerOps for DefaultPointerOps<Arc<T>> {
    #[inline]
    unsafe fn try_get_mut(&self, value: *const Self::Value) -> Option<*mut Self::Value> {
        let mut arc = Arc::from_raw(value);
        let ptr = Arc::get_mut(&mut arc).map(|ptr| ptr as *mut Self::Value);
        drop(Arc::into_raw(arc));
        ptr
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> PointerOps for DefaultPointerOps<Pin<Arc<T>>> {
    type Value = T;
    type Pointer = Pin<Arc<T>>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const T) -> Pin<Arc<T>> {
        Pin::new_unchecked(Arc::from_raw(raw))
    }

    #[inline]
    fn into_raw(&self, ptr: Pin<Arc<T>>) -> *const T {
        Arc::into_raw(unsafe { Pin::into_inner_unchecked(ptr) })
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> TryExclusivePointerOps for DefaultPointerOps<Pin<Arc<T>>> {
    #[inline]
    unsafe fn try_get_mut(&self, value: *const Self::Value) -> Option<*mut Self::Value> {
        let mut arc = Arc::from_raw(value);
        let ptr = Arc::get_mut(&mut arc).map(|ptr| ptr as *mut Self::Value);
        drop(Arc::into_raw(arc));
        ptr
    }
}

/// Clones a `PointerOps::Pointer` from a `*const PointerOps::Value`
///
/// This method is only safe to call if the raw pointer is known to be
/// managed by the provided `PointerOps` type.
#[inline]
pub(crate) unsafe fn clone_pointer_from_raw<T: PointerOps>(
    pointer_ops: &T,
    ptr: *const T::Value,
) -> T::Pointer
where
    T::Pointer: Clone,
{
    /// Guard which converts an pointer back into its raw version
    /// when it gets dropped. This makes sure we also perform a full
    /// `from_raw` and `into_raw` round trip - even in the case of panics.
    struct PointerGuard<'a, T: PointerOps> {
        pointer: ManuallyDrop<T::Pointer>,
        pointer_ops: &'a T,
    }

    impl<'a, T: PointerOps> Drop for PointerGuard<'a, T> {
        #[inline]
        fn drop(&mut self) {
            // Prevent shared pointers from being released by converting them
            // back into the raw pointers
            // SAFETY: `pointer` is never dropped. `ManuallyDrop::take` is not stable until 1.42.0.
            let _ = self
                .pointer_ops
                .into_raw(unsafe { core::ptr::read(&*self.pointer) });
        }
    }

    let holder = PointerGuard {
        pointer: ManuallyDrop::new(pointer_ops.from_raw(ptr)),
        pointer_ops,
    };
    holder.pointer.deref().clone()
}

#[cfg(test)]
mod tests {
    use super::{DefaultPointerOps, PointerOps};
    use std::boxed::Box;
    use std::fmt::Debug;
    use std::mem;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::sync::Arc;

    #[test]
    fn test_box() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Box<_>>::new();
            let p = Box::new(1);
            let a: *const i32 = &*p;
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            let p2: Box<i32> = pointer_ops.from_raw(r);
            let a2: *const i32 = &*p2;
            assert_eq!(a, a2);
        }
    }

    #[test]
    fn test_rc() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Rc<_>>::new();
            let p = Rc::new(1);
            let a: *const i32 = &*p;
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            let p2: Rc<i32> = pointer_ops.from_raw(r);
            let a2: *const i32 = &*p2;
            assert_eq!(a, a2);
        }
    }

    #[test]
    fn test_arc() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Arc<_>>::new();
            let p = Arc::new(1);
            let a: *const i32 = &*p;
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            let p2: Arc<i32> = pointer_ops.from_raw(r);
            let a2: *const i32 = &*p2;
            assert_eq!(a, a2);
        }
    }

    #[test]
    fn test_box_unsized() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Box<_>>::new();
            let p = Box::new(1) as Box<dyn Debug>;
            let a: *const dyn Debug = &*p;
            let b: (usize, usize) = mem::transmute(a);
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            assert_eq!(b, mem::transmute(r));
            let p2: Box<dyn Debug> = pointer_ops.from_raw(r);
            let a2: *const dyn Debug = &*p2;
            assert_eq!(a, a2);
            assert_eq!(b, mem::transmute(a2));
        }
    }

    #[test]
    fn test_rc_unsized() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Rc<_>>::new();
            let p = Rc::new(1) as Rc<dyn Debug>;
            let a: *const dyn Debug = &*p;
            let b: (usize, usize) = mem::transmute(a);
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            assert_eq!(b, mem::transmute(r));
            let p2: Rc<dyn Debug> = pointer_ops.from_raw(r);
            let a2: *const dyn Debug = &*p2;
            assert_eq!(a, a2);
            assert_eq!(b, mem::transmute(a2));
        }
    }

    #[test]
    fn test_arc_unsized() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Arc<_>>::new();
            let p = Arc::new(1) as Arc<dyn Debug>;
            let a: *const dyn Debug = &*p;
            let b: (usize, usize) = mem::transmute(a);
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            assert_eq!(b, mem::transmute(r));
            let p2: Arc<dyn Debug> = pointer_ops.from_raw(r);
            let a2: *const dyn Debug = &*p2;
            assert_eq!(a, a2);
            assert_eq!(b, mem::transmute(a2));
        }
    }

    #[test]
    fn clone_arc_from_raw() {
        use super::clone_pointer_from_raw;
        unsafe {
            let pointer_ops = DefaultPointerOps::<Arc<_>>::new();
            let p = Arc::new(1);
            let raw = &*p as *const i32;
            let p2: Arc<i32> = clone_pointer_from_raw(&pointer_ops, raw);
            assert_eq!(2, Arc::strong_count(&p2));
        }
    }

    #[test]
    fn clone_rc_from_raw() {
        use super::clone_pointer_from_raw;
        unsafe {
            let pointer_ops = DefaultPointerOps::<Rc<_>>::new();
            let p = Rc::new(1);
            let raw = &*p as *const i32;
            let p2: Rc<i32> = clone_pointer_from_raw(&pointer_ops, raw);
            assert_eq!(2, Rc::strong_count(&p2));
        }
    }

    #[test]
    fn test_pin_box() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Pin<Box<_>>>::new();
            let p = Pin::new(Box::new(1));
            let a: *const i32 = &*p;
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            let p2: Pin<Box<i32>> = pointer_ops.from_raw(r);
            let a2: *const i32 = &*p2;
            assert_eq!(a, a2);
        }
    }

    #[test]
    fn test_pin_rc() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Pin<Rc<_>>>::new();
            let p = Pin::new(Rc::new(1));
            let a: *const i32 = &*p;
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            let p2: Pin<Rc<i32>> = pointer_ops.from_raw(r);
            let a2: *const i32 = &*p2;
            assert_eq!(a, a2);
        }
    }

    #[test]
    fn test_pin_arc() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Pin<Arc<_>>>::new();
            let p = Pin::new(Arc::new(1));
            let a: *const i32 = &*p;
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            let p2: Pin<Arc<i32>> = pointer_ops.from_raw(r);
            let a2: *const i32 = &*p2;
            assert_eq!(a, a2);
        }
    }

    #[test]
    fn test_pin_box_unsized() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Pin<Box<_>>>::new();
            let p = Pin::new(Box::new(1)) as Pin<Box<dyn Debug>>;
            let a: *const dyn Debug = &*p;
            let b: (usize, usize) = mem::transmute(a);
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            assert_eq!(b, mem::transmute(r));
            let p2: Pin<Box<dyn Debug>> = pointer_ops.from_raw(r);
            let a2: *const dyn Debug = &*p2;
            assert_eq!(a, a2);
            assert_eq!(b, mem::transmute(a2));
        }
    }

    #[test]
    fn test_pin_rc_unsized() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Pin<Rc<_>>>::new();
            let p = Pin::new(Rc::new(1)) as Pin<Rc<dyn Debug>>;
            let a: *const dyn Debug = &*p;
            let b: (usize, usize) = mem::transmute(a);
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            assert_eq!(b, mem::transmute(r));
            let p2: Pin<Rc<dyn Debug>> = pointer_ops.from_raw(r);
            let a2: *const dyn Debug = &*p2;
            assert_eq!(a, a2);
            assert_eq!(b, mem::transmute(a2));
        }
    }

    #[test]
    fn test_pin_arc_unsized() {
        unsafe {
            let pointer_ops = DefaultPointerOps::<Pin<Arc<_>>>::new();
            let p = Pin::new(Arc::new(1)) as Pin<Arc<dyn Debug>>;
            let a: *const dyn Debug = &*p;
            let b: (usize, usize) = mem::transmute(a);
            let r = pointer_ops.into_raw(p);
            assert_eq!(a, r);
            assert_eq!(b, mem::transmute(r));
            let p2: Pin<Arc<dyn Debug>> = pointer_ops.from_raw(r);
            let a2: *const dyn Debug = &*p2;
            assert_eq!(a, a2);
            assert_eq!(b, mem::transmute(a2));
        }
    }

    #[test]
    fn clone_pin_arc_from_raw() {
        use super::clone_pointer_from_raw;
        unsafe {
            let pointer_ops = DefaultPointerOps::<Pin<Arc<_>>>::new();
            let p = Pin::new(Arc::new(1));
            let raw = &*p as *const i32;
            let p2: Pin<Arc<i32>> = clone_pointer_from_raw(&pointer_ops, raw);
            assert_eq!(2, Arc::strong_count(&Pin::into_inner(p2)));
        }
    }

    #[test]
    fn clone_pin_rc_from_raw() {
        use super::clone_pointer_from_raw;
        unsafe {
            let pointer_ops = DefaultPointerOps::<Pin<Rc<_>>>::new();
            let p = Pin::new(Rc::new(1));
            let raw = &*p as *const i32;
            let p2: Pin<Rc<i32>> = clone_pointer_from_raw(&pointer_ops, raw);
            assert_eq!(2, Rc::strong_count(&Pin::into_inner(p2)));
        }
    }
}
