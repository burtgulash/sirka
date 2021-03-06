use std::{str,slice,cmp,mem};
use std::io::{Write};
use std::rc::{Rc,Weak};
use std::cell::{RefCell,Ref,RefMut};
use std::ops::Deref;
use std::iter::{Iterator};

use types::*;
use util::*;
use nutrie::TrieNodeHeader;
use postings::{VecPostings,Postings,PostingsStore,SequenceStorage,SequenceEncoder,RawCursor,MergerWithoutDuplicatesUnrolled};


fn delta_encode(xs: &[DocId]) -> Vec<DocId> {
    let mut v = Vec::new();
    if xs.len() < 1 {
        v.extend_from_slice(xs);
    } else {
        let tail = xs[1..].iter();
        v.push(xs[0]);
        v.extend(xs.iter().zip(tail).map(|(a, b)| b - a));
    }
    v
}

#[derive(Clone)]
pub struct WrittenTerm {
    term: String,
    term_ptr: usize,
    term_id: TermId,
}

impl WrittenTerm {
    fn new(term: &str, term_id: TermId, term_ptr: usize) -> WrittenTerm {
        WrittenTerm {
            term: term.into(),
            term_ptr: term_ptr,
            term_id: term_id,
        }
    }
}

pub struct PostingsEncoders<DocsEncoder, TfsEncoder, PosEncoder> {
    pub docs: DocsEncoder,
    pub tfs: TfsEncoder,
    pub positions: PosEncoder,
}

pub fn create_trie<PS, W, DE, TE, PE>(mut term_serial: TermId, terms: &[Term], postings_store: &mut PS,
                                          dict_out: &mut W, enc: &mut PostingsEncoders<DE, TE, PE>)
    -> (Vec<WrittenTerm>, usize, usize, usize)
    where PS: PostingsStore,
          W: Write,
          DE: SequenceEncoder,
          TE: SequenceEncoder,
          PE: SequenceEncoder
{
    let mut new_terms = Vec::<WrittenTerm>::new();

    // Create 2 dummy roots - because you need 2 node pointers - parent and current
    let root_term = WrittenTerm::new("", 0, 0);
    let root1 = TrieNode::new(None, root_term.clone(), None);
    let root2 = TrieNode::new(Some(root1.clone()), root_term, None);
    root1.clone().add_child(root2.clone());

    let mut parent: TrieNode = root1.clone();
    let mut current: TrieNode = root2.clone();

    let mut term_ptr = 0;
    let mut dict_ptr = 0;
    let mut postings_ptr = 0;
    let mut last_tf = 0;

    for &Term{ref term, term_id} in terms.iter() {
        // TODO add null termination to terminals
        let nullterm = format!("{}{}", term, "\0");

        let prefix_len = common_prefix_len(&current.borrow().t.term, &nullterm);
        let child_postings = postings_store.get_postings(term_id);

        //println!("IT {} {} {}", current.borrow().t.term, term, prefix_len);

        // align parent and current pointers
        while prefix_len < parent.borrow().t.term.len() {
            current.flush(&parent, &mut dict_ptr, &mut postings_ptr, &mut last_tf, dict_out, enc);
            current = parent.clone();
            parent = parent.parent().unwrap();
        }

        if prefix_len >= current.term_len() {
            parent = current.clone();
            // NOTE: With null terminating strings this must be unreachable (except root)
            assert_eq!(current.term_id(), 0);
        } else if prefix_len == parent.term_len() {
            current.flush(&parent, &mut dict_ptr, &mut postings_ptr, &mut last_tf, dict_out, enc);
        } else if prefix_len > parent.term_len() {
            //let parent_term_ptr = current.borrow().term_ptr;
            term_serial += 1;
            let new_term = {
                let last_term = &current.borrow().t.term;
                let nt = &last_term[.. cmp::min(last_term.len(), prefix_len)];
                WrittenTerm::new(nt, term_serial, current.term_ptr())
            };

            new_terms.push(new_term.clone());
            let fork_node = TrieNode::new(
                current.parent(),
                new_term,
                None,
            );

            // Flush with fork_node as a new parent
            current.flush(&fork_node, &mut dict_ptr, &mut postings_ptr, &mut last_tf, dict_out, enc);

            parent = current.clone();
            current = fork_node.clone();
            mem::swap(&mut *current.borrow_mut(), &mut *parent.borrow_mut());

            current.set_parent(&parent);
            parent.add_child(fork_node);
        }

        let new_term = WrittenTerm::new(&nullterm, term_id, term_ptr);
        new_terms.push(new_term.clone());
        // Don't write the terminating '\0' character. Use term.len(), not nullterm.len()
        term_ptr += term.len();

        let parent_clone = parent.clone();
        current = parent.add_child(TrieNode::new(
            Some(parent_clone),
            new_term,
            child_postings,
        ));
    }

    while let Some(parent_parent) = parent.parent() {
        current.flush(&parent, &mut dict_ptr, &mut postings_ptr, &mut last_tf, dict_out, enc);
        current = parent.clone();
        parent = parent_parent;
    }

    let root_ptr = dict_ptr;
    assert!(current.parent().unwrap().term_id() == 0);
    // Flush root2 node
    current.flush(&parent, &mut dict_ptr, &mut postings_ptr, &mut last_tf, dict_out, enc);

    // Don't forget to write last_tf so that differences tfs[i + 1] - tfs[i] work for all doc
    // positions
    let _ = enc.tfs.write(last_tf).unwrap();


    for t in terms.iter() {
        dict_out.write(&t.term.as_bytes()).unwrap();
    }

    (new_terms, dict_ptr, root_ptr, term_ptr)
}

