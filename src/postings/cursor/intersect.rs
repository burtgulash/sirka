use types::*;
use postings::{PostingsCursor,VecPostings};
use std::usize;

pub struct IntersectUnrolled<C: PostingsCursor> {
    cursors: Vec<C>
}

impl <C: PostingsCursor> IntersectUnrolled<C> {
    pub fn new(cursors: Vec<C>) -> Self {
        IntersectUnrolled {
            cursors: cursors
        }
    }

    pub fn collect(&mut self) -> VecPostings {
        let mut result = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };

        let mut current_doc_id = 0;
        'intersect: loop {
            for cur in &mut self.cursors {
                if let Some(doc_id) = cur.advance() {
                    // Start next iteration alignment with maximum doc id
                    if doc_id > current_doc_id {
                        current_doc_id = doc_id;
                    }
                } else {
                    // This cursor is depleted and thus it can't produce no more matches
                    break 'intersect;
                }
            }

            'align: loop {
                for cur in &mut self.cursors {
                    if let Some(doc_id) = cur.advance_to(current_doc_id) {
                        if doc_id > current_doc_id {
                            current_doc_id = doc_id;
                            continue 'align;
                        }
                    } else {
                        break 'intersect;
                    }
                }
                break 'align;
            }

            for cur in &mut self.cursors {
                let _ = cur.catch_up(&mut result);
            }
        }

        result
    }
}

pub struct Intersect<C: PostingsCursor> {
    cursors: Vec<C>,
    current: DocId,
    size: usize,
}

impl<C: PostingsCursor> Intersect<C> {
    pub fn new(cursors: Vec<C>) -> Self {
        let size = cursors.iter().map(|c| c.remains()).min().unwrap();
        Intersect {
            cursors: cursors,
            current: 0,
            size: size,
        }
    }
}

impl<C: PostingsCursor> PostingsCursor for Intersect<C> {
    type DS = C::DS;
    type TS = C::TS;
    type PS = C::PS;

    unsafe fn current(&self) -> DocId {
        // when matched then all cursors must have the same current() docid
        assert_eq!(self.cursors[0].current(), self.current);
        self.current
    }

    fn advance(&mut self) -> Option<DocId> {
        for cur in &mut self.cursors {
            if let Some(doc_id) = cur.advance() {
                // Start next iteration alignment with maximum doc id
                if doc_id > self.current {
                    self.current = doc_id;
                }
            } else {
                return None;
            }
        }

        'align: loop {
            for cur in &mut self.cursors {
                if let Some(next_doc) = cur.advance_to(self.current) {
                    if next_doc > self.current {
                        self.current = next_doc;
                        continue 'align;
                    }
                } else {
                    return None;
                }
            }
            return Some(self.current);
        }
    }

    fn remains(&self) -> usize {
        self.size
    }

    fn catch_up(&mut self, result: &mut VecPostings) -> usize {
        let mut result_size = 0;
        for cur in &mut self.cursors {
            result_size += cur.catch_up(result);
        }
        result_size
    }
}
