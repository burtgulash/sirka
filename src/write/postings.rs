use std::cmp::Ordering;
use std::iter::FromIterator;
use std::collections::BinaryHeap;
use types::{TermId,DocId};
use types::{Sequence};


pub trait PostingsStore {
    fn get_postings(&mut self, term_id: TermId) -> Option<Postings<Vec<DocId>>>;
}

#[derive(Clone)]
pub struct Postings<T> {
    pub docs: T,
    pub tfs: T,
    pub positions: T,
}

struct FrontierPointer<S> {
    current_doc: DocId,
    current_pos: usize,
    postings: Postings<S>,
}

impl<S> Ord for FrontierPointer<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Switch compare order because Rust's BinaryHeap is a maxheap We want a minheap
        self.current_doc.cmp(&other.current_doc).reverse()
    }
}

impl<S> PartialOrd for FrontierPointer<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl<S> PartialEq for FrontierPointer<S> {
    fn eq(&self, other: &Self) -> bool { self.current_doc == other.current_doc }
}

impl<S> Eq for FrontierPointer<S> {}

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

struct Merger<S> {
    frontier: BinaryHeap<FrontierPointer<S>>,
}

impl<S: Sequence> Merger<S> {
    pub fn new(to_merge: &[Postings<S>]) -> Self {
        let heap = BinaryHeap::from_iter(to_merge.iter().map(|pp| {
            let mut p = pp.clone();
            assert!(p.docs.remains() == p.tfs.remains());
            assert!(p.docs.remains() > 0);

            let first_doc = p.docs.next().unwrap();
            FrontierPointer {
                current_doc: first_doc,
                current_pos: 1,
                postings: Postings {
                    docs: p.docs,
                    tfs: p.tfs,
                    positions: p.positions,
                }
            }
        }));

        Merger {
            frontier: heap,
        }
    }

    fn next(&mut self) -> Option<FrontierPointer<S>> {
        self.frontier.pop()
    }

    fn put(&mut self, ptr: FrontierPointer<S>) {
        self.frontier.push(ptr);
    }
}

impl<S: Sequence> Postings<S> {
    pub fn merge_without_duplicates(to_merge: &[Postings<S>]) -> Postings<Vec<DocId>> {
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

        while let Some(mut ptr) = merger.next() {
            let doc_id = ptr.current_doc;
            let tf = ptr.postings.tfs.next().unwrap();
            assert!(tf > 0);

            if last_doc_id == 0 || doc_id == last_doc_id {
                for _ in 0..tf {
                    let pos = ptr.postings.positions.next().unwrap();
                    tmp_pos.push(pos)
                }
            } else {
                assert!(doc_id > last_doc_id);

                // TODO is this necessary?
                if last_doc_id != 0 {
                    ADD_DOC!();
                }
                last_doc_id = doc_id;
            }

            // Insert next doc_id into heap if it exists
            if let Some(next_doc_id) = ptr.postings.docs.next() {
                ptr.current_doc = next_doc_id;
                merger.put(ptr);
            }
        }
        ADD_DOC!();

        res
    }
}
