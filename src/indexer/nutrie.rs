use std::u8;
use std::str;
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

// pub struct TrieResult {
//     pub new_terms: Vec<Term>,
//     pub written_dict_positions: Vec<(TermId, usize)>,
// }

pub fn create_trie<PS, W>(
    mut term_serial: TermId,
    terms: &[Term],
    postings_store: &mut PS,
    dict_out: &mut W,
    docs_out: &mut W,
    tfs_out:  &mut W,
    pos_out:  &mut W,
)
    where PS: PostingsStore,
          W: Write
{

    // Create 2 dummy roots - because you need 2 node pointers - parent and current
    let root1 = TrieNode::new(None, "", 0, None, true, None);
    let root2 = TrieNode::new(Some(root1.clone()), "", 0, None, true, None);
    root1.clone().add_child(root2.clone());

    let mut parent: TrieNode = root1.clone();
    let mut current: TrieNode = root2.clone();

    let mut term_ptr = 0;
    let mut dict_ptr = 0;
    let mut postings_ptr = 0;

    for current_term in terms.iter() {
        let prefix_len = get_common_prefix_len(&current.borrow().term, &current_term.term);
        let child_postings = postings_store.get_postings(current_term.term_id);

        println!("IT {} {} {}", current.borrow().term, current_term.term, prefix_len);

        // align parent and current pointers
        while prefix_len < parent.term_len() {
            current.flush(&parent, &mut dict_ptr, &mut postings_ptr, dict_out, docs_out, tfs_out, pos_out);
            current = parent.clone();
            parent = parent.parent().unwrap();
        }

        if prefix_len >= current.term_len() {
            parent = current.clone();
        } else if prefix_len == parent.term_len() {
            current.flush(&parent, &mut dict_ptr, &mut postings_ptr, dict_out, docs_out, tfs_out, pos_out);
        } else if prefix_len > parent.term_len() {
            let parent_term_ptr = current.borrow().term_ptr.expect("Prefix must be created from existing term");
            let new_term = {
                let last_term = &current.borrow().term;
                &last_term[.. cmp::min(last_term.len(), prefix_len)]
            };

            term_serial += 1;
            let fork_node = {
                let _current_borrow = current.borrow();
                let ref child2_postings = _current_borrow.postings.as_ref().unwrap();
                let postings_to_merge = vec![child_postings.as_ref().unwrap(), child2_postings];
                TrieNode::new(
                    current.parent(),
                    new_term, term_serial, Some(parent_term_ptr), true,
                    Some(Postings::merge(&postings_to_merge[..])),
                )
            };

            // Flush with fork_node as a new parent
            current.flush(&fork_node, &mut dict_ptr, &mut postings_ptr, dict_out, docs_out, tfs_out, pos_out);

            parent = current.clone();
            current = fork_node.clone();
            mem::swap(&mut *current.borrow_mut(), &mut *parent.borrow_mut());

            current.set_parent(&parent);
            parent.add_child(fork_node);
        }

        let parent_clone = parent.clone();
        current = parent.add_child(TrieNode::new(
            Some(parent_clone),
            &current_term.term, current_term.term_id, Some(term_ptr), false,
            child_postings,
        ));
        term_ptr += current_term.term.len();
    }

    while current.borrow().term_id != 0 {
        current.flush(&parent, &mut dict_ptr, &mut postings_ptr, dict_out, docs_out, tfs_out, pos_out);
        current = parent.clone();
        parent = parent.parent().unwrap();
    }

    for t in terms.iter() {
        dict_out.write(&t.term.as_bytes()).unwrap();
    }
}


// TODO packed necessary?
#[repr(packed)]
struct TrieNodeHeader {
    postings_ptr: u32, // DOCID
    term_ptr: u32,
    term_id: u32, // TERMID
    term_length: u8,
    num_children: u8,
    is_prefix: bool,
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

    fn term<'a>(&self, term_buffer: &'a [u8]) -> &'a str {
        unsafe {
            let slice = slice::from_raw_parts(&term_buffer[self.term_ptr as usize] as *const u8, self.term_length as usize);
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
    fn find_term(&self, dictbuf: &[u8], term_buffer: &[u8], find_nearest: bool, term: &str) -> Option<&Self> {
        let mut cursor = self;
        let mut cur_term = term;
        loop {
            let current_term = cursor.term(term_buffer);
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

    fn from_trienode<'n>(n: &TrieNodeRef<'n>, prefix_len: usize, postings_ptr: u32) -> TrieNodeHeader {
        let nb = n.borrow();
        let term = &nb.term[prefix_len..];
        let term_ptr = nb.term_ptr.unwrap() + prefix_len;

        // TODO Handle longer strings by truncating
        assert!(term.len() < u8::max_value() as usize);

        TrieNodeHeader {
            postings_ptr: postings_ptr,
            term_ptr: term_ptr as u32,
            term_id: nb.term_id,
            term_length: term.len() as u8,
            num_children: nb.children.len() as u8,
            is_prefix: nb.is_prefix,
        }
    }

}

// 't: 'n means that terms ('t) can live longer than nodes ('n) It is needed so that root term can
// be allocated in shorter lifetime than that of other terms.  No other reason
type TrieNodeRef<'n> = Rc<RefCell<_TrieNode<'n>>>;
type TrieNodeWeak<'n> = Weak<RefCell<_TrieNode<'n>>>;
struct TrieNode<'n>(TrieNodeRef<'n>);
struct _TrieNode<'n> {
    term: &'n str,
    term_id: TermId,
    term_ptr: Option<usize>,
    is_prefix: bool,
    pointer_in_dictbuf: Option<usize>,
    postings: Option<Postings>,
    parent: Option<TrieNodeWeak<'n>>,
    children: Vec<TrieNodeRef<'n>>,
}

impl<'n> TrieNode<'n> {
    fn new(parent: Option<TrieNode<'n>>, term: &'n str, term_id: TermId, term_ptr: Option<usize>, is_prefix: bool, postings: Option<Postings>) -> TrieNode<'n> {
        TrieNode(Rc::new(RefCell::new(_TrieNode {
            term: term,
            term_id: term_id,
            term_ptr: term_ptr,
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
        self.borrow().term.len()
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
            let suffix = &ch_borrow.term[prefix.len()..];
            assert!(suffix.len() > 0);
            suffix.as_bytes()[0] // output first letter (byte) after parent prefix
        }).collect()
    }

    fn flush<W: Write>(&self, parent: &Self, dict_ptr: &mut usize, postings_ptr: &mut u32, dict_out: &mut W, docs_out: &mut W, tfs_out: &mut W, pos_out: &mut W) {
        let dict_position = *dict_ptr;
        let maybe_merged = {
            let self_borrow = self.borrow();
            let parent_borrow = parent.borrow();
            let prefix = &parent_borrow.term;

            let header = TrieNodeHeader::from_trienode(&self, prefix.len(), *postings_ptr);
            *dict_ptr += dict_out.write(header.to_bytes()).unwrap();

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
        *postings_ptr += docs_out.write(typed_to_bytes(&postings.docs)).unwrap() as u32;
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
