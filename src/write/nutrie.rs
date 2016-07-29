use std::{str,slice,cmp,mem};
use std::io::{Write};
use std::rc::{Rc,Weak};
use std::cell::{RefCell,Ref,RefMut};
use std::ops::Deref;

use write::postings::{Postings,PostingsStore};
use types::{DocId,TermId,Term,TrieNodeHeader};
use types::{SequenceEncoder,SequenceStorage};
use ::util::*;


#[derive(Clone)]
pub struct WrittenTerm<'a> {
    term: &'a str,
    term_ptr: usize,
    term_id: TermId,
}

impl<'a> WrittenTerm<'a> {
    fn new(term: &'a str, term_id: TermId, term_ptr: usize) -> WrittenTerm<'a> {
        WrittenTerm {
            term: term,
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

pub fn create_trie<'a, PS, W, DE, TE, PE>(mut term_serial: TermId, terms: &'a [Term], postings_store: &mut PS,
                                          dict_out: &mut W, enc: &mut PostingsEncoders<DE, TE, PE>)
    -> (Vec<WrittenTerm<'a>>, usize, usize, usize)
    where PS: PostingsStore,
          W: Write,
          DE: SequenceEncoder,
          TE: SequenceEncoder,
          PE: SequenceEncoder
{
    let mut new_terms = Vec::<WrittenTerm>::new();

    // Create 2 dummy roots - because you need 2 node pointers - parent and current
    let root_term = WrittenTerm::new("", 0, 0);
    let root1 = TrieNode::new(None, root_term.clone(), false, None);
    let root2 = TrieNode::new(Some(root1.clone()), root_term, false, None);
    root1.clone().add_child(root2.clone());

    let mut parent: TrieNode = root1.clone();
    let mut current: TrieNode = root2.clone();

    let mut term_ptr = 0;
    let mut dict_ptr = 0;
    let mut postings_ptr = 0;

    for &Term{ref term, term_id} in terms.iter() {
        let prefix_len = common_prefix_len(current.term(), term);
        let child_postings = postings_store.get_postings(term_id);

        // println!("IT {} {} {}", current.term(), term, prefix_len);

        // align parent and current pointers
        while prefix_len < parent.term().len() {
            current.flush(&parent, &mut dict_ptr, &mut postings_ptr, dict_out, enc);
            current = parent.clone();
            parent = parent.parent().unwrap();
        }

        if prefix_len >= current.term().len() {
            parent = current.clone();
        } else if prefix_len == parent.term().len() {
            current.flush(&parent, &mut dict_ptr, &mut postings_ptr, dict_out, enc);
        } else if prefix_len > parent.term().len() {
            //let parent_term_ptr = current.borrow().term_ptr;
            term_serial += 1;
            let new_term = {
                let last_term = current.term();
                let nt = &last_term[.. cmp::min(last_term.len(), prefix_len)];
                WrittenTerm::new(nt, term_serial, current.term_ptr())
            };

            new_terms.push(new_term.clone());
            let fork_node = {
                let _current_borrow = current.borrow();
                let ref child2_postings = _current_borrow.postings.as_ref().unwrap();
                let postings_to_merge = vec![child_postings.as_ref().unwrap(), child2_postings];
                TrieNode::new(
                    current.parent(),
                    new_term, false,
                    Some(Postings::merge(&postings_to_merge[..])),
                )
            };

            // Flush with fork_node as a new parent
            current.flush(&fork_node, &mut dict_ptr, &mut postings_ptr, dict_out, enc);

            parent = current.clone();
            current = fork_node.clone();
            mem::swap(&mut *current.borrow_mut(), &mut *parent.borrow_mut());

            current.set_parent(&parent);
            parent.add_child(fork_node);
        }

        let new_term = WrittenTerm::new(term, term_id, term_ptr);
        new_terms.push(new_term.clone());
        term_ptr += term.len();

        let parent_clone = parent.clone();
        current = parent.add_child(TrieNode::new(
            Some(parent_clone),
            new_term, true,
            child_postings,
        ));
    }

    let mut root_ptr;
    loop {
        root_ptr = dict_ptr;
        current.flush(&parent, &mut dict_ptr, &mut postings_ptr, dict_out, enc);
        current = parent.clone();
        match parent.parent() {
            Some(node) => parent = node,
            None => break,
        };
    }
    assert!(current.parent().is_none());

    for t in terms.iter() {
        dict_out.write(&t.term.as_bytes()).unwrap();
    }

    (new_terms, dict_ptr, root_ptr, term_ptr)
}

// 't: 'n means that terms ('t) can live longer than nodes ('n) It is needed so that root term can
// be allocated in shorter lifetime than that of other terms.  No other reason
type TrieNodeRef<'n> = Rc<RefCell<_TrieNode<'n>>>;
type TrieNodeWeak<'n> = Weak<RefCell<_TrieNode<'n>>>;
struct TrieNode<'n>(TrieNodeRef<'n>);
struct _TrieNode<'n> {
    t: WrittenTerm<'n>,
    is_word: bool,
    pointer_in_dictbuf: Option<usize>,
    postings: Option<Postings>,
    parent: Option<TrieNodeWeak<'n>>,
    children: Vec<TrieNodeRef<'n>>,
}

impl<'n> TrieNode<'n> {
    fn new(parent: Option<TrieNode<'n>>, t: WrittenTerm<'n>, is_word: bool, postings: Option<Postings>) -> TrieNode<'n> {
        TrieNode(Rc::new(RefCell::new(_TrieNode {
            t: t,
            is_word: is_word,
            pointer_in_dictbuf: None,
            postings: postings,
            parent: parent.map(|p| Rc::downgrade(&p.clone())),
            children: Vec::new(),
        })))
    }

    fn set_parent(&mut self, parent: &TrieNodeRef<'n>) {
        self.borrow_mut().parent = Some(Rc::downgrade(parent));
        //(&mut *current.borrow_mut()).parent = Some(Rc::downgrade(&parent));
    }

    fn parent(&self) -> Option<TrieNode<'n>> {
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

    fn term(&self) -> &'n str {
        &self.borrow().t.term
    }

