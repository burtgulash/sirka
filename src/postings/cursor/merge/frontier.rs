use std::cmp::Ordering;
use std::iter::FromIterator; // needed for ::from_iter
use std::collections::BinaryHeap;
use postings::PostingsCursor;

pub struct FrontierPointer<C: PostingsCursor> {
    pub cursor: C,
}

impl<C: PostingsCursor> Ord for FrontierPointer<C> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Switch compare order because Rust's BinaryHeap is a maxheap We want a minheap
        unsafe {self.cursor.current().cmp(&other.cursor.current()).reverse()}
    }
}

impl<C: PostingsCursor> PartialOrd for FrontierPointer<C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<C: PostingsCursor> PartialEq for FrontierPointer<C> {
    fn eq(&self, other: &Self) -> bool {
        unsafe {self.cursor.current() == other.cursor.current()}
    }
}

impl<C: PostingsCursor> Eq for FrontierPointer<C> {}


pub fn create_heap<C: PostingsCursor>(to_merge: Vec<C>) -> BinaryHeap<FrontierPointer<C>> {
    BinaryHeap::from_iter(to_merge.into_iter().map(|mut cur| {
        let _ = cur.advance().unwrap();
        FrontierPointer {
            cursor: cur
        }
    }))
}
