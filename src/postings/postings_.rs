use std::marker::PhantomData;
use std::cmp::Ordering;
use std::iter::FromIterator; // needed for ::from_iter
use std::collections::BinaryHeap;
use postings::{Postings,VecPostings,Sequence,SequenceStorage,PostingsCursor,SimpleCursor};
use postings::slice::SliceSequence;
use types::*;

impl<A: Sequence, B: Sequence, C: Sequence> Ord for SimpleCursor<A, B, C> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Switch compare order because Rust's BinaryHeap is a maxheap We want a minheap
        self.current().cmp(&other.current()).reverse()
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PartialOrd for SimpleCursor<A, B, C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PartialEq for SimpleCursor<A, B, C> {
    fn eq(&self, other: &Self) -> bool {
        self.current() == other.current()
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

        SimpleCursor::new(p, 0, 0, 0)
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
    current_cursor: Option<SimpleCursor<A, B, C>>,
    current_doc: Option<DocId>,
    current_positions: Option<Vec<DocId>>,
    current_tf: Option<DocId>,
    size: usize,
    processed: usize,
    //phantom: PhantomData<&'a SliceSequence<'a>>,
}

impl<A: Sequence, B: Sequence, C: Sequence> MergerWithoutDuplicates<A, B, C> {
    pub fn new(to_merge: &[Postings<A, B, C>]) -> Self {
        let size = to_merge.iter().map(|p| p.docs.remains()).fold(0, |acc, x| acc + x);
        println!("merged SIZE: {}", size);

        let mut heap = create_heap(to_merge);
        let first_cursor = heap.pop();

        MergerWithoutDuplicates {
            frontier: heap,
            current_cursor: first_cursor,
            current_doc: None,
            current_positions: None,
            current_tf: None,
            size: size,
            processed: 1,
        }
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PostingsCursor<A, B, C> for MergerWithoutDuplicates<A, B, C> {
    fn advance(&mut self) -> Option<DocId> {
        if self.current_cursor.is_none() {
            return None;
        }

        let mut current_cursor = self.current_cursor.take().unwrap();
        let current_doc = current_cursor.current();
        let mut positions_buffer = Vec::new();

        loop {
            println!("CACHING UP {}", current_doc);
            let (doc, tf, positions) = current_cursor.catch_up();
            positions_buffer.extend_from_slice(&positions[..]);

            if let Some(mut next_cursor) = self.frontier.pop() {
                self.processed += 1;
                let next_doc = next_cursor.current();
                next_cursor.advance();

                if next_doc == current_doc {
                    self.frontier.push(next_cursor);
                } else {
                    self.current_cursor = Some(next_cursor);
                    break;
                }
            } else {
                self.current_cursor = None;
            }
        }

        positions_buffer.sort();
        let unique_positions = keep_unique(&positions_buffer);
        self.current_tf = Some(unique_positions.len() as DocId);
        self.current_positions = Some(unique_positions);
        self.current_doc = Some(current_doc);
        self.current_doc
    }

    // TODO use this as default impl for trait
    fn advance_to(&mut self, doc_id: DocId) -> Option<DocId> {
        if self.current() == doc_id {
            return Some(doc_id);
        }

        while let Some(next_doc_id) = self.advance() {
            if next_doc_id >= doc_id {
                return Some(next_doc_id);
            }
        }

        None
    }

    fn catch_up(&mut self) -> (DocId, DocId, Vec<DocId>) {
        assert!(self.current_cursor.is_some());
        let positions = self.current_positions.take().unwrap();
        (self.current_doc.unwrap(), self.current_tf.unwrap(), positions)
    }

    fn current(&self) -> DocId {
        assert!(self.current_cursor.is_some());
        self.current_doc.unwrap()
    }

    fn remains(&self) -> usize {
        self.size - self.processed
    }
}

impl<S: Sequence> Postings<S, S, S> {
    pub fn merge_without_duplicates(to_merge: &[Self]) -> VecPostings {
        let cum_encoded: Vec<_> = to_merge.iter().map(|p| {
            assert_eq!(p.tfs.remains() - 1, p.docs.remains());
            println!("DOCS: {:?}", p.docs.clone().to_vec());
            println!("tfs: {:?}", p.tfs.clone().to_vec());
            println!("-----------");
            Postings {
                // TODO clones necessary?
                docs: p.docs.clone(),
                tfs: p.tfs.clone(),
                positions: p.positions.clone(),
            }
        }).collect();

        let mut res = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };

        let mut merger = MergerWithoutDuplicates::new(&cum_encoded);
        while let Some(_) = merger.advance() {
            let (doc, tf, positions) = merger.catch_up();
            res.docs.push(doc);
            res.tfs.push(tf);
            res.positions.extend_from_slice(&positions);
        }

        res
    }
}
