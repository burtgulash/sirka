use std::mem;
use types::{DocId,TermId};
use write::postings::{Postings,PostingsStore};

pub struct TermBuf {
    pub buffers: Vec<Option<Vec<DocId>>>,
    max_term_id: TermId,
}

impl TermBuf {
    pub fn new() -> TermBuf {
        TermBuf {
            buffers: Vec::new(),
            max_term_id: 0,
        }
    }

    pub fn add_doc(&mut self, term_id: TermId, doc_id: DocId) {
        while self.max_term_id <= term_id {
            self.buffers.push(Some(Vec::new()));
            self.max_term_id += 1;
        }

        self.buffers[term_id as usize].as_mut().unwrap().push(doc_id);
    }

    pub fn get_termbuf(&mut self, term_id: TermId) -> Option<Vec<DocId>> {
        if term_id > self.max_term_id {
            None
        } else {
            let buffer = mem::replace(&mut self.buffers[term_id as usize], None);
            assert!(buffer.is_some());
            assert!(buffer.as_ref().unwrap().len() > 0);
            buffer
        }
    }

    #[allow(dead_code)]
    fn return_termbuf(&mut self, term_id: TermId, buf: Vec<DocId>) {
        if term_id > self.max_term_id {
            panic!();
        }

        let _ = mem::replace(&mut self.buffers[term_id as usize], Some(buf));
    }
}

macro_rules! tryopt {
    ($e:expr) => (match $e {
        Some(value) => value,
        None => return None,
    })
}

impl<'a> PostingsStore for (&'a mut TermBuf, &'a mut TermBuf, &'a mut TermBuf) {
    fn get_postings(&mut self, term_id: TermId) -> Option<Postings> {
        Some(Postings {
            docs: tryopt!(self.0.get_termbuf(term_id)),
            tfs: tryopt!(self.1.get_termbuf(term_id)),
            positions: tryopt!(self.2.get_termbuf(term_id)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut tb = TermBuf::new();
        let max = 31;
        for i in 1..2000 {
            tb.add_doc(i % max, i as u64);
        }
        let chosen_term_id = 25;
        for doc_id in tb.get_termbuf(chosen_term_id).unwrap() {
            println!("ITERATING THROUGH DOCS OF TERM_ID {}: {}", chosen_term_id, doc_id);
        }
    }
}
