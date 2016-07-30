pub use self::slice::*;
pub use self::encoding::*;
pub use self::postings_::*;
pub use self::seq::*;

pub mod slice;
pub mod encoding;
pub mod postings_;
pub mod seq;

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

