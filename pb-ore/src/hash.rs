//! Hashing utilities.

pub struct Xxh3Hasher {
    inner: xxhash_rust::xxh3::Xxh3,
}

impl Xxh3Hasher {
    /// Create a new [`Xxh3Hasher`].
    pub const fn new() -> Self {
        Xxh3Hasher {
            inner: xxhash_rust::xxh3::Xxh3::new(),
        }
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn update(&mut self, input: &[u8]) {
        self.inner.update(input);
    }

    pub fn digest(&self) -> pb_types::Xxh64Hash {
        pb_types::Xxh64Hash::new(self.inner.digest())
    }

    pub fn digest128(&self) -> pb_types::Xxh128Hash {
        pb_types::Xxh128Hash::new(self.inner.digest128())
    }
}
