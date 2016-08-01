use std::marker::PhantomData;
use std::cmp::Ordering;
use std::mem;
use std::iter::FromIterator; // needed for ::from_iter
use std::collections::BinaryHeap;
use postings::{Postings,VecPostings,Sequence,SequenceStorage,PostingsCursor,SimpleCursor};
use postings::slice::SliceSequence;
use types::*;

impl<A: Sequence, B: Sequence, C: Sequence> Ord for SimpleCursor<A, B, C> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Switch compare order because Rust's BinaryHeap is a maxheap We want a minheap
        self.current().unwrap().cmp(&other.current().unwrap()).reverse()
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PartialOrd for SimpleCursor<A, B, C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PartialEq for SimpleCursor<A, B, C> {
    fn eq(&self, other: &Self) -> bool {
        self.current().unwrap() == other.current().unwrap()
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> Eq for SimpleCursor<A, B, C> { }


fn keep_unique<T: Copy + PartialEq>(xs: &[T]) -> Vec<T> {
    let mut res = Vec::new();
    if xs.len() > 0 {
        let mut group_elem = xs[0];
        for x in xs[1..].into_iter().cloned() {
            if x != group_elem {
                res.push(group_elem);
                group_elem = x;
            }
        }
        res.push(group_elem);
    }
    res
}

fn create_heap<A: Sequence, B: Sequence, C: Sequence>(to_merge: &[Postings<A, B, C>]) -> BinaryHeap<SimpleCursor<A, B, C>> {
    BinaryHeap::from_iter(to_merge.iter().map(|pp| {
        let mut p = pp.clone();
        assert_eq!(p.docs.remains(), p.tfs.remains() - 1);
        assert!(p.docs.remains() > 0);

        let mut cur = SimpleCursor::new(p, 0, 0, 0);
        cur.advance();
        cur
    }))
}

// struct MergerWithDuplicates<A, B, C> {
//     frontier: BinaryHeap<SimpleCursor<A, B, C>>,
// }
// impl<A: Sequence, B: Sequence, C: Sequence> for MergerWithDuplicates<A, B, C> {
//     pub fn new(to_merge: &[Postings<A, B, C>]) -> Self {
//         MergerWithDuplicates {
//             frontier: create_heap(to_merge)
//         }
//     }
// }


struct MergerWithoutDuplicates<A, B, C> {
    frontier: BinaryHeap<SimpleCursor<A, B, C>>,
    next_cursor: Option<SimpleCursor<A, B, C>>,
    current_doc: DocId,
    current_positions: Vec<DocId>,
    current_tf: DocId,
    size: usize,
    processed: usize,
}

impl<A: Sequence, B: Sequence, C: Sequence> MergerWithoutDuplicates<A, B, C> {
    pub fn new(to_merge: &[Postings<A, B, C>]) -> Self {
        let size = to_merge.iter().map(|p| p.docs.remains()).fold(0, |acc, x| acc + x);

        let mut heap = create_heap(to_merge);
        let mut first_cursor = heap.pop();
        let first_doc = first_cursor.as_ref().unwrap().current().unwrap();

        MergerWithoutDuplicates {
            frontier: heap,
            next_cursor: first_cursor,
            current_doc: first_doc,
            current_positions: Vec::new(),
            current_tf: 1137,
            size: size,
            processed: 1,
        }
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PostingsCursor<A, B, C> for MergerWithoutDuplicates<A, B, C> {
    fn advance(&mut self) -> Option<DocId> {
        if self.next_cursor.is_none() {
            return None;
        }

        let mut current_cursor = self.next_cursor.take().unwrap();

        let mut positions_buffer = Vec::new();
        let current_doc = self.current_doc;

        let mut finished = false;
        loop {
            let (tf, positions) = current_cursor.catch_up();
            positions_buffer.extend_from_slice(&positions[..]);
            if let Some(_) = current_cursor.current() {
                self.frontier.push(current_cursor);
            }

            if let Some(mut next_cursor) = self.frontier.pop() {
                self.processed += 1;
                if let Some(next_doc) = next_cursor.advance() {
                    if next_doc == current_doc {
                        current_cursor = next_cursor;
                    } else {
                        self.next_cursor = Some(next_cursor);
                        self.current_doc = next_doc;
                        break;
                    }
                } else {
                    let (tf, positions) = next_cursor.catch_up();
                    positions_buffer.extend_from_slice(&positions[..]);
                    self.next_cursor = None;
                    break;
                }
            } else {
                self.next_cursor = None;
                break;
            }
        }

        positions_buffer.sort();
        let unique_positions = keep_unique(&positions_buffer);
        self.current_tf = unique_positions.len() as DocId;
        self.current_positions = unique_positions;
        Some(current_doc)
    }

    // TODO use this as default impl for trait
    fn advance_to(&mut self, doc_id: DocId) -> Option<DocId> {
        while let Some(next_doc_id) = self.advance() {
            if next_doc_id >= doc_id {
                return Some(next_doc_id);
            }
        }

        None
    }

    fn catch_up(&mut self) -> (DocId, Vec<DocId>) {
        let positions = mem::replace(&mut self.current_positions, Vec::new());
        (self.current_tf, positions)
    }

    fn current(&self) -> Option<DocId> {
        Some(self.current_doc)
    }

    fn remains(&self) -> usize {
        self.size - self.processed
    }
}

impl<S: Sequence> Postings<S, S, S> {
    pub fn merge_without_duplicates(to_merge: &[Self]) -> VecPostings {
        let mut res = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };

        let mut merger = MergerWithoutDuplicates::new(to_merge);
        while let Some(doc) = merger.advance() {
            let (tf, positions) = merger.catch_up();
            println!("DOC: {}, TF: {}, MERGED POS: {:?}", doc, tf, positions);
            res.docs.push(doc);
            res.tfs.push(tf);
            res.positions.extend_from_slice(&positions);
        }
        println!("MERGED: docs: {:?}", &res.docs);
        println!("MERGED: tfs: {:?}", &res.tfs);
        println!("MERGED: pos: {:?}", &res.positions);
        println!("---");

        res
    }
}
