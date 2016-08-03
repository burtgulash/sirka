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

        let mut current_doc_id = self.cursors[0].advance().unwrap();
        'intersect: loop {
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
        }

        result
    }
}

pub struct Intersect<C: PostingsCursor> {
    cursors: Vec<C>,
    current: DocId,
    finished: bool,
    size: usize,
}

impl<C: PostingsCursor> Intersect<C> {
    pub fn new(mut cursors: Vec<C>) -> Self {
        let size = cursors.iter().map(|c| c.remains()).min().unwrap();
        let first = cursors[0].advance();
        let (finished, first) = match first {
            Some(x) => (false, x),
            None => (true, 0),
        };
        Intersect {
            cursors: cursors,
            current: first,
            finished: finished,
            size: size,
        }
    }
}

impl<C: PostingsCursor> PostingsCursor for Intersect<C> {
    type DS = C::DS;
    type TS = C::TS;
    type PS = C::PS;

    fn advance(&mut self) -> Option<DocId> {
        if self.finished {
            return None;
        }

        'align: loop {
            for cur in &mut self.cursors {
                if let Some(next_doc) = cur.advance_to(self.current) {
                    if next_doc > self.current {
                        self.current = next_doc;
                        continue 'align;
                    }
                } else {
                    self.finished = true;
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

        for cur in &mut self.cursors {
            if let Some(doc_id) = cur.advance() {
                // Start next iteration alignment with maximum doc id
                if doc_id > self.current {
                    self.current = doc_id;
                }
            } else {
                // This cursor is depleted and thus it can't produce no more matches
                self.finished = true;
                break;
            }
        }

        result_size
    }
}
