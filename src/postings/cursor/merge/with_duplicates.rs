use std::collections::BinaryHeap;
use types::*;
use postings::{VecPostings,PostingsCursor};
use super::frontier::{FrontierPointer,create_heap};

pub struct MergeUnrolled<C: PostingsCursor> {
    to_merge: Option<Vec<C>>,
}

impl<C: PostingsCursor> MergeUnrolled<C> {
    pub fn new(to_merge: Vec<C>) -> Self {
        MergeUnrolled {
            to_merge: Some(to_merge),
        }
    }

    pub fn collect(&mut self) -> VecPostings {
        let mut frontier = create_heap(self.to_merge.take().unwrap());
        let mut result = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };

        while let Some(cursor) = frontier.pop() {
            let _ = cursor.cursor.catch_up(&mut result);
            if let Some(doc_id) = cursor.cursor.advance() {
                frontier.push(cursor);
            }
        }

        result
    }
}

pub struct Merge<C: PostingsCursor> {
    to_merge: Option<Vec<C>>,
    frontier: BinaryHeap<FrontierPointer<C>>,
    current_cursor: Option<C>,
    size: usize,
}

impl<C: PostingsCursor> Merge<C> {
    pub fn new(to_merge: Vec<C>) -> Self {
        let size = to_merge.iter().map(|c| c.remains()).fold(0, |acc, x| acc + x);
        let mut heap = create_heap(to_merge);
        let first = Some(heap.pop().unwrap().cursor);
        Merge{
            to_merge: Some(to_merge),
            frontier: heap,
            current_cursor: first,
            size: size,
        }
    }
}

impl<C: PostingsCursor> PostingsCursor for Merge<C> {
    type DS = C::DS;
    type TS = C::TS;
    type PS = C::PS;

    fn advance(&mut self) -> Option<DocId> {
        if let Some(cursor) = self.current_cursor {
            if let Some(doc_id) = cursor.cursor.advance() {
                self.frontier.push(cursor);
                self.current_cursor = self.frontier.pop();
                return Some(doc_id);
            }
        }
        None
    }

    fn catch_up(&mut self, result: &mut VecPostings) -> usize {
        if let Some(cursor) = self.current_cursor {
            cursor.catch_up(result)
        } else {
            0
        }
    }

    fn remains(&self) -> usize {
        self.size
    }
}
