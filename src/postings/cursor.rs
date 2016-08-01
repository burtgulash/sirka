use types::*;
use postings::{Postings,Sequence};

pub trait PostingsCursor<DS, TS, PS>
    where DS: Sequence,
          TS: Sequence,
          PS: Sequence,
{
    fn advance(&mut self) -> Option<DocId>;
    fn advance_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn catch_up(&mut self) -> (DocId, DocId, Vec<DocId>);
    fn current(&self) -> DocId;
    fn remains(&self) -> usize;
}


pub struct SimpleCursor<DS, TS, PS> {
    postings: Postings<DS, TS, PS>,
    ptr: Postings<usize, usize, usize>,
    current: DocId,
    current_tf: DocId,

    i: usize,
    term_id: TermId,
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> SimpleCursor<DS, TS, PS> {
    pub fn new(mut postings: Postings<DS, TS, PS>, doc_ptr: usize, index: usize, term_id: TermId) -> Self {
        let first_doc = postings.docs.current().unwrap();
        let first_tf = postings.tfs.current().unwrap();
        // let _ = postings.docs.next();

        SimpleCursor {
            postings: postings,
            ptr: Postings {
                docs: doc_ptr,
                tfs: doc_ptr,
                positions: 0,
            },
            current: first_doc,
            current_tf: first_tf,
            i: index,
            term_id: term_id,
        }
    }
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> PostingsCursor<DS, TS, PS> for SimpleCursor<DS, TS, PS> {
    fn remains(&self) -> usize {
        self.postings.docs.remains()
    }

    fn current(&self) -> DocId {
        self.current
    }

    fn advance(&mut self) -> Option<DocId> {
        if let Some(doc_id) = self.postings.docs.current() {
            let _ = self.postings.docs.next();
            self.ptr.docs += 1;
            self.current = doc_id;

            Some(doc_id)
        } else {
            None
        }
    }

    fn advance_to(&mut self, align_to: DocId) -> Option<DocId> {
        self.ptr.docs += self.postings.docs.move_to(align_to);
        if let Some(doc_id) = self.postings.docs.current() {
            self.current = doc_id;
        } else {
            return None;
        }

        assert!(self.current >= align_to);
        Some(self.current)
    }

    fn catch_up(&mut self) -> (DocId, DocId, Vec<DocId>) {
        // Align tfs to docs
        self.postings.tfs.move_n(self.ptr.docs - 1 - self.ptr.tfs);
        let tf = self.postings.tfs.current().unwrap();
        self.ptr.tfs = self.ptr.docs - 1;

        // '-1' because tfs sequence has one more element from the sequence
        // of next term in sequence
        // assert_eq!(self.postings.tfs.remains() - 1, self.postings.docs.remains());

        // Tfs must have one more element than docs at the end. So that you can take difference
        // between 'next' and 'previous' tfs
        let next_tf = self.postings.tfs.next().unwrap();
        self.ptr.tfs += 1;
        self.current_tf = next_tf - tf;

        // TODO assign new subsequence to self.postings.positions to avoid skipping over the same
        // elements in next round
        let positions = self.postings.positions.subsequence(tf as usize, self.current_tf as usize).to_vec();

        (self.current, self.current_tf, positions)
    }
}
