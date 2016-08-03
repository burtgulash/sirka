use types::*;
use postings::{PostingsCursor,VecPostings};

pub struct IntersectUnrolled<C: PostingsCursor> {
    cursors: Vec<C>
}

impl <C: PostingsCursor> IntersectUnrolled<C> {
    fn collect(mut cursors: Vec<C>) -> VecPostings {
        let mut result = VecPostings {
            docs: Vec::new(),
            tfs: Vec::new(),
            positions: Vec::new(),
        };

        let mut current_doc_id = cursors[0].current().unwrap();
        'intersect: loop {
            'align: loop {
                for cur in &mut cursors {
                    if let Some(doc_id) = cur.advance_to(current_doc_id) {
                        if doc_id > current_doc_id {
                            current_doc_id = doc_id;
                            continue 'align;
                        }
                    } else {
                        break 'intersect;
                    }
                }
                break 'align;
            }

            for cur in &mut cursors {
                let tf = cur.catch_up(&mut result.positions);
                result.docs.push(current_doc_id);
                result.tfs.push(tf);

                if let Some(doc_id) = cur.advance() {
                    // Start next iteration alignment with maximum doc id
                    if doc_id > current_doc_id {
                        current_doc_id = doc_id;
                    }
                } else {
                    // This cursor is depleted and thus it can't produce no more matches
                    break 'intersect;
                }
            }
        }

        result
    }
}
