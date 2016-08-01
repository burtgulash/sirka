use types::*;
use postings::{Postings,Sequence};

pub trait PostingsCursor<DS, TS, PS>
    where DS: Sequence,
          TS: Sequence,
          PS: Sequence,
{
    fn advance(&mut self) -> Option<DocId>;
    fn advance_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn catch_up(&mut self) -> (DocId, Vec<DocId>);
    fn current(&self) -> Option<DocId>;
    fn remains(&self) -> usize;
}


pub struct SimpleCursor<DS, TS, PS> {
    postings: Postings<DS, TS, PS>,
    ptr: Postings<usize, usize, usize>,
    current: Option<DocId>,
    current_tf: DocId,
    advanced: bool,

    i: usize,
    term_id: TermId,
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> SimpleCursor<DS, TS, PS> {
    pub fn new(mut postings: Postings<DS, TS, PS>, doc_ptr: usize, index: usize, term_id: TermId) -> Self {
        let first_doc = postings.docs.advance().unwrap();
        let first_tf = postings.tfs.current().unwrap();

        SimpleCursor {
            postings: postings,
            ptr: Postings {
                docs: doc_ptr + 1,
                tfs: doc_ptr,
                positions: 0,
            },
            current: Some(first_doc),
            current_tf: first_tf,
            i: index,
            term_id: term_id,
            advanced: true,
        }
    }
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> PostingsCursor<DS, TS, PS> for SimpleCursor<DS, TS, PS> {
    fn remains(&self) -> usize {
        self.postings.docs.remains() + 1
    }

    fn current(&self) -> Option<DocId> {
        self.current
    }

    fn advance(&mut self) -> Option<DocId> {
        self.advanced = true;
        self.ptr.docs += 1;
        if let Some(doc_id) = self.postings.docs.advance() {
            let current = self.current;
            self.current = Some(doc_id);
            current
        } else if self.current.is_some() {
            let current = self.current;
            self.current = None;
            current
        } else {
            self.advanced = false;
            None
        }
    }

    fn advance_to(&mut self, align_to: DocId) -> Option<DocId> {
        self.ptr.docs += self.postings.docs.move_to(align_to);
        if let Some(doc_id) = self.postings.docs.current() {
            self.current = Some(doc_id);
            assert!(doc_id >= align_to);
            Some(doc_id)
        } else {
            None
        }
    }

    fn catch_up(&mut self) -> (DocId, Vec<DocId>) {
        assert!(self.advanced);
        self.advanced = false;

        //println!("DOCPTR: {}, TFPTR: {}", self.ptr.docs, self.ptr.tfs);
        // Align tfs to docs
        self.postings.tfs.move_n(self.ptr.docs - 2 - self.ptr.tfs).unwrap();
        self.ptr.tfs = self.ptr.docs - 2;

        let tf = self.postings.tfs.advance().unwrap();
        self.ptr.tfs += 1;

        let next_tf = self.postings.tfs.current().unwrap();
        self.current_tf = next_tf - tf;


        // TODO assign new subsequence to self.postings.positions to avoid skipping over the same
        // elements in next round
        let positions = self.postings.positions.subsequence(tf as usize, self.current_tf as usize).to_vec();
        let nn = positions.clone();
        // println!("CATCH UP: {}-{}, {:?}", tf, next_tf - tf, nn.to_vec());

        (self.current_tf, positions)
    }
}



#[cfg(test)]
mod tests {
    use types::*;
    use super::*;
    use postings::{Postings,Sequence,SequenceStorage};

    #[test]
    fn test_cursor() {
        let ps = Postings {
            docs: vec![3,5],
            tfs:  vec![0, 3, 8],
            positions: vec![3,3,3,5,5,5,5,5],
        };
        let seqs = Postings {
            docs: (&ps.docs).to_sequence(),
            tfs: (&ps.tfs).to_sequence(),
            positions: (&ps.positions).to_sequence(),
        };
        let mut cur = SimpleCursor::new(seqs, 0, 0, 0);
        while let Some(doc_id) = cur.advance() {
            let (tf, positions) = cur.catch_up();
            println!("DOC: {}, TF: {}, POSITIONS: {:?}", doc_id, tf, positions);
        }
        println!("---");
    }

    #[test]
    fn test_cursor2() {
        let ps = Postings {
            docs: vec![1],
            tfs:  vec![0, 1],
            positions: vec![1337],
        };
        let seqs = Postings {
            docs: (&ps.docs).to_sequence(),
            tfs: (&ps.tfs).to_sequence(),
            positions: (&ps.positions).to_sequence(),
        };
        let mut cur = SimpleCursor::new(seqs, 0, 0, 0);
        while let Some(doc_id) = cur.advance() {
            let (tf, positions) = cur.catch_up();
            println!("DOC: {}, TF: {}, POSITIONS: {:?}", doc_id, tf, positions);
        }
        println!("---");
    }
}
