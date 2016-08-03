use types::*;
use std::cmp::Ordering;
use std::iter::FromIterator; // needed for ::from_iter
use std::collections::BinaryHeap;
use postings::PostingsCursor;

pub struct FrontierPointer<C: PostingsCursor> {
    pub current: DocId,
    pub cursor: C,
}

impl<C: PostingsCursor> Ord for FrontierPointer<C> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Switch compare order because Rust's BinaryHeap is a maxheap We want a minheap
        self.current.cmp(&other.current).reverse()
    }
}

impl<C: PostingsCursor> PartialOrd for FrontierPointer<C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<C: PostingsCursor> PartialEq for FrontierPointer<C> {
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
    }
}

impl<C: PostingsCursor> Eq for FrontierPointer<C> {}


pub fn create_heap<C: PostingsCursor>(to_merge: Vec<C>) -> BinaryHeap<FrontierPointer<C>> {
    BinaryHeap::from_iter(to_merge.into_iter().map(|mut cur| {
        let current = cur.advance().unwrap();
        FrontierPointer {
            current: current,
            cursor: cur,
        }
    }))
}
