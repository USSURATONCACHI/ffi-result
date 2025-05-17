use std::{
    fmt::Debug,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

/// FFI-compatibe and ABI-stable analogue for [`core::result::Result`].
///
/// Can be freely converted to and from the core Result.
#[repr(C)]
pub struct Result<T, E> {
    kind: ResultKind,
    data: ResultData<T, E>,
}

impl<T, E> Result<T, E> {
    pub const fn new_ok(t: T) -> Self {
        Self {
            kind: ResultKind::Ok,
            data: ResultData {
                ok: ManuallyDrop::new(t),
            },
        }
    }
    pub const fn new_err(e: E) -> Self {
        Self {
            kind: ResultKind::Err,
            data: ResultData {
                err: ManuallyDrop::new(e),
            },
        }
    }

    pub fn is_ok(&self) -> bool {
        self.kind == ResultKind::Ok
    }
    pub fn is_err(&self) -> bool {
        self.kind == ResultKind::Err
    }

    pub const fn kind(&self) -> &ResultKind {
        &self.kind
    }

    /// # Safety
    /// Cannot guarantee that the user will preserve correct kind-data relationship.
    pub const unsafe fn kind_mut(&mut self) -> &mut ResultKind {
        &mut self.kind
    }

    pub const fn data(&self) -> &ResultData<T, E> {
        &self.data
    }

    /// # Safety
    /// Cannot guarantee that the user will preserve correct kind-data relationship.
    pub const unsafe fn data_mut(&mut self) -> &mut ResultData<T, E> {
        &mut self.data
    }

    pub fn as_ref(&self) -> Result<&T, &E> {
        Result {
            kind: self.kind,
            data: match self.kind {
                ResultKind::Ok => {
                    let inner = unsafe { &self.data.ok };
                    ResultData {
                        ok: ManuallyDrop::new(inner.deref()),
                    }
                }
                ResultKind::Err => {
                    let inner = unsafe { &self.data.err };
                    ResultData {
                        err: ManuallyDrop::new(inner.deref()),
                    }
                }
            },
        }
    }

    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        Result {
            kind: self.kind,
            data: match self.kind {
                ResultKind::Ok => {
                    let inner = unsafe { &mut self.data.ok };
                    ResultData {
                        ok: ManuallyDrop::new(inner.deref_mut()),
                    }
                }
                ResultKind::Err => {
                    let inner = unsafe { &mut self.data.err };
                    ResultData {
                        err: ManuallyDrop::new(inner.deref_mut()),
                    }
                }
            },
        }
    }

    pub fn ok(self) -> Option<T> {
        let kind = self.kind;
        // SAFETY: we only read the union bits, which are valid for either variant
        let mut data = unsafe { std::ptr::read(&self.data) };
        std::mem::forget(self);

        match kind {
            ResultKind::Ok => Some(unsafe { ManuallyDrop::into_inner(data.ok) }),
            ResultKind::Err => {
                unsafe { ManuallyDrop::drop(&mut data.err) };
                None
            }
        }
    }
    pub fn err(self) -> Option<E> {
        let kind = self.kind;
        // SAFETY: we only read the union bits, which are valid for either variant
        let mut data = unsafe { std::ptr::read(&self.data) };
        std::mem::forget(self);

        match kind {
            ResultKind::Err => Some(unsafe { ManuallyDrop::into_inner(data.err) }),
            ResultKind::Ok => {
                unsafe { ManuallyDrop::drop(&mut data.ok) };
                None
            }
        }
    }
    pub fn into_result(self) -> core::result::Result<T, E> {
        let kind = self.kind;
        // SAFETY: we only read the union bits, which are valid for either variant
        let data = unsafe { std::ptr::read(&self.data) };
        std::mem::forget(self);

        match kind {
            ResultKind::Ok => {
                core::result::Result::Ok(unsafe { ManuallyDrop::into_inner(data.ok) })
            }
            ResultKind::Err => {
                core::result::Result::Err(unsafe { ManuallyDrop::into_inner(data.err) })
            }
        }
    }
    pub fn from_result(result: core::result::Result<T, E>) -> Self {
        match result {
            Ok(ok) => Self::new_ok(ok),
            Err(err) => Self::new_err(err),
        }
    }

    pub fn map<T2>(self, op: impl FnOnce(T) -> T2) -> Result<T2, E> {
        self.into_result().map(op).into()
    }
    pub fn map_err<E2>(self, op: impl FnOnce(E) -> E2) -> Result<T, E2> {
        self.into_result().map_err(op).into()
    }
}
impl<T: Debug, E> Result<T, E> {
    pub fn unwrap_err(self) -> E {
        self.into_result().unwrap_err()
    }
    pub fn expect_err(self, msg: &str) -> E {
        self.into_result().expect_err(msg)
    }
}
impl<T, E: Debug> Result<T, E> {
    pub fn unwrap(self) -> T {
        self.into_result().unwrap()
    }
    pub fn expect(self, msg: &str) -> T {
        self.into_result().expect(msg)
    }
}

impl<T, E> From<Result<T, E>> for core::result::Result<T, E> {
    fn from(val: Result<T, E>) -> core::result::Result<T, E> {
        val.into_result()
    }
}
impl<T, E> From<core::result::Result<T, E>> for Result<T, E> {
    fn from(val: core::result::Result<T, E>) -> Self {
        Self::from_result(val)
    }
}

impl<T: Debug, E: Debug> Debug for Result<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut structt = f.debug_struct("Result");
        let dbg = structt.field("kind", &self.kind);

        unsafe {
            match self.kind {
                ResultKind::Ok => dbg.field("data", self.data.ok.deref()).finish(),
                ResultKind::Err => dbg.field("data", self.data.err.deref()).finish(),
            }
        }
    }
}

impl<T: Clone, E: Clone> Clone for Result<T, E> {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                kind: self.kind,
                data: match self.kind {
                    ResultKind::Ok => ResultData {
                        ok: self.data.ok.clone(),
                    },
                    ResultKind::Err => ResultData {
                        err: self.data.err.clone(),
                    },
                },
            }
        }
    }
}

impl<T: PartialEq, E: PartialEq> PartialEq for Result<T, E> {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_ref().into_result(), other.as_ref().into_result()) {
            (Ok(a), Ok(b)) => a.eq(b),
            (Err(a), Err(b)) => a.eq(b),
            (Ok(_), Err(_)) => false,
            (Err(_), Ok(_)) => false,
        }
    }
}

impl<T, E> Drop for Result<T, E> {
    fn drop(&mut self) {
        unsafe {
            match self.kind {
                ResultKind::Ok => {
                    ManuallyDrop::drop(&mut self.data.ok);
                }
                ResultKind::Err => {
                    ManuallyDrop::drop(&mut self.data.err);
                }
            }
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResultKind {
    Ok,
    Err,
}

#[repr(C)]
pub union ResultData<T, E> {
    pub ok: ManuallyDrop<T>,
    pub err: ManuallyDrop<E>,
}
