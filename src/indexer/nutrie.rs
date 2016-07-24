use std::u8;
use std::str;
use std::ptr;
use std::slice;
use std::io::Write;
use std::cmp;
use std::rc::{Rc,Weak};
use std::cell::{RefCell,Ref,RefMut};
use std::mem;

use indexer::*;

pub fn get_common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars())
        .take_while(|&(ac, bc)| { ac == bc })
        .fold(0, |acc, (x, _)| acc + x.len_utf8())
}

fn allocate_term(arena: &mut Vec<Box<Term>>, term: Term) -> *const Term {
    let boxed_term = Box::new(term);
    let handle = &*boxed_term as *const Term;
    arena.push(boxed_term);
    handle
}

pub struct TrieResult {
    pub new_terms: Vec<Term>,
    pub written_dict_positions: Vec<(TermId, usize)>,
}

pub fn create_trie<'a, I, PS, W>(
    mut term_serial: TermId,
    terms: I,
    postings_store: &mut PS,
    dict_out: &mut W,
    docs_out: &mut W,
    tfs_out:  &mut W,
    pos_out:  &mut W,
) -> TrieResult
    where I: Iterator<Item=&'a Term>,
          PS: PostingsStore,
          W: Write
{
    let mut written_positions = Vec::new();
    let mut new_terms = Vec::<Box<Term>>::new();

    let root_term = Term{term: "".into(), term_id: 0};
    let root = TrieNode::new(None, &root_term, true, None);

    let mut last_node: TrieNode = root.clone();
    let mut parent: TrieNode;

    for current_term in terms {
        let prefix_len = get_common_prefix_len(&last_node.borrow().t.term, &current_term.term);

        // println!("IT {} {} {}", last_node.borrow().t.term, current_term.term, prefix_len);

        if prefix_len >= last_node.borrow().t.term.len() {
            parent = last_node;
            let parent_clone = parent.clone();
            last_node = parent.add_child(TrieNode::new(
                Some(parent_clone),
                current_term,
                false,
                postings_store.get_postings(current_term.term_id),
            ));
            continue;
        }

        while prefix_len < last_node.parent_term_len() {
            last_node = last_node.parent().unwrap();
            last_node.flush(dict_out, docs_out, tfs_out, pos_out);
        }

        let child_postings = postings_store.get_postings(current_term.term_id).unwrap();

        if prefix_len == last_node.parent_term_len() {
            parent = last_node.parent().unwrap();
        } else {
            term_serial += 1;
            let new_term: *const Term = {
                let last_term = &last_node.borrow().t.term;
                allocate_term(&mut new_terms, Term {
                    term: last_term[..cmp::min(last_term.len(), prefix_len)].into(),
                    term_id: term_serial,
                })
            };

            let new_node = {
                let _last_node_borrow = last_node.borrow();
                let ref child2_postings = _last_node_borrow.postings.as_ref().unwrap();
                let postings_to_merge = vec![&child_postings, child2_postings];
                TrieNode::new(
                    last_node.parent(),
                    // UNSAFE: new_term will be alive, because 'new_terms' arena will be dropped
                    // only after the trie is finished
                    unsafe { &*new_term },
                    true,
                    Some(Postings::merge(&postings_to_merge[..])),
                )
            };

            parent = last_node;
            last_node = new_node.clone();
            mem::swap(&mut *last_node.borrow_mut(), &mut *parent.borrow_mut());

            (&mut *last_node.borrow_mut()).parent = Some(Rc::downgrade(&parent.0));
            // *(last_node.0.borrow().parent.unwrap().upgrade().unwrap().borrow_mut()) = Some(parent);
            parent.0.borrow_mut().children.push(new_node.0);
        }

        let parent_clone = parent.clone();
        last_node = parent.add_child(TrieNode::new(
            Some(parent_clone),
            current_term,
            false,
            Some(child_postings)
        ));
    }

    while let Some(parent) = last_node.parent() {
        last_node = parent;
        last_node.flush(dict_out, docs_out, tfs_out, pos_out);
    }

    TrieResult {
        // unbox terms
        new_terms: new_terms.into_iter().map(|t| *t).collect(),
        written_dict_positions: written_positions,
    }
}


const HEADER_SIZE: usize = 10; // size without 'term'
const ALIGNED_SIZE: usize = 24;
const TERM_AVAILABLE_SIZE: usize = ALIGNED_SIZE - HEADER_SIZE;
const TERM_POINTER_SIZE: usize = 8;

#[repr(packed)]
struct TrieNodeHeader {
    postings_ptr: u32, // DOCID
    term_id: u32, // TERMID
    term_length: u8,
    is_prefix: bool,
    term: [u8; TERM_AVAILABLE_SIZE],
}

