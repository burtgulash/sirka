use std::collections::BinaryHeap;
use types::*;
use postings::{VecPostings,PostingsCursor};
use super::frontier::{FrontierPointer,create_heap};

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

pub struct MergerWithoutDuplicatesUnrolled<C: PostingsCursor> {
    to_merge: Option<Vec<C>>,
}

impl<C: PostingsCursor> MergerWithoutDuplicatesUnrolled<C> {
    pub fn new(to_merge: Vec<C>) -> Self {
        MergerWithoutDuplicatesUnrolled {
            to_merge: Some(to_merge),
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
        let mut current_doc = unsafe {ptr.cursor.current()};

        let mut merged = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };

        macro_rules! ADD {
            () => {
                assert!(merged.positions.len() > 0);
                (&mut merged.positions[..]).sort();
                let unique_positions = keep_unique(&merged.positions);
                let tf = unique_positions.len() as DocId;

                res.docs.push(current_doc);
                res.tfs.push(tf);
                res.positions.extend_from_slice(&unique_positions[..]);
            }
        }

        'merge: loop {
            merged.docs.clear();
            merged.tfs.clear();
            merged.positions.clear();

            loop {
                let next_doc = unsafe {ptr.cursor.current()};
                if next_doc == current_doc {
                    let _ = ptr.cursor.catch_up(&mut merged);
                    if let Some(_) = ptr.cursor.advance() {
                        frontier.push(ptr);
                    }
                } else {
                    ADD!();
                    current_doc = next_doc;
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
    merged: VecPostings,
    size: usize,
    processed: usize,
}

impl<C: PostingsCursor> MergerWithoutDuplicates<C> {
    pub fn new(to_merge: Vec<C>) -> Self {
        let size = to_merge.iter().map(|c| c.remains()).fold(0, |acc, x| acc + x);

        let mut heap = create_heap(to_merge);
        let first_ptr = heap.pop().unwrap();
        let first_doc = unsafe {first_ptr.cursor.current()};

        MergerWithoutDuplicates {
            frontier: heap,
            current_ptr: Some(first_ptr),
            current_doc: first_doc,
            merged: VecPostings {
                docs: Vec::new(),
                tfs: Vec::new(),
                positions: Vec::new(),
            },
            size: size,
            processed: 1, // heap already popped
        }
    }
}

impl<C: PostingsCursor> PostingsCursor for MergerWithoutDuplicates<C> {
    type DS = C::DS;
    type TS = C::TS;
    type PS = C::PS;

    unsafe fn current(&self) -> DocId {
        self.current_ptr.as_ref().unwrap().cursor.current()
    }

    fn advance(&mut self) -> Option<DocId> {
        if self.current_ptr.is_none() {
            return None;
        }
        self.merged.docs.clear();
        self.merged.tfs.clear();
        self.merged.positions.clear();

        let mut ptr = self.current_ptr.take().unwrap();
        let current_doc = self.current_doc;

        loop {
            //println!("LOOPING");
            let next_doc = unsafe {ptr.cursor.current()};
            if next_doc == current_doc {
                self.processed += 1;
                let _ = ptr.cursor.catch_up(&mut self.merged);

                if let Some(_) = ptr.cursor.advance() {
                    //println!("putting back: {}", next_doc);
                    self.frontier.push(ptr);
                }
            } else {
                self.current_doc = next_doc;
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

        self.merged.docs.push(current_doc);
        Some(current_doc)
    }

    fn catch_up(&mut self, result: &mut VecPostings) -> usize {
        assert!(self.merged.positions.len() > 0, "No positions found. Is 'tfs' encoded as cumulative?");
        (&mut self.merged.positions[..]).sort();
        self.merged.positions = keep_unique(&self.merged.positions);
        let tf = self.merged.positions.len();

        result.positions.extend_from_slice(&self.merged.positions[..]);
        result.tfs.push(tf as DocId);
        result.docs.push(self.merged.docs[0]);

        1
    }

    fn remains(&self) -> usize {
        self.size - self.processed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postings::{RawCursor,VecPostings,Postings,PostingsCursor,SequenceStorage};

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
        let merged = merger.collect();

        assert_eq!(merged.docs, vec![1, 2, 3]);
        assert_eq!(merged.tfs, vec![1, 2, 3]); // NOTE: result tfs are not cumulated though!
        assert_eq!(merged.positions, vec![1, 1, 2, 1, 2, 3]);
    }
}
