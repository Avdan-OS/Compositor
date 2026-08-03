use std::ops::Deref;

pub struct Guard<T: Copy, F: FnOnce(T)>(Option<(T, F)>);

impl<T: Copy, F: FnOnce(T)> Guard<T, F> {
    pub const fn new(value: T, cleanup: F) -> Self {
        Self(Some((value, cleanup)))
    }

    pub fn finish(mut self) -> T {
        self.0.take().unwrap().0
    }
}

impl<T: Copy, F: FnOnce(T)> Deref for Guard<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.as_ref().unwrap().0
    }
}

impl<T: Copy, F: FnOnce(T)> Drop for Guard<T, F> {
    fn drop(&mut self) {
        if let Some((t, f)) = self.0.take() {
            f(t)
        }
    }
}
