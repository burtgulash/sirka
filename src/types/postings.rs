use types::*;

pub trait PostingsStore {
    fn get_postings(&mut self, term_id: TermId) -> Option<Postings<Vec<DocId>>>;
}

#[derive(Clone)]
pub struct Postings<T> {
    pub docs: T,
    pub tfs: T,
    pub positions: T,
}

