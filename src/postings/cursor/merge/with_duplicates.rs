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

        while let Some(mut cursor) = frontier.pop() {
            let _ = cursor.cursor.catch_up(&mut result);
            if let Some(_) = cursor.cursor.advance() {
                frontier.push(cursor);
            }
        }

        result
    }
}

pub struct Merge<C: PostingsCursor> {
    frontier: BinaryHeap<FrontierPointer<C>>,
    current_ptr: Option<FrontierPointer<C>>,
    size: usize,
}

impl<C: PostingsCursor> Merge<C> {
    pub fn new(to_merge: Vec<C>) -> Self {
        let size = to_merge.iter().map(|c| c.remains()).fold(0, |acc, x| acc + x);
        let heap = create_heap(to_merge);
        Merge{
            current_ptr: None,
            frontier: heap,
            size: size,
        }
    }
}

impl<C: PostingsCursor> PostingsCursor for Merge<C> {
    type DS = C::DS;
    type TS = C::TS;
    type PS = C::PS;


    fn remains(&self) -> usize {
        self.size
    }

    unsafe fn current(&self) -> DocId {
        self.current_ptr.as_ref().unwrap().cursor.current()
    }

    fn catch_up(&mut self, result: &mut VecPostings) -> usize {
        self.current_ptr.as_mut().unwrap().cursor.catch_up(result)
    }

    fn advance_to(&mut self, doc_id: DocId) -> Option<DocId> {
        let current_doc = unsafe {self.current()};
        if current_doc >= doc_id {
            Some(current_doc)
        } else {
            let mut ptrs = self.frontier.drain().collect::<Vec<_>>();
            self.frontier.extend(ptrs.into_iter()
                                     .map(|mut ptr| {
                                         ptr.cursor.advance();
                                         ptr
                                     }));
            self.advance()
        }
    }

    fn advance(&mut self) -> Option<DocId> {
        if let Some(mut ptr) = self.current_ptr.take() {
            if let Some(_) = ptr.cursor.advance() {
                self.frontier.push(ptr);
            }
        } 
        
        self.current_ptr = self.frontier.pop();
        if let Some(ref ptr) = self.current_ptr {
            unsafe {Some(ptr.cursor.current())}
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postings::{RawCursor,VecPostings,Postings,PostingsCursor,SequenceStorage};

    #[test]
    fn test_merge_with_duplicates() {
        let ps1 = VecPostings {
            docs: vec![1, 2],
            tfs: vec![0, 1, 2], // NOTE: tfs must already be cumulated!
            positions: vec![1, 2],
        };
        let ps2 = VecPostings {
            docs: vec![2, 3],
            tfs: vec![0, 2, 5], // NOTE: tfs must already be cumulated!
            positions: vec![1, 2, 1, 2, 3],
        };

        let ps1c = RawCursor::new(Postings {
            docs: (&ps1.docs).to_sequence(),
            tfs: (&ps1.tfs).to_sequence(),
            positions: (&ps1.positions).to_sequence(),
        });
        let ps2c = RawCursor::new(Postings {
            docs: (&ps2.docs).to_sequence(),
            tfs: (&ps2.tfs).to_sequence(),
            positions: (&ps2.positions).to_sequence(),
        });

        let mut merger = Merge::new(vec![ps1c, ps2c]);
        let merged = merger.collect();

        assert_eq!(merged.docs, vec![1, 2, 2, 3]);
        assert_eq!(merged.tfs, vec![1, 2, 1, 3]);
        assert_eq!(merged.positions, vec![1, 1, 2, 2, 1, 2, 3]);
    }
}
