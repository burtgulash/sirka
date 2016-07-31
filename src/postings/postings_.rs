use std::cmp::Ordering;
use std::iter::FromIterator; // needed for ::from_iter
use std::collections::BinaryHeap;
use postings::{Postings,VecPostings,Sequence,PostingsCursor,SimpleCursor};
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
        assert!(p.docs.remains() == p.tfs.remains());
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
    current_positions: Option<Vec<DocId>>,
    current_tf: Option<DocId>,
    size: usize,
}

impl<A: Sequence, B: Sequence, C: Sequence> MergerWithoutDuplicates<A, B, C> {
    pub fn new(to_merge: &[Postings<A, B, C>]) -> Self {
        let size = to_merge.iter().map(|p| p.docs.remains()).fold(0, |acc, &x| acc + x);
        println!("merged SIZE: {}", size);

        let mut heap = create_heap(to_merge);
        MergerWithoutDuplicates {
            frontier: heap,
            current_cursor: heap.pop(),
            current_positions: None,
            current_tf: None,
            size: size,
        }
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PostingsCursor<A, B, C> for MergerWithoutDuplicates<A, B, C> {
    fn advance<S: Sequence>(&mut self) -> Option<DocId> {
        if let Some(mut current_cursor) = self.current_cursor {
            let current_doc = current_cursor.current();
            let positions_buffer = Vec::new();

            loop {
                let (tf, mut positions) = current_cursor.catch_up();
                while let Some(position) = positions.next() {
                    positions_buffer.push(position);
                }

                if let Some(mut next_cursor) = current_cursor.advance() {
                    let next_doc = next_cursor.current();
                    self.current_cursor = Some(next_cursor);

                    if next_doc != current_doc {
                        break;
                    }
                } else {
                    self.current_cursor = None;
                }
            }

            positions_buffer.sort();
            self.current_positions = Some(keep_unique(&positions_buffer));
            self.current_tf = Some(self.current_positions.len() as DocId);

            Some(current_doc)
        } else {
            None
        }
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

    fn catch_up<S: Sequence>(&mut self) -> (DocId, DocId, S) {
        assert!(self.current_cursor.is_some());
        (self.current_cursor.current(), self.current_tf.unwrap(), self.current_positions.as_ref().unwrap().to_sequence())
    }

    fn current(&self) -> DocId {
        assert!(self.current_cursor.is_some());
        self.current_cursor.current();
    }

    fn remains(&self) -> DocId {
        assert!(self.current_cursor.is_some());
        self.current_cursor.current();
    }
}

impl<S: Sequence> Postings<S, S, S> {
    pub fn merge_without_duplicates(to_merge: &[Self]) -> VecPostings {
        let mut merger = Merger::new(to_merge);
        let mut last_doc_id = 0;
        let mut tmp_pos: Vec<DocId> = Vec::new();
        let mut res = Postings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };

        macro_rules! ADD_DOC {
            () => {
                tmp_pos.sort();
                let unique_positions = keep_unique(&tmp_pos);

                res.positions.extend_from_slice(&unique_positions);
                tmp_pos.clear();

                res.docs.push(last_doc_id);
                res.tfs.push(unique_positions.len() as DocId);
            }
        }

        while let Some(mut cur) = merger.next() {
            let doc_id = cur.current();

            if last_doc_id == 0 || doc_id == last_doc_id {
                let mut positions = cur.current_positions();
                while let Some(position) = positions.next() {
                    tmp_pos.push(position);
                }
            } else {
                assert!(doc_id > last_doc_id);

                // TODO is this necessary?
                if last_doc_id != 0 {
                    ADD_DOC!();
                }
                last_doc_id = doc_id;
            }

            if let Some(next_doc_id) = cur.advance() {
                merger.put(cur);
            }
        }
        ADD_DOC!();

        res
    }
}
