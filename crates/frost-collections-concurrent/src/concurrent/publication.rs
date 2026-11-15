//! Retry policy shared by snapshot publication and the bounded scheduler model.
//!
//! The caller supplies the atomic publication primitive. A successful commit
//! returns the displaced handle; a failed commit returns the rejected value and
//! the handle observed by that attempt. This module does not implement atomic
//! pointer publication or memory reclamation.

use core::ops::Deref;

pub(crate) fn update<F, H>(
    mut current: H,
    mut derive: impl FnMut(&F) -> F,
    mut publish: impl FnMut(&H, F) -> Result<H, (F, H)>,
) -> H
where
    H: Deref<Target = F>,
{
    loop {
        let next = derive(&current);
        match publish(&current, next) {
            Ok(previous) => return previous,
            Err((_, observed)) => current = observed,
        }
    }
}
