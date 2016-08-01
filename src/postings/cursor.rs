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
    advanced: bool,
    ahead: usize,

    i: usize,
    term_id: TermId,
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> SimpleCursor<DS, TS, PS> {
    pub fn new(mut postings: Postings<DS, TS, PS>, doc_ptr: usize, index: usize, term_id: TermId) -> Self {
        SimpleCursor {
            postings: postings,
            i: index,
            term_id: term_id,
            advanced: true,
            ahead: 0,
        }
    }
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> PostingsCursor<DS, TS, PS> for SimpleCursor<DS, TS, PS> {
    fn remains(&self) -> usize {
        self.postings.docs.remains()
    }

    fn current(&self) -> Option<DocId> {
        Some(self.postings.docs.current())
    }

    fn advance(&mut self) -> Option<DocId> {
        self.advanced = true;
        self.ahead += 1;
        self.postings.docs.next() 
    }

    fn advance_to(&mut self, align_to: DocId) -> Option<DocId> {
        let (skipped, x) = self.postings.docs.skip_to(align_to);
        self.ahead += skipped;
        x
    }

    fn catch_up(&mut self) -> (DocId, Vec<DocId>) {
        assert!(self.advanced);
        self.advanced = false;

        //println!("DOCPTR: {}, TFPTR: {}", self.ptr.docs, self.ptr.tfs);
        // Align tfs to docs
        let start_tf = self.postings.tfs.skip_n(self.ahead).unwrap();
        let next_tf = self.postings.tfs.next().unwrap();
        self.ahead = 0;

        let tf = next_tf - start_tf;
        println!("TF: {}", tf);
        let positions = self.postings.positions.subsequence(start_tf as usize, next_tf as usize).to_vec();

        (tf, positions)
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
