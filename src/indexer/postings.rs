use std::cmp::{self,Ordering};
use std::iter::FromIterator;
use std::collections::BinaryHeap;
use indexer::*;


pub struct PostingsT<T> {
    pub docs: T,
    pub tfs: T,
    pub positions: T,
}
pub type Postings = PostingsT<Vec<DocId>>;

#[derive(Clone, Copy)]
struct IteratorPointer {
    it_i: usize,
    current_doc: DocId,
}

impl Ord for IteratorPointer {
    fn cmp(&self, other: &Self) -> Ordering {
        // Switch compare order because Rust's BinaryHeap is a maxheap
        // We want a minheap
        (&-(self.current_doc as isize)).cmp(&-(other.current_doc as isize))
    }
}

impl PartialOrd for IteratorPointer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for IteratorPointer {
    fn eq(&self, other: &Self) -> bool {
        self.current_doc == other.current_doc
    }
}

impl Eq for IteratorPointer {}

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

impl Postings {
    pub fn merge(to_merge: &[&Postings]) -> Postings {
        let mut its = to_merge.iter().map(|p| PostingsT {
            docs: p.docs.iter().cloned(),
            tfs: p.tfs.iter().cloned(),
            positions: p.positions.iter().cloned(),
        }).collect::<Vec<PostingsT<_>>>();

        let mut h = BinaryHeap::from_iter(its.iter_mut().enumerate().map(|(i, p)| {
            IteratorPointer{it_i: i, current_doc: p.docs.next().unwrap()}
        }));

        let mut res_docs = Vec::<DocId>::new();
        let mut res_tfs = Vec::<DocId>::new();
        let mut res_pos = Vec::<DocId>::new();

        let mut last_doc_id = 0;
        let mut tmp_pos: Vec<DocId> = Vec::new();

        macro_rules! ADD_DOC {
            () => {
                tmp_pos.sort();
                let unique_positions = keep_unique(&tmp_pos);
                res_pos.extend_from_slice(&unique_positions);
                tmp_pos.clear();

                res_docs.push(last_doc_id);
                res_tfs.push(unique_positions.len() as DocId);
            }
        }

        while let Some(mut itptr) = h.pop() {
            if let Some(doc_id) = its[itptr.it_i].docs.next() {
                itptr.current_doc = doc_id;
                h.push(itptr);

                let it_tf = its[itptr.it_i].tfs.next().unwrap();
                if doc_id == last_doc_id {
                    for _ in 0..it_tf {
                        let pos = its[itptr.it_i].positions.next().unwrap();
                        tmp_pos.push(pos)
                    }
                } else {
                    if last_doc_id != 0 {
                        ADD_DOC!();
                    }

                    last_doc_id = doc_id;
                }
            }
        }
        ADD_DOC!();

        Postings {
            docs: res_docs,
            tfs: res_tfs,
            positions: res_pos,
        }
    }
}
