use types::*;
use std::collections::BinaryHeap;
use std::cmp::min;
use std::cmp::{Ordering};


#[derive(Clone)]
struct MergeEncoder<S> {
    frontier: BinaryHeap<FrontierPointer<S>>,
    size: usize,
}

impl<S: Sequence> MergeEncoder<S> {
    fn new(sequences_to_merge: &[S]) -> Self {
        assert!(sequences_to_merge.len() > 0);

        let mut heap = BinaryHeap::new();
        let mut size = 0;

        for s in sequences_to_merge.iter() {
            let mut sequence = s.clone();
            size += sequence.remains();
            if let Some(first_doc_id) = sequence.next() {
                heap.push(FrontierPointer {
                    sequence: sequence,
                    current: first_doc_id,
                });
            }
        }
        assert!(heap.len() > 0);

        MergeEncoder {
            frontier: heap,
            size: size,
        }
    }
}

impl<S: Sequence> Sequence for MergeEncoder<S> {
    fn next(&mut self) -> Option<DocId> {
        // This can happen because subsequences do override this sequence's size
        if self.size == 0 {
            return None;
        }
        if let Some(mut ptr) = self.frontier.pop() {
            let current_doc = ptr.current;
            if let Some(doc_id) = ptr.sequence.next() {
                ptr.current = doc_id;
                self.frontier.push(ptr);
            }
            self.size -= 1;
            Some(current_doc)
        } else {
            None
        }
    }

    fn remains(&self) -> usize {
        self.size
    }

    fn subsequence(&self, start: usize, len: usize) -> Self {
        let mut sub = self.clone();
        for _ in 0..start {
            sub.next();
        }
        sub.size = min(sub.size, len);
        sub
    }
}


#[derive(Clone)]
struct FrontierPointer<S> {
    sequence: S,
    current: DocId,
}

impl<S> Ord for FrontierPointer<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Switch compare order because Rust's BinaryHeap is a maxheap We want a minheap
        self.current.cmp(&other.current).reverse()
    }
}

impl<S> PartialOrd for FrontierPointer<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> PartialEq for FrontierPointer<S> {
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
    }
}

impl<S> Eq for FrontierPointer<S> {}
