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

    // TODO unsafe because it performs no bounds or error checking
    unsafe fn current(&self) -> DocId;

    fn remains(&self) -> usize;
    fn advance(&mut self) -> Option<DocId>;
    fn catch_up(&mut self, result: &mut VecPostings) -> usize;

    fn advance_to(&mut self, doc_id: DocId) -> Option<DocId> {
        if unsafe {self.current()} == doc_id {
            return Some(doc_id);
        }
        while let Some(next_doc_id) = self.advance() {
            if next_doc_id >= doc_id {
                return Some(next_doc_id);
            }
        }

        None
    }

    fn collect(&mut self) -> VecPostings {
        let mut result = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };
        while let Some(_) = self.advance() {
            let _ = self.catch_up(&mut result);
        }
        result
    }
}
