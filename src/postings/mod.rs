pub use self::slice::*;
pub use self::postings_::*;
pub use self::seq::*;
pub use self::cursor::*;

pub mod slice;
pub mod postings_;
pub mod seq;
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
