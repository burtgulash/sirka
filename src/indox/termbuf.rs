use indox::*;
use std::mem;

pub struct TermBuf {
    buffers: Vec<Vec<DocId>>,
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
            self.buffers.push(Vec::new());
            self.max_term_id += 1;
        }

        self.buffers[term_id as usize].push(doc_id);
    }

    pub fn get_termbuf(&mut self, term_id: TermId) -> Option<Vec<DocId>> {
        if term_id > self.max_term_id {
            None
        } else {
            let buffer = mem::replace(&mut self.buffers[term_id as usize], Vec::new());
            Some(buffer)
        }
    }

    pub fn return_termbuf(&mut self, term_id: TermId, buf: Vec<DocId>) {
        if term_id > self.max_term_id {
            panic!();
        }

        mem::replace(&mut self.buffers[term_id as usize], buf);
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
            tb.add_doc(i % max, i);
        }
        let chosen_term_id = 25;
        for doc_id in tb.get_termbuf(chosen_term_id).unwrap() {
            println!("ITERATING THROUGH DOCS OF TERM_ID {}: {}", chosen_term_id, doc_id);
        }
    }
}
