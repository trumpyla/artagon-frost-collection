//! Crate-owned iterator types. Family types never expose a backend's
//! iterator type in their signatures, so the representation can change
//! without a breaking release.

/// Define a newtype iterator delegating to `$inner`.
///
/// Flags (any order): `double` → `DoubleEndedIterator`, `exact` →
/// `ExactSizeIterator`, `clone` → `Clone`.
///
/// Every generated impl has bounds on the wrapper's own type parameters only,
/// never on the backend iterator type, so no backend type appears in public
/// signatures. `FusedIterator` is unconditional and checked at compile time;
/// `Debug` is opaque (`Name { .. }`), like the iterators' content is not part
/// of their contract.
macro_rules! wrap_iter {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident <$($p:tt),*> ($inner:ty) -> $item:ty
        $(; $($flag:ident),*)?
    ) => {
        $(#[$meta])*
        $vis struct $name<$($p),*>(pub(crate) $inner);

        impl<$($p),*> Iterator for $name<$($p),*> {
            type Item = $item;
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.0.next()
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.0.size_hint()
            }
            #[inline]
            fn fold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B {
                self.0.fold(init, f)
            }
            #[inline]
            fn count(self) -> usize {
                self.0.count()
            }
        }

        impl<$($p),*> core::iter::FusedIterator for $name<$($p),*> {}

        // Compile-time proof that the promise above holds for every
        // instantiation: fails to build if the inner iterator is not fused.
        const _: () = {
            fn _assert_fused<$($p),*>(it: $inner) {
                fn need<I: core::iter::FusedIterator>(_: I) {}
                need(it);
            }
        };

        impl<$($p),*> core::fmt::Debug for $name<$($p),*> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }

        $crate::iter::wrap_iter!(@flags [$($($flag),*)?] $name [$($p),*] ($inner));
    };
    (@flags [$($flag:ident),*] $name:ident $gen:tt ($inner:ty)) => {
        $( $crate::iter::wrap_iter!(@flag $flag, $name, $gen, $inner); )*
    };
    (@flag double, $name:ident, [$($p:tt)*], $inner:ty) => {
        impl<$($p)*> DoubleEndedIterator for $name<$($p)*> {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.0.next_back()
            }
            #[inline]
            fn rfold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B {
                self.0.rfold(init, f)
            }
        }
    };
    (@flag clone, $name:ident, [$($p:tt)*], $inner:ty) => {
        impl<$($p)*> Clone for $name<$($p)*> {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }
    };
    (@flag exact, $name:ident, [$($p:tt)*], $inner:ty) => {
        impl<$($p)*> ExactSizeIterator for $name<$($p)*> {
            #[inline]
            fn len(&self) -> usize {
                self.0.len()
            }
        }
    };
}

pub(crate) use wrap_iter;

/// Keys of a crate-owned entry iterator `I` (`(&K, V)` items).
///
/// Generic over crate-owned iterators only, so it never exposes a backend
/// type; returned by `keys()` on maps whose entry iterator is not a
/// dedicated newtype.
#[derive(Clone)]
pub struct Keys<I>(pub(crate) I);

/// Values of a crate-owned entry iterator `I` (`(K, V)` items, `V` may be
/// `&V` or `&mut V`).
#[derive(Clone)]
pub struct Values<I>(pub(crate) I);

impl<'a, K: 'a, V, I: Iterator<Item = (&'a K, V)>> Iterator for Keys<I> {
    type Item = &'a K;
    #[inline]
    fn next(&mut self) -> Option<&'a K> {
        self.0.next().map(|(k, _)| k)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    #[inline]
    fn fold<B, F: FnMut(B, &'a K) -> B>(self, init: B, mut f: F) -> B {
        self.0.fold(init, |acc, (k, _)| f(acc, k))
    }
}
impl<'a, K: 'a, V, I: DoubleEndedIterator<Item = (&'a K, V)>> DoubleEndedIterator for Keys<I> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a K> {
        self.0.next_back().map(|(k, _)| k)
    }
}
impl<'a, K: 'a, V, I: ExactSizeIterator<Item = (&'a K, V)>> ExactSizeIterator for Keys<I> {}
impl<'a, K: 'a, V, I: core::iter::FusedIterator<Item = (&'a K, V)>> core::iter::FusedIterator
    for Keys<I>
{
}

impl<A, V, I: Iterator<Item = (A, V)>> Iterator for Values<I> {
    type Item = V;
    #[inline]
    fn next(&mut self) -> Option<V> {
        self.0.next().map(|(_, v)| v)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    #[inline]
    fn fold<B, F: FnMut(B, V) -> B>(self, init: B, mut f: F) -> B {
        self.0.fold(init, |acc, (_, v)| f(acc, v))
    }
}
impl<A, V, I: DoubleEndedIterator<Item = (A, V)>> DoubleEndedIterator for Values<I> {
    #[inline]
    fn next_back(&mut self) -> Option<V> {
        self.0.next_back().map(|(_, v)| v)
    }
}
impl<A, V, I: ExactSizeIterator<Item = (A, V)>> ExactSizeIterator for Values<I> {}
impl<A, V, I: core::iter::FusedIterator<Item = (A, V)>> core::iter::FusedIterator for Values<I> {}

impl<I> core::fmt::Debug for Keys<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Keys").finish_non_exhaustive()
    }
}
impl<I> core::fmt::Debug for Values<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Values").finish_non_exhaustive()
    }
}