// 't: 'n means that terms ('t) can live longer than nodes ('n) It is needed so that root term can
// be allocated in shorter lifetime than that of other terms.  No other reason
type TrieNodeRef = Rc<RefCell<_TrieNode>>;
type TrieNodeWeak = Weak<RefCell<_TrieNode>>;
struct TrieNode(TrieNodeRef);
struct _TrieNode {
    t: WrittenTerm,
    pointer_in_dictbuf: Option<usize>,
    postings: Option<VecPostings>,
    parent: Option<TrieNodeWeak>,
    children: Vec<TrieNodeRef>,
}

impl TrieNode {
    fn new(parent: Option<TrieNode>, t: WrittenTerm, postings: Option<VecPostings>) -> TrieNode {
        //if let Some(ref p) = postings {
        //    println!("");
        //    println!("docs: {:?}", &p.docs);
        //    println!("tfs : {:?}", &p.tfs);
        //    println!("pos : {:?}", &p.positions);
        //    println!("---");
        //}
        TrieNode(Rc::new(RefCell::new(_TrieNode {
            t: t,
            pointer_in_dictbuf: None,
            postings: postings,
            parent: parent.map(|p| Rc::downgrade(&p.clone())),
            children: Vec::new(),
        })))
    }

    fn set_parent(&mut self, parent: &TrieNodeRef) {
        self.borrow_mut().parent = Some(Rc::downgrade(parent));
        //(&mut *current.borrow_mut()).parent = Some(Rc::downgrade(&parent));
    }

    fn parent(&self) -> Option<TrieNode> {
        match self.borrow().parent {
            Some(ref weak_link) => Some(TrieNode(weak_link.upgrade().unwrap())),
            None => None,
        }
    }

    fn term_id(&self) -> TermId {
        self.borrow().t.term_id
    }

    fn term_ptr(&self) -> usize {
        self.borrow().t.term_ptr
    }

//    fn parent_term_len(&self) -> usize {
//        self.parent().t.term.len()
//    }

    fn term_len(&self) -> usize {
        self.borrow().t.term.len()
    }

    fn add_child(&mut self, child: TrieNode) -> TrieNode {
        let borrow = child.clone();
        self.borrow_mut().children.push(child.0);
        TrieNode(borrow.0)
    }

    fn borrow(&self) -> Ref<_TrieNode> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> RefMut<_TrieNode> {
        self.0.borrow_mut()
    }

    fn postings_len(&self) -> usize {
        if let Some(ref postings) = self.borrow().postings {
            postings.docs.len()
        } else {
            0
        }
    }

    fn create_child_pointers(&self) -> Vec<u32> {
        self.borrow().children.iter().map(|ch| {
            ch.borrow().pointer_in_dictbuf.expect("This node must be written by now") as u32
        }).collect()
    }

    fn create_child_index(&self) -> Vec<u32> {
        let prefix = &self.borrow().t.term;
        self.borrow().children.iter().map(|ch| {
            let ch_borrow = ch.borrow();
            let suffix = &ch_borrow.t.term[prefix.len()..];
            // println!("prefix='{}',   term='{}', suffix='{}'", prefix, ch_borrow.t.term, suffix);
            assert!(suffix.len() > 0);
            first_letter(suffix)
        }).collect()
    }

