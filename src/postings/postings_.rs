use std::marker::PhantomData;
use std::cmp::Ordering;
use std::mem;
use std::iter::FromIterator; // needed for ::from_iter
use std::collections::BinaryHeap;
use postings::{Postings,VecPostings,Sequence,SequenceStorage,PostingsCursor,SimpleCursor};
use postings::slice::SliceSequence;
use types::*;

struct FrontierPointer<A: Sequence, B: Sequence, C: Sequence> {
    current: DocId,
    cursor: SimpleCursor<A, B, C>,
}

impl<A: Sequence, B: Sequence, C: Sequence> Ord for FrontierPointer<A, B, C> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Switch compare order because Rust's BinaryHeap is a maxheap We want a minheap
        self.current.cmp(&other.current).reverse()
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PartialOrd for FrontierPointer<A, B, C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PartialEq for FrontierPointer<A, B, C> {
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> Eq for FrontierPointer<A, B, C> {}


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

fn create_heap<A: Sequence, B: Sequence, C: Sequence>(to_merge: &[Postings<A, B, C>]) -> BinaryHeap<FrontierPointer<A, B, C>> {
    BinaryHeap::from_iter(to_merge.iter().map(|pp| {
        let mut p = pp.clone();
        assert_eq!(p.docs.remains(), p.tfs.remains() - 1);
        assert!(p.docs.remains() > 0);

        let mut cur = SimpleCursor::new(p, 0, 0, 0);
        let current = cur.advance().unwrap();
        FrontierPointer {
            current: current,
            cursor: cur,
        }
    }))
}

struct MergerWithoutDuplicates<A: Sequence, B: Sequence, C: Sequence> {
    frontier: BinaryHeap<FrontierPointer<A, B, C>>,
    current_ptr: Option<FrontierPointer<A,B,C>>,
    current_doc: DocId,
    current_positions: Option<Vec<DocId>>,
    current_tf: DocId,
    size: usize,
    processed: usize,
}

impl<A: Sequence, B: Sequence, C: Sequence> MergerWithoutDuplicates<A, B, C> {
    pub fn new(to_merge: &[Postings<A, B, C>]) -> Self {
        let size = to_merge.iter().map(|p| p.docs.remains()).fold(0, |acc, x| acc + x);

        let mut heap = create_heap(to_merge);
        let mut first_ptr = heap.pop().unwrap();
        let first_doc = first_ptr.current;

        MergerWithoutDuplicates {
            frontier: heap,
            current_ptr: Some(first_ptr),
            current_doc: first_doc,
            current_positions: None,
            current_tf: 1137,
            size: size,
            processed: 1,
        }
    }
}

impl<A: Sequence, B: Sequence, C: Sequence> PostingsCursor<A, B, C> for MergerWithoutDuplicates<A, B, C> {
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
                let tf = ptr.cursor.catch_up(&mut positions_buffer);

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

            if let Some(mut next_ptr) = self.frontier.pop() {
                ptr = next_ptr;
            } else {
                self.current_ptr = None;
                break;
            }
        }
        //println!("MIMO LOOP");


        assert!(positions_buffer.len() > 0);
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

    fn current(&self) -> Option<DocId> {
        Some(self.current_doc)
    }

    fn remains(&self) -> usize {
        self.size - self.processed
    }
}

impl<S: Sequence> Postings<S, S, S> {
    pub fn merge_without_duplicates_unrolled(to_merge: &[Self]) -> VecPostings {
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

        let mut frontier = create_heap(to_merge);
        let mut ptr = frontier.pop().unwrap();
        let mut current_doc = ptr.current;
        let mut previous_doc = current_doc;
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
                    let tf = ptr.cursor.catch_up(&mut positions_buffer);
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

    pub fn merge_without_duplicates(to_merge: &[Self]) -> VecPostings {
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

        let mut merger = MergerWithoutDuplicates::new(to_merge);
        while let Some(doc) = merger.advance() {
            let tf = merger.catch_up(&mut res.positions);
//            println!("DOC: {}, TF: {}, MERGED POS: {:?}", doc, tf, positions);
            res.docs.push(doc);
            res.tfs.push(tf);
        }
       // println!("MERGED: docs: {:?}", &res.docs);
       // println!("MERGED: tfs: {:?}", &res.tfs);
       // println!("MERGED: pos: {:?}", &res.positions);
       // println!("---\n\n");

        res
    }
}
