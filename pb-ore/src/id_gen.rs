//! ID generator utilities.

#[derive(Debug)]
pub struct Gen<Id> {
    next: u64,
    phantom: std::marker::PhantomData<fn() -> Id>,
}

impl<Id> Default for Gen<Id> {
    fn default() -> Self {
        Gen::from_start(0)
    }
}

impl<Id> Gen<Id> {
    pub fn from_start(start: u64) -> Self {
        Gen {
            next: start,
            phantom: std::marker::PhantomData::default(),
        }
    }
}

impl<Id: From<u64>> Gen<Id> {
    pub fn next(&mut self) -> Id {
        let id = self.next;
        self.next = id.checked_add(1).expect("ID allocator overflowed u64");
        Id::from(id)
    }
}
