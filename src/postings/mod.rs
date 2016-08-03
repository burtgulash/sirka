pub use self::sequence::*;
pub use self::cursor::*;

pub mod sequence;
pub mod cursor;

use types::*;


#[derive(Clone)]
pub struct Postings<A, B, C> {
    pub docs: A,
    pub tfs: B,
    pub positions: C,
}

pub type VecPostings = Postings<Vec<DocId>, Vec<DocId>, Vec<DocId>>;

pub trait PostingsStore {
    fn get_postings(&mut self, term_id: TermId) -> Option<VecPostings>;
}
