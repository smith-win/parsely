use std::io::{Read, Bytes, Result};
use std::vec::Vec;

use self::Mark::*;

/// Struct that creates a iterator of chars from a Read
pub(crate) struct Chars<R: Read> {
    inner: Bytes<R>,
}


impl <R: Read> Chars<R> {

    /// Creates a new instance, owns the bytes 
    pub(crate) fn new(b: Bytes<R>) -> Chars<R> {
        Chars{inner: b}
    }

}

impl <R: Read> Iterator for Chars<R> {
    type Item = Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(r) => {
                if r.is_ok() {
                    // convert -- worry about not ASCII later!!
                    // what to do with full UTF-8 compliance
                    Some(Ok(r.unwrap() as char))
                } else {
                    Some(Err(r.unwrap_err()))
                }
            }
            None => None,
        }
    }
}

/// A Mark in the buffer
pub enum Mark {
    /// A position in the buffer
    Pos(usize),
}


/// A Re-windable stream of characters.
pub struct RewindableChars<R:Read> {

    chars: Chars<R>,
    buffer: Vec<u8>, 
    pos: usize,
}



impl <R:Read> RewindableChars<R> {

    pub fn new(b: Bytes<R>) -> RewindableChars<R> {
        RewindableChars{
            chars: Chars::new(b),
            buffer: Vec::new(),
            pos: 0
        }
    }

    pub fn rewind(&mut self, m: Mark) {
        match m {
            Pos(n) => self.pos = n,
        };
    }

    pub fn mark(&mut self) -> Mark {
        Pos(self.pos)
    }

    /// Accepts data read so far (forgets any current re-wind)
    pub fn accept(&mut self) -> Mark {
        self.buffer.clear();
        Pos(0)
    }

    /// Goes back one char
    pub fn backup(&mut self) {
        if self.pos != 0 {
            self.pos -= 1;
        }
    }

}

/// Implements an iterator for RewindableChars.
impl <R: Read> Iterator for RewindableChars<R> {
    type Item = Result<char>;

    fn next(&mut self) -> Option<Self::Item> {

        /* first we check our buffer and move the pointer along */
        if self.pos < self.buffer.len() {
            self.pos += 1;
            return Some(Ok(*self.buffer.get(self.pos-1).unwrap() as char)); // TODO: conversion of next char in the buffer for >127
        }

        match self.chars.next() {
            Some(r) => {
                if r.is_ok() {
                    // convert -- worry about not ASCII later!!
                    // what to do with full UTF-8 compliance
                    let my_char = r.unwrap() as char;
                    
                    // Push the utf bytes onto the Vec
                    if my_char as u32 > 127 {
                        panic!("Need to sort out proper encoding!")
                    } else {
                        self.pos += 1;
                        self.buffer.push(my_char as u8);
                    }

                    Some(Ok(my_char))
                } else {
                    // careful here on re-wrapping
                    Some(Err(r.unwrap_err()))
                }
            }
            None => None,
        }
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use std::io::Cursor;

    #[test]
    pub fn check_can_rewind() {
        let s = String::from("apple banana cherry");
        let mut rb = RewindableChars::new(Cursor::new(s).bytes());
        let mut s = String::new();

        let mut m = rb.mark();

        // out of interest, how big is a Mark
        println!("Size of a mark {} bytes, size of usize {} bytes", std::mem::size_of::<Mark>(), std::mem::size_of::<usize>());

        // Maybe use a String and push a char onto, then can test with assert!
        for _i in 0..5 {
            s.push(rb.next().unwrap().unwrap());
        }
        println!("{:?}", s);
        assert_eq!("apple", s);

        rb.rewind(m);
        s.clear();
        for _i in 0..5 {
            s.push(rb.next().unwrap().unwrap());
        }
        println!("{:?}", s);
        assert_eq!("apple", s);


        s.clear();
        m = rb.accept();
        for _i in 0..7 {
            s.push(rb.next().unwrap().unwrap());
        }
        println!("{:?}", s);
        assert_eq!(" banana", s);

    }
}

pub mod parsers {
    //! Module provinding basic parsing funcs that other parsers can be build on.
    //! By convention return borrowed items such that parsing is zero copy.