    fn write_postings<DE, TE, PE>(&self, enc: &mut PostingsEncoders<DE, TE, PE>, postings_ptr: &mut DocId, last_tf: &mut DocId)
        where
          DE: SequenceEncoder,
          TE: SequenceEncoder,
          PE: SequenceEncoder,
    {
        assert!(self.borrow().postings.is_some());
        // println!("Writing postings of size {}", self.postings_len());
        if let Some(ref mut postings) = self.borrow_mut().postings {
            debug_assert!(is_sorted_ascending(&postings.docs));

            // println!("OLD TFS: {:?}", &postings.tfs);
            let mut cum = 0;
            for ptr in &mut postings.tfs {
                let tf = *ptr;
                let positions = delta_encode(&postings.positions[cum as usize .. (cum + tf) as usize]);
                // println!("positions: {:?}", &postings.positions);
                // println!("CUM: {}, TF:{}\n---\n", cum, tf);
                let _ = enc.positions.write_sequence((&positions).to_sequence()).unwrap();

                *ptr = cum;
                cum += tf;
            }

            let _ = enc.docs.write_sequence((&postings.docs).to_sequence()).unwrap();
            let mut seq = Vec::with_capacity(postings.tfs.len());
            for cumtf in &postings.tfs {
                seq.push(*last_tf + cumtf);
            }
            let _ = enc.tfs.write_sequence((&seq).to_sequence()).unwrap();
            postings.tfs.push(cum);
            // println!("NEW TFS: {:?}", &postings.tfs);

            *postings_ptr += postings.docs.len() as DocId;
            *last_tf += cum;
        }
    }


    fn flush<W, DE, TE, PE>(&self, parent: &Self, dict_ptr: &mut usize, postings_ptr: &mut DocId, last_tf: &mut DocId,
                            dict_out: &mut W, enc: &mut PostingsEncoders<DE, TE, PE>)
        where W: Write,
              DE: SequenceEncoder,
              TE: SequenceEncoder,
              PE: SequenceEncoder
    {
        // println!("flushing node with {} children: term: '{}'", self_borrow.children.len(), self.term());
        if self.borrow().children.len() > 0 {
            let merged_postings = {
                let selfb = self.borrow();
                assert!(selfb.children.len() <= u32::max_value() as usize);

                // Need to store actual borrows first
                let borrows = selfb.children.iter().map(|p| { p.borrow() }).collect::<Vec<_>>();

                let mut postings_to_merge = Vec::new();
                for child in &borrows {
                    if let Some(ref p) = child.postings {
                        postings_to_merge.push(RawCursor::new(Postings {
                            docs: (&p.docs).to_sequence(),
                            tfs: (&p.tfs).to_sequence(),
                            positions: (&p.positions).to_sequence(),
                        }));
                    }
                }
                MergerWithoutDuplicatesUnrolled::new(postings_to_merge).collect()
            };
            self.borrow_mut().postings = Some(merged_postings);
        }

        let dict_position = *dict_ptr;
        let prefix = &parent.borrow().t.term;

        // NOTE aligning is not needed when Header, child index and child pointers are aligned
        // to repr(C) (autoalign)
        let header = TrieNodeHeader::from_trienode(TrieNode(self.0.clone()), prefix, *postings_ptr);
        *dict_ptr += dict_out.write(header.to_bytes()).unwrap();

        if self.borrow().children.len() > 0 {
            // TODO assert that children_index and child_pointers are in ascending order
            let children_index = self.create_child_index();
            let child_pointers = self.create_child_pointers();

            *dict_ptr += dict_out.write(typed_to_bytes(&children_index)).unwrap();
            *dict_ptr += dict_out.write(typed_to_bytes(&child_pointers)).unwrap();
            *dict_ptr += dict_out.write(&[0,8][..align_to(*dict_ptr, mem::align_of::<TrieNodeHeader>())]).unwrap();
        }

        // Root node won't be written
        if self.term_id() != 0 {
            assert!(self.postings_len() > 0);
            self.write_postings(enc, postings_ptr, last_tf);
        }

        // Because root node won't be written, remove all postings from 1-character term prefixes.
        // They will not be needed anymore. This will save some memory during indexing
        let termlen = self.borrow().t.term.chars().count();
        if self.borrow().children.len() > 0 && termlen == 1 {
            self.borrow_mut().postings = None;
        }

        self.borrow_mut().children.clear();
        self.borrow_mut().pointer_in_dictbuf = Some(dict_position);
    }
}

impl Clone for TrieNode {
    fn clone(&self) -> TrieNode {
        TrieNode(self.0.clone())
    }
}

impl Deref for TrieNode {
    type Target = TrieNodeRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


impl TrieNodeHeader {
    fn to_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, mem::size_of::<TrieNodeHeader>()) }
    }

    fn from_trienode(n: TrieNode, prefix: &str, postings_ptr: DocId) -> TrieNodeHeader {
        let term = &n.borrow().t.term[prefix.len()..];
        // TODO Handle longer strings by truncating
        assert!(term.len() < u16::max_value() as usize);
        assert!(n.postings_len() > 0);

        TrieNodeHeader {
            postings_ptr: postings_ptr,
            term_ptr: (n.term_ptr() + prefix.len()) as u32,
            term_id: n.borrow().t.term_id,
            term_length: term.len() as u16,
            num_postings: n.postings_len() as u64,
            num_children: n.borrow().children.len() as u32,
        }
    }
}
