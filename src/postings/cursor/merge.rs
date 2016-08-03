use std::cmp::Ordering;
use std::iter::FromIterator; // needed for ::from_iter
use std::collections::BinaryHeap;
use postings::{VecPostings,PostingsCursor};
use types::*;

struct FrontierPointer<C: PostingsCursor> {
    current: DocId,
    cursor: C,
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

fn create_heap<C: PostingsCursor>(to_merge: Vec<C>) -> BinaryHeap<FrontierPointer<C>> {
    BinaryHeap::from_iter(to_merge.into_iter().map(|mut cur| {
        let current = cur.advance().unwrap();
        FrontierPointer {
            current: current,
            cursor: cur,
        }
    }))
}

pub struct MergerWithoutDuplicatesUnrolled<C: PostingsCursor> {
    to_merge: Option<Vec<C>>,
    #[allow(dead_code)]
    term_id: TermId,
}

impl<C: PostingsCursor> MergerWithoutDuplicatesUnrolled<C> {
    pub fn new(to_merge: Vec<C>, term_id: TermId) -> Self {
        MergerWithoutDuplicatesUnrolled {
            to_merge: Some(to_merge),
            term_id: term_id,
        }
    }

    pub fn collect(&mut self) -> VecPostings {
        let mut res = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };

       //println!("TO MERGE:");
       //for m in to_merge.iter() {
       //    println!("DOCS: {:?}", m.docs.clone().to_vec());
       //    println!("tfs: {:?}", m.tfs.clone().to_vec());
       //    println!("pos: {:?}", m.positions.clone().to_vec());
       //}
       //println!("---");

        let mut frontier = create_heap(self.to_merge.take().unwrap());
        let mut ptr = frontier.pop().unwrap();
        let mut current_doc = ptr.current;
        let mut positions_buffer = Vec::new();

        macro_rules! ADD {
            () => {
                assert!(positions_buffer.len() > 0);
                positions_buffer.sort();
                let unique_positions = keep_unique(&positions_buffer);
                let tf = unique_positions.len() as DocId;
                positions_buffer.clear();

                res.docs.push(current_doc);
                res.tfs.push(tf);
                res.positions.extend_from_slice(&unique_positions[..]);
            }
        }

        'merge: loop {
            loop {
                if ptr.current == current_doc {
                    let _ = ptr.cursor.catch_up(&mut positions_buffer);
                    if let Some(next_doc) = ptr.cursor.advance() {
                        ptr.current = next_doc;
                        frontier.push(ptr);
                    }
                } else {
                    ADD!();
                    current_doc = ptr.current;
                    break;
                }

                if let Some(next_ptr) = frontier.pop() {
                    ptr = next_ptr;
                } else {
                    ADD!();
                    break 'merge;
                }
            }
        }
         // println!("MERGED: docs: {:?}", &res.docs);
         // println!("MERGED: tfs: {:?}", &res.tfs);
         // println!("MERGED: pos: {:?}", &res.positions);
         // println!("---\n\n");

        res
    }
}

pub struct MergerWithoutDuplicates<C: PostingsCursor> {
    frontier: BinaryHeap<FrontierPointer<C>>,
    current_ptr: Option<FrontierPointer<C>>,
    current_doc: DocId,
    current_positions: Option<Vec<DocId>>,
    current_tf: DocId,
    term_id: TermId,
    size: usize,
    processed: usize,
}

impl<C: PostingsCursor> MergerWithoutDuplicates<C> {
    pub fn new(to_merge: Vec<C>, term_id: TermId) -> Self {
        let size = to_merge.iter().map(|c| c.remains()).fold(0, |acc, x| acc + x);

        let mut heap = create_heap(to_merge);
        let first_ptr = heap.pop().unwrap();
        let first_doc = first_ptr.current;

        MergerWithoutDuplicates {
            frontier: heap,
            current_ptr: Some(first_ptr),
            current_doc: first_doc,
            current_positions: None,
            current_tf: 1337,
            term_id: term_id,
            size: size,
            processed: 1, // heap already popped
        }
    }
}

impl<C: PostingsCursor> PostingsCursor for MergerWithoutDuplicates<C> {
    type DS = C::DS;
    type TS = C::TS;
    type PS = C::PS;

    fn advance(&mut self) -> Option<DocId> {
        if self.current_ptr.is_none() {
            return None;
        }
        let mut positions_buffer = Vec::new();
        //println!("CUR: {}", self.current_doc);
                    //println!("HEAP SIZE: {}", self.frontier.len());

        let mut ptr = self.current_ptr.take().unwrap();
        let current_doc = self.current_doc;

        loop {
            //println!("LOOPING");
            if ptr.current == current_doc {
                self.processed += 1;
                let _ = ptr.cursor.catch_up(&mut positions_buffer);

                if let Some(next_doc) = ptr.cursor.advance() {
                    ptr.current = next_doc;
                    //println!("putting back: {}", next_doc);
                    self.frontier.push(ptr);
                }
            } else {
                self.current_doc = ptr.current;
                    //println!("putting backa: {}", self.current_doc);
                self.current_ptr = Some(ptr);
                // self.frontier.push(ptr);
                //println!("HEAP SIZE: {}", self.frontier.len());
                break;
            }

            if let Some(next_ptr) = self.frontier.pop() {
                ptr = next_ptr;
            } else {
                self.current_ptr = None;
                break;
            }
        }
        //println!("MIMO LOOP");


        assert!(positions_buffer.len() > 0, "No positions found. Is 'tfs' encoded as cumulative?");
        positions_buffer.sort();
        let unique_positions = keep_unique(&positions_buffer);
        self.current_tf = unique_positions.len() as DocId;
        self.current_positions = Some(unique_positions);

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

    fn catch_up(&mut self, positions_dst: &mut Vec<DocId>) -> DocId {
        for position in self.current_positions.take().unwrap() {
            positions_dst.push(position);
        }
        self.current_tf
    }

    fn term_id(&self) -> TermId {
        self.term_id
    }

    fn current(&self) -> Option<DocId> {
        Some(self.current_doc)
    }

    fn remains(&self) -> usize {
        self.size - self.processed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postings::{RawCursor,VecPostings,Postings,PostingsCursor,Sequence,SequenceStorage};

    #[test]
    fn test_merge_without_duplicates() {
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

        let mut merger = MergerWithoutDuplicates::new(vec![ps1c, ps2c]);
        let mut merged = merger.collect();

        assert_eq!(merged.docs, vec![1, 2, 3]);
        assert_eq!(merged.tfs, vec![1, 2, 3]); // NOTE: result tfs are not cumulated though!
        assert_eq!(merged.positions, vec![1, 1, 2, 1, 2, 3]);
    }
}