    use super::RewindableChars;
    use std::io::Read;
    use std::io::Cursor; // TODO: used in tests only, how to tidy up?

    /// Matches a string 
    pub fn match_str<R>(s: &str, rc: &mut RewindableChars<R>) -> bool
        where R: Read 
    {
        // TODO: what does the return look like
        // Can we reduce need to stop passing "rc" arround ?

        // Check each char
        for c in s.chars() {
            
            // TODO: Double-enum is yukky
            if let Some(x) = rc.next() {
                
                if let Ok(y) =  x {

                    if y != c {
                        return false;
                    }
                }

            } else {
                return false;
            }
        }
        true
    }


    /// Scans input while provided predicate is true
    pub fn skip_while<R, F>(f: F, rc: &mut RewindableChars<R>) 
        where R: Read, F: Fn(char) -> bool 
    {
        while let Some(x) = rc.next() {
            if let Ok(y) = x {
                println!("Char {}", y);
                if !f(y) {
                    println!("\t --> not ws");
                    rc.backup();
                    return;
                }
            }
        }
    }


    // TODO: return type needs to signal EOF
    /// To skip whitespace in character stream
    pub fn skip_whitespace<R>(rc: &mut RewindableChars<R>) 
        where R: Read 
    {
        // we define a closure
        let f = |c:char| c.is_whitespace();
        skip_while(f, rc);
    }



    /// WIP - see if we can match an optional string
    pub fn option_2<R>(s1: &str, s2: &str, rc: &mut RewindableChars<R>) -> bool
        where R: Read 
    {
        // Take a mark, try 1 then 2
        let mark = rc.mark();

        if match_str(s1, rc) {
            return true;
        }
        
        rc.rewind(mark);

        if match_str(s2, rc) {
            return true;
        }

        false
    }


    #[test]
    pub fn check_match_str() {
        let s = String::from("apple banana cherry");
        let mut rb = RewindableChars::new(Cursor::new(s).bytes());

        assert_eq!(true, match_str("apple", &mut rb) );
        assert_eq!(true, match_str(" banana", &mut rb) );
        assert_eq!(false, match_str(" something else", &mut rb) );
        
    }

    #[test]
    pub fn check_match_str_eof() {
        let s = String::from("apple banana cherry");
        let mut rb = RewindableChars::new(Cursor::new(s).bytes());

        assert_eq!(true, match_str("apple", &mut rb) );
        assert_eq!(true, match_str(" banana", &mut rb) );

        // this will run over the end, so will EOF .. how does it do that?
        assert_eq!(false, match_str(" cherryblossome", &mut rb) );

    }

    #[test]
    pub fn check_match_str_and_skip_whitespace() {
        let s = String::from(" apple  banana\tcherry");
        let mut rb = RewindableChars::new(Cursor::new(s).bytes());

        skip_whitespace(&mut rb);
        assert_eq!(true, match_str("apple", &mut rb) );
        skip_whitespace(&mut rb);
        assert_eq!(true, match_str("banana", &mut rb) );
        skip_whitespace(&mut rb);
        assert_eq!(true, match_str("cherry", &mut rb) );

    }


    #[test]
    pub fn check_match_options() {
        let s = String::from("lamb carrot cabbage");
        let mut rb = RewindableChars::new(Cursor::new(s).bytes());

        skip_whitespace(&mut rb);
        assert_eq!(true, option_2("beef", "lamb", &mut rb) );
        skip_whitespace(&mut rb);
        assert_eq!(true, option_2("carrot", "cabbage", &mut rb) );
        skip_whitespace(&mut rb);
        assert_eq!(true, option_2("carrot", "cabbage", &mut rb) );

    }



}
