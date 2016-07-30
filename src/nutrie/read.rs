use std::{mem,slice,str};
use std::io::Read;
use nutrie::TrieNodeHeader;
use util::*;

impl TrieNodeHeader {
    fn from_bytes<'a>(bs: *const u8) -> &'a TrieNodeHeader {
        unsafe { mem::transmute(bs) }
    }

    fn term<'a>(&self, term_buffer: &'a [u8]) -> &'a str {
        unsafe {
            let slice = slice::from_raw_parts(&term_buffer[self.term_ptr as usize] as *const u8, self.term_length as usize);
            str::from_utf8_unchecked(slice)
        }
    }

    fn get_children_index(&self) -> &[u32] {
        unsafe {
            let index_ptr = (self as *const Self).offset(1) as *const u32;
            slice::from_raw_parts(index_ptr, self.num_children as usize)
        }
    }

    fn get_child_pointers(&self) -> &[u32] {
        unsafe {
            let children_index  = (self as *const Self).offset(1) as *const u32;
            let child_pointers = children_index.offset(self.num_children as isize);
            slice::from_raw_parts(child_pointers, self.num_children as usize)
        }
    }
}

pub struct StaticTrie<'a> {
    root: &'a TrieNodeHeader,
    trie_buffer: &'a [u8],
    term_buffer: &'a [u8],
}

impl<'a> StaticTrie<'a> {
    pub fn read<R: Read>(reader: &mut R) -> Vec<u8> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        bytes
    }

    pub fn new(bytes: &'a [u8], dict_size: usize, root_ptr: usize, terms_size: usize) -> Self {
        let (trie, terms) = bytes.split_at(dict_size);
        StaticTrie {
            root: TrieNodeHeader::from_bytes(&trie[root_ptr] as *const _),
            trie_buffer: trie,
            term_buffer: terms,
        }
    }

    pub fn find_term(&self, mut term: &str, find_nearest: bool) -> Option<&TrieNodeHeader> {
        let mut cursor = self.root;
        loop {
            let current_term = cursor.term(self.term_buffer);
            // println!("looking for: '{}', cursor term: '{}', len: {}", term, current_term, cursor.term_length);
            // println!("CURSOR: {:?}", cursor);
            let skip = common_prefix_len(current_term, term);
            if skip < term.len() {
                term = &term[skip..];
                let first_letter = first_letter(term);
                let children_index = cursor.get_children_index();
                let child_index = match children_index.binary_search(&first_letter) {
                    Ok(index) => index,
                    Err(_) => return None,
                };
                let child_pointer = cursor.get_child_pointers()[child_index] as usize;
                // println!("child index: {:?}", children_index);
                let bufslice = &self.trie_buffer[child_pointer..];
                cursor = TrieNodeHeader::from_bytes(bufslice.as_ptr());
            } else if skip > term.len() {
                if find_nearest {
                    return Some(&cursor);
                } else {
                    return None;
                }
            } else {
                return Some(&cursor);
            }
            // println!("");
        }
    }
}