impl TrieNodeHeader {
    fn from_bytes<'a>(bs: &'a [u8]) -> &'a TrieNodeHeader {
        unsafe { mem::transmute(&bs[0]) }
    }

    fn to_bytes(&self) -> &[u8] {
        unsafe {
            let self_ptr: *const u8 = mem::transmute(self);
            slice::from_raw_parts(self_ptr, mem::size_of::<TrieNodeHeader>())
        }
    }

    #[allow(dead_code)]
    fn term<'a>(&self, toast: &'a [u8]) -> &'a str {
        unsafe {
            let slice = if self.term_length as usize > TERM_POINTER_SIZE {
                let p: &usize = mem::transmute(&self.term[TERM_AVAILABLE_SIZE - TERM_POINTER_SIZE]);
                slice::from_raw_parts(&toast[*p] as *const u8, self.term_length as usize)
            } else {
                slice::from_raw_parts(mem::transmute(&self.term), self.term_length as usize)
            };
            str::from_utf8_unchecked(slice)
        }
    }

    fn from_trienode<'n>(n: &TrieNodeRef<'n>, prefix_len: usize, postings_ptr: u32, mut toast_ptr: usize, toast: &mut Vec<u8>) -> TrieNodeHeader {
        let nb = n.borrow();
        let mut term_bytes = [0u8; TERM_AVAILABLE_SIZE];
        let term = &nb.t.term[prefix_len..];
        let term_len = term.len();

        // Handle longer strings by truncating
        assert!(term_len < u8::max_value() as usize);


        if term_len > TERM_AVAILABLE_SIZE {
            println!("using toast! for {}|{}", &nb.t.term[..prefix_len], term);
            toast.extend(term.as_bytes());
            unsafe {
                ptr::copy_nonoverlapping(mem::transmute(&toast_ptr), &mut term_bytes[TERM_AVAILABLE_SIZE - TERM_POINTER_SIZE], TERM_POINTER_SIZE);
            }
            toast_ptr += term_len;
        } else {
            term_bytes[..term_len].copy_from_slice(term.as_bytes());
        }

        TrieNodeHeader {
            postings_ptr: postings_ptr,
            term_id: nb.t.term_id,
            term_length: term_len as u8,
            is_prefix: nb.is_prefix,
            term: term_bytes,
        }
    }

}

// 't: 'n means that terms ('t) can live longer than nodes ('n) It is needed so that root term can
// be allocated in shorter lifetime than that of other terms.  No other reason
type TrieNodeRef<'n> = Rc<RefCell<_TrieNode<'n>>>;
type TrieNodeWeak<'n> = Weak<RefCell<_TrieNode<'n>>>;
struct TrieNode<'n>(TrieNodeRef<'n>);
struct _TrieNode<'n> {
    t: &'n Term,
    is_prefix: bool,
    postings: Option<Postings>,
    parent: Option<TrieNodeWeak<'n>>,
    children: Vec<TrieNodeRef<'n>>,
}

impl<'n> TrieNode<'n> {
    fn new(parent: Option<TrieNode<'n>>, t: &'n Term, is_prefix: bool, postings: Option<Postings>) -> TrieNode<'n> {
        TrieNode(Rc::new(RefCell::new(_TrieNode {
            t: t,
            is_prefix: is_prefix,
            postings: postings,
            parent: parent.map(|p| Rc::downgrade(&p.0.clone())),
            children: Vec::new(),
        })))
    }

    fn parent(&self) -> Option<TrieNode<'n>> {
        match self.0.borrow().parent {
            Some(ref weak_link) => Some(TrieNode(weak_link.upgrade().unwrap())),
            None => None,
        }
    }

    fn parent_term_len(&self) -> usize {
        let parent = self.parent().unwrap();
        let pb = parent.borrow();
        pb.t.term.len()
    }

    fn add_child(&mut self, child: TrieNode<'n>) -> TrieNode<'n> {
        let borrow = child.0.clone();
        self.0.borrow_mut().children.push(child.0);
        TrieNode(borrow)
    }

    fn borrow(&self) -> Ref<_TrieNode<'n>> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> RefMut<_TrieNode<'n>> {
        self.0.borrow_mut()
    }

    fn flush<W: Write>(&self, dict_out: &mut W, docs_out: &mut W, tfs_out: &mut W, pos_out: &mut W) {
        {
            let self_borrow = self.borrow();
            let prefix = &self_borrow.t.term;
            for child in self_borrow.children.iter() {
                // TODO flush
                let child_term = &child.borrow().t.term;
                let suffix = &child_term[prefix.len()..];
                //println!("Flushing node {}|{}, term: {}", prefix, suffix, child_term);

                let mut toast_tmp = Vec::new();
                let header = TrieNodeHeader::from_trienode(child, prefix.len(), 0, 0, &mut toast_tmp);
                dict_out.write(header.to_bytes()).unwrap();

                // TODO enable this
                if let Some(ref postings) = child.borrow().postings {
                    docs_out.write(unsafe {mem::transmute(&postings.docs[..])}).unwrap();
                    tfs_out.write(unsafe {mem::transmute(&postings.tfs[..])}).unwrap();
                    pos_out.write(unsafe {mem::transmute(&postings.positions[..])}).unwrap();
                }
            }

            // Need to store actual borrows first
            let borrows = self_borrow.children.iter().map(|p| { p.borrow() }).collect::<Vec<_>>();
            let postings_to_merge = borrows.iter().map(|p| { p.postings.as_ref().unwrap() }).collect::<Vec<_>>();
            let prefix_postings = Postings::merge(&postings_to_merge[..]);
        }
        // TODO assert children are sorted

        self.borrow_mut().children.clear();
    }
}

impl<'n> Clone for TrieNode<'n> {
    fn clone(&self) -> TrieNode<'n> {
        TrieNode(self.0.clone())
    }
}
