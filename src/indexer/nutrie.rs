use std::u8;
use std::str;
use std::ptr;
use std::slice;
use std::io::Write;
use std::cmp;
use std::rc::{Rc,Weak};
use std::cell::{RefCell,Ref,RefMut};
use std::mem;
use std::ops::Deref;

use indexer::*;

pub fn get_common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars())
        .take_while(|&(ac, bc)| { ac == bc })
        .fold(0, |acc, (x, _)| acc + x.len_utf8())
}

fn typed_to_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe {
        slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * mem::size_of::<T>())
    }
}

fn bytes_to_typed<T>(buf: &[u8]) -> &[T] {
    unsafe {
        slice::from_raw_parts(buf.as_ptr() as *const T, buf.len() / mem::size_of::<T>())
    }
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
    let mut toast = Vec::<u8>::new();

    // Create 2 dummy roots - because you need 2 node pointers - parent and current
    let root_term = Term{term: "".into(), term_id: 0};
    let root1 = TrieNode::new(None, &root_term, true, None);
    let root2 = TrieNode::new(Some(root1.clone()), &root_term, true, None);
    root1.clone().add_child(root2.clone());

    let mut parent: TrieNode = root1.clone();
    let mut current: TrieNode = root2.clone();

    let mut dict_ptr = 0;
    let mut postings_ptr = 0;

    for current_term in terms {
        let prefix_len = get_common_prefix_len(&current.borrow().t.term, &current_term.term);
        let child_postings = postings_store.get_postings(current_term.term_id);

        println!("IT {} {} {}", current.borrow().t.term, current_term.term, prefix_len);

        // align parent and current pointers
        while prefix_len < parent.term_len() {
            current.flush(&parent, &mut toast, dict_ptr, postings_ptr, dict_out, docs_out, tfs_out, pos_out);
            current = parent.clone();
            parent = parent.parent().unwrap();
        }

        if prefix_len >= current.term_len() {
            parent = current.clone();
        } else if prefix_len == parent.term_len() {
            current.flush(&parent, &mut toast, dict_ptr, postings_ptr, dict_out, docs_out, tfs_out, pos_out);
        } else if prefix_len > parent.term_len() {
            term_serial += 1;
            let new_term: *const Term = {
                let last_term = &current.borrow().t.term;
                allocate_term(&mut new_terms, Term {
                    term: last_term[..cmp::min(last_term.len(), prefix_len)].into(),
                    term_id: term_serial,
                })
            };

            let fork_node = {
                let _current_borrow = current.borrow();
                let ref child2_postings = _current_borrow.postings.as_ref().unwrap();
                let postings_to_merge = vec![child_postings.as_ref().unwrap(), child2_postings];
                TrieNode::new(
                    current.parent(),
                    // UNSAFE: new_term will be alive, because 'new_terms' arena will be dropped
                    // only after the trie is finished
                    unsafe { &*new_term },
                    true,
                    Some(Postings::merge(&postings_to_merge[..])),
                )
            };

            // Flush with fork_node as a new parent
            current.flush(&fork_node, &mut toast, dict_ptr, postings_ptr, dict_out, docs_out, tfs_out, pos_out);

            parent = current.clone();
            current = fork_node.clone();
            mem::swap(&mut *current.borrow_mut(), &mut *parent.borrow_mut());

            current.set_parent(&parent);
            parent.add_child(fork_node);
        }

        let parent_clone = parent.clone();
        current = parent.add_child(TrieNode::new(
            Some(parent_clone),
            current_term,
            false,
            child_postings,
        ));
    }

    while current.borrow().t.term_id != 0 {
        current.flush(&parent, &mut toast, dict_ptr, postings_ptr, dict_out, docs_out, tfs_out, pos_out);
        current = parent.clone();
        parent = parent.parent().unwrap();
    }

    // TODO get current dict length. This size - sizeof(trienodeheader) = ptr to root node
    dict_out.write(&toast).unwrap();

    TrieResult {
        // unbox terms
        new_terms: new_terms.into_iter().map(|t| *t).collect(),
        written_dict_positions: written_positions,
    }
}


const HEADER_SIZE: usize = 11; // size without 'term'
const ALIGNED_SIZE: usize = 24;
const TERM_AVAILABLE_SIZE: usize = ALIGNED_SIZE - HEADER_SIZE;
const TERM_POINTER_SIZE: usize = 8;

#[repr(packed)]
struct TrieNodeHeader {
    postings_ptr: u32, // DOCID
    term_id: u32, // TERMID
    term_length: u8,
    num_children: u8,
    is_prefix: bool,
    term: [u8; TERM_AVAILABLE_SIZE],
}

