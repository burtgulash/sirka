pub use self::slice::*;
pub use self::encoding::*;
pub use self::postings_::*;
pub use self::seq::*;

pub mod slice;
pub mod encoding;
pub mod postings_;
pub mod seq;

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
