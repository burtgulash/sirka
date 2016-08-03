pub use self::rawcursor::*;
pub use self::intersect::*;
pub use self::merge::*;

pub mod rawcursor;
pub mod intersect;
pub mod merge;

use types::*;
use postings::{VecPostings,Sequence};

pub trait PostingsCursor {
    type DS: Sequence;
    type TS: Sequence;
    type PS: Sequence;

    fn advance(&mut self) -> Option<DocId>;
    fn advance_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn catch_up(&mut self, positions_dst: &mut Vec<DocId>) -> DocId;
    fn current(&self) -> Option<DocId>;
    fn remains(&self) -> usize;
    fn term_id(&self) -> TermId;

    fn collect(&mut self) -> VecPostings {
        let mut result = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };
        while let Some(doc_id) = self.advance() {
            let tf = self.catch_up(&mut result.positions);
            result.docs.push(doc_id);
            result.tfs.push(tf);
        }
        result
    }
}