impl TrieNodeHeader {
    fn from_bytes<'a>(bs: *const u8) -> &'a TrieNodeHeader {
        unsafe { mem::transmute(bs) }
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

    fn get_children_index(&self) -> &[u8] {
        unsafe { slice::from_raw_parts((self as *const Self).offset(1) as *const u8, self.num_children as usize) }
    }

    fn get_child_pointers(&self) -> &[u32] {
        unsafe {
            let children_index = (self as *const Self).offset(1) as *const u8;
            let child_pointers = children_index.offset(self.num_children as isize) as *const u32;
            slice::from_raw_parts(child_pointers, self.num_children as usize)
        }
    }

    // TODO self == root
    fn find_term(&self, dictbuf: &[u8], toast: &[u8], find_nearest: bool, term: &str) -> Option<&Self> {
        let mut cursor = self;
        let mut cur_term = term;
        loop {
            let current_term = cursor.term(toast);
            let skip = get_common_prefix_len(current_term, cur_term);
            if skip < cur_term.len() {
                if find_nearest {
                    return Some(&cursor);
                } else {
                    return None;
                }
            } else if skip > cur_term.len() {
                cur_term = &cur_term[skip..];
                let children_index = cursor.get_children_index();
                let first_letter = term.as_bytes()[0];
                let child_index = match children_index.binary_search(&first_letter) {
                    Ok(index) => index,
                    Err(_) => return None,
                };
                let child_pointer = cursor.get_child_pointers()[child_index];
                cursor = Self::from_bytes((&dictbuf[child_pointer as usize ..]).as_ptr());
            } else {
                return Some(&cursor);
            }
        }
    }

    fn from_trienode<'n>(n: &TrieNodeRef<'n>, prefix_len: usize, postings_ptr: u32, toast: &mut Vec<u8>) -> TrieNodeHeader {
        let nb = n.borrow();
        let mut term_bytes = [0u8; TERM_AVAILABLE_SIZE];
        let term = &nb.t.term[prefix_len..];
        let term_len = term.len();

        // Handle longer strings by truncating
        assert!(term_len < u8::max_value() as usize);


        if term_len > TERM_AVAILABLE_SIZE {
            println!("using toast! for {}|{}", &nb.t.term[..prefix_len], term);
            unsafe {
                ptr::copy_nonoverlapping(mem::transmute(&toast.len()), &mut term_bytes[TERM_AVAILABLE_SIZE - TERM_POINTER_SIZE], TERM_POINTER_SIZE);
            }
            toast.extend(term.as_bytes());
        } else {
            term_bytes[..term_len].copy_from_slice(term.as_bytes());
        }

        TrieNodeHeader {
            postings_ptr: postings_ptr,
            term_id: nb.t.term_id,
            term_length: term_len as u8,
            num_children: nb.children.len() as u8,
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
    pointer_in_dictbuf: Option<usize>,
    postings: Option<Postings>,
    parent: Option<TrieNodeWeak<'n>>,
    children: Vec<TrieNodeRef<'n>>,
}

impl<'n> TrieNode<'n> {
    fn new(parent: Option<TrieNode<'n>>, t: &'n Term, is_prefix: bool, postings: Option<Postings>) -> TrieNode<'n> {
        TrieNode(Rc::new(RefCell::new(_TrieNode {
            t: t,
            is_prefix: is_prefix,
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

    fn term_len(&self) -> usize {
        self.borrow().t.term.len()
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

    fn create_child_pointers(&self) -> Vec<usize> {
        self.borrow().children.iter().map(|ch| {
            ch.borrow().pointer_in_dictbuf.expect("This node must be written by now")
        }).collect()
    }

    fn create_child_index(&self, prefix: &str) -> Vec<u8> {
        self.borrow().children.iter().map(|ch| {
            let ch_borrow = ch.borrow();
            let suffix = &ch_borrow.t.term[prefix.len()..];
            assert!(suffix.len() > 0);
            suffix.as_bytes()[0] // output first letter (byte) after parent prefix
        }).collect()
    }

    fn flush<W: Write>(&self, parent: &Self, toast: &mut Vec<u8>, mut dict_ptr: usize, mut postings_ptr: u32, dict_out: &mut W, docs_out: &mut W, tfs_out: &mut W, pos_out: &mut W) {
        let dict_position = dict_ptr;
        let maybe_merged = {
            let self_borrow = self.borrow();
            let parent_borrow = parent.borrow();
            let prefix = &parent_borrow.t.term;

            let header = TrieNodeHeader::from_trienode(&self, prefix.len(), postings_ptr, toast);
            dict_ptr += dict_out.write(header.to_bytes()).unwrap();

            if self_borrow.children.len() > 0 {
                let child_pointers = self.create_child_pointers();
                let children_index = self.create_child_index(prefix);

                dict_out.write(&children_index[..]).unwrap();
                dict_out.write(typed_to_bytes(&children_index[..])).unwrap();

                // Need to store actual borrows first
                let borrows = self_borrow.children.iter().map(|p| { p.borrow() }).collect::<Vec<_>>();
                let postings_to_merge = borrows.iter().map(|p| { p.postings.as_ref().unwrap() }).collect::<Vec<_>>();
                Some(Postings::merge(&postings_to_merge[..]))
            } else {
                // else this node is a leaf
                None
            }
        };
        // TODO assert children are sorted

        {
            if let Some(postings) = maybe_merged {
                self.borrow_mut().postings = Some(postings);
            }
            self.borrow_mut().children.clear();
            self.borrow_mut().pointer_in_dictbuf = Some(dict_position);
        }

        let self_borrow = self.borrow();
        let postings = self_borrow.postings.as_ref().unwrap();
        postings_ptr += docs_out.write(typed_to_bytes(&postings.docs)).unwrap() as u32;
        tfs_out.write(typed_to_bytes(&postings.tfs)).unwrap();
        pos_out.write(typed_to_bytes(&postings.positions)).unwrap();
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