    fn add_child(&mut self, child: TrieNode<'n>) -> TrieNode<'n> {
        let borrow = child.clone();
        self.borrow_mut().children.push(child.0);
        TrieNode(borrow.0)
    }

    fn borrow(&self) -> Ref<_TrieNode<'n>> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> RefMut<_TrieNode<'n>> {
        self.0.borrow_mut()
    }

    fn create_child_pointers(&self) -> Vec<u32> {
        self.borrow().children.iter().map(|ch| {
            ch.borrow().pointer_in_dictbuf.expect("This node must be written by now") as u32
        }).collect()
    }

    fn create_child_index(&self) -> Vec<u32> {
        let prefix = self.term();
        self.borrow().children.iter().map(|ch| {
            let ch_borrow = ch.borrow();
            let suffix = &ch_borrow.t.term[prefix.len()..];
            // println!("prefix='{}',   term='{}', suffix='{}'", prefix, ch_borrow.t.term, suffix);
            assert!(suffix.len() > 0);
            first_letter(suffix)
        }).collect()
    }

    fn flush<W, DE, TE, PE>(&self, parent: &Self, dict_ptr: &mut usize, postings_ptr: &mut DocId, 
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
                let postings_to_merge = borrows.iter().map(|p| { p.postings.as_ref().unwrap() }).collect::<Vec<_>>();
                Postings::merge(&postings_to_merge[..])
            };
            self.borrow_mut().postings = Some(merged_postings);
        }

        let dict_position = *dict_ptr;
        let prefix = parent.term();

        // NOTE aligning is not needed when Header, child index and child pointers are aligned
        // to repr(C) (autoalign)
        let header = TrieNodeHeader::from_trienode(&self, prefix, *postings_ptr);
        *dict_ptr += dict_out.write(header.to_bytes()).unwrap();
        *postings_ptr += self.borrow().postings.as_ref().unwrap().docs.len() as DocId;

        if self.borrow().children.len() > 0 {
            // TODO assert that children_index and child_pointers are in ascending order
            let children_index = self.create_child_index();
            let child_pointers = self.create_child_pointers();

            *dict_ptr += dict_out.write(typed_to_bytes(&children_index)).unwrap();
            *dict_ptr += dict_out.write(typed_to_bytes(&child_pointers)).unwrap();
            *dict_ptr += dict_out.write(&[0,8][..align_to(*dict_ptr, mem::align_of::<TrieNodeHeader>())]).unwrap();
        }

        // if let Some(postings) = maybe_merged {
        //     self.borrow_mut().postings = Some(postings);
        // }
        self.borrow_mut().children.clear();
        self.borrow_mut().pointer_in_dictbuf = Some(dict_position);

        let self_borrow = self.borrow();
        let postings = self_borrow.postings.as_ref().unwrap();

        // assert!(is_sorted_ascending(&postings.docs));
        enc.docs.write_sequence((&postings.docs[..]).to_sequence()).unwrap();
        for &tf in &postings.tfs {
            enc.tfs.write(tf).unwrap();
        }
        for &position in &postings.positions {
            enc.positions.write(position).unwrap();
        }
    }
}

impl<'n> Clone for TrieNode<'n> {
    fn clone(&self) -> TrieNode<'n> {
        TrieNode(self.0.clone())
    }
}

impl<'n> Deref for TrieNode<'n> {
    type Target = TrieNodeRef<'n>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


impl TrieNodeHeader {
    fn to_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, mem::size_of::<TrieNodeHeader>()) }
    }

    fn from_trienode<'n>(n: &TrieNodeRef<'n>, prefix: &str, postings_ptr: DocId) -> TrieNodeHeader {
        let term = &n.borrow().t.term[prefix.len()..];
        let term_ptr = n.borrow().t.term_ptr + prefix.len();

        // TODO Handle longer strings by truncating
        assert!(term.len() < u16::max_value() as usize);

        TrieNodeHeader {
            postings_ptr: postings_ptr,
            term_ptr: term_ptr as u32,
            term_id: n.borrow().t.term_id,
            term_length: term.len() as u16,
            num_postings: n.borrow().postings.as_ref().unwrap().docs.len() as u64,
            num_children: n.borrow().children.len() as u32,
            is_word: n.borrow().is_word,
        }
    }
}
