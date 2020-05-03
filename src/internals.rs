use std::io::{Read, Bytes, Result};
use std::vec::Vec;

use self::Mark::*;

/*
Notes .. 

errors
    
    underlying I/O (from std::io::Result)
    and EOF (no enough data in stream, so incomplete)

    -- are we effectively "tokenizing" in this layer ?

Have a look at std::io::Result and see its definition and if we can make use of it?

    Os - from underling operating system with the code
    Simple - with an associate ErrorKind - we could re-use (InvalidData & UnexpectedEof)
    Custom
    
is it possible to define a type alias for out method signatures to keep the code tidy ?

    e.g type parser func: Fn<??? ?? ? ?>

*/


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
    type Item = std::io::Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(r) => {
                if r.is_ok() {
                    // convert -- worry about not ASCII later!!
                    // what to do with full UTF-8 compliance ??
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

        let m = rb.mark();

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
        let _m = rb.accept();
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
    use std::io::{Read, Result, Error, ErrorKind};

    #[cfg(test)]
    use std::io::{Cursor};

    /// Matches a string 
    pub fn match_str<R>(s: &str, rc: &mut RewindableChars<R>) -> Result<bool>
        where R: Read 
    {
        // TODO: what does the return look like
        // Can we reduce need to stop passing "rc" arround ?

        // Check each char
        for c in s.chars() {
            
            // check the io_option, if None -- ithe we're EOF
            if let Some(io_option) = rc.next() {
                
                if let Ok(y) = io_option {

                    if y != c {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            } else {
                // unexpected oef
                return Err(Error::new(ErrorKind::UnexpectedEof, "reached EOF before matching string"));
            }
        }
        Ok(true)
    }


    /// Scans input while provided predicate is true
    pub fn skip_while<R, F>(f: F, rc: &mut RewindableChars<R>) 
        where R: Read, F: Fn(char) -> bool 
    {
        while let Some(x) = rc.next() {
            if let Ok(y) = x {
                if !f(y) {
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


    #[inline]
    fn is_eof(err: &std::io::Error) -> bool {
        err.kind() == ErrorKind::UnexpectedEof
    }

    /// WIP - see if we can match an optional string
    pub fn option_2<R>(s1: &str, s2: &str, rc: &mut RewindableChars<R>) -> std::io::Result<bool> 
        where R: Read 
    {
        // Take a mark, try 1 then 2
        let mark = rc.mark();

        if match_str(s1, rc)? {
            return Ok(true);
        }

        let mut matched = false;

        match match_str(s1, rc) {
            Ok(b) => matched = b ,
            Err(e) => {
                // Eof may not be fatal in case of option
                if !is_eof(&e) { return Err(e) }
                }
        };

        if matched {
            return Ok(true);
        } 
        
        // Even if EOF, can rewind
        rc.rewind(mark);
        return match match_str(s2, rc) {
            Ok(b) => Ok(b) ,
            Err(e) => Err(e)
        };

    }


    /// A convenience func used in tests
    #[cfg(test)]
    fn create_rc(s: &str) -> RewindableChars<Cursor<String>> {
        let c = Cursor::new(String::from(s));
        RewindableChars::new(c.bytes())
    }

    #[test]
    pub fn check_match_str() -> std::io::Result<()> {
        let mut rb = create_rc("apple banana cherry");

        assert_eq!(true, match_str("apple", &mut rb)? );
        assert_eq!(true, match_str(" banana", &mut rb)? );
        assert_eq!(false, match_str(" something else", &mut rb)? );
        Ok(())
    }

    #[test]
    pub fn check_match_str_eof() -> std::io::Result<()> {
        let mut rb = create_rc("apple banana cherry");

        assert_eq!(true, match_str("apple", &mut rb)? );
        assert_eq!(true, match_str(" banana", &mut rb)? );

        // this will run over the end, so will EOF .. how does it do that?
        assert!( match_str(" cherryblossome", &mut rb).is_err() ) ;
        Ok(())
    }

    #[test]
    pub fn check_match_str_and_skip_whitespace() -> std::io::Result<()> {
        let mut rb = create_rc(" apple  banana\tcherry");
        
        skip_whitespace(&mut rb);
        assert_eq!(true, match_str("apple", &mut rb) ? );
        skip_whitespace(&mut rb);
        assert_eq!(true, match_str("banana", &mut rb) ? );
        skip_whitespace(&mut rb);
        assert_eq!(true, match_str("cherry", &mut rb) ? );
        Ok(())
    }



    #[test]
    pub fn check_match_options() -> std::io::Result<()> {
        let mut rb = create_rc("lamb carrot cabbage");

        skip_whitespace(&mut rb);
        assert_eq!(true, option_2("beef", "lamb", &mut rb)? );
        skip_whitespace(&mut rb);
        assert_eq!(true, option_2("carrot", "cabbage", &mut rb)? );
        skip_whitespace(&mut rb);
        assert_eq!(true, option_2("carrot", "cabbage", &mut rb)? );
        Ok(())
    }


    #[test]
    pub fn check_eof_on_options() -> std::io::Result<()> {
        //! Check that we can cope with an EOF, and that EOF not prematurely raised
        let mut rb = create_rc("lamb carrot carrot");

        skip_whitespace(&mut rb);
        assert_eq!(true, option_2("beef", "lamb", &mut rb)? );
        skip_whitespace(&mut rb);
        assert_eq!(true, option_2("carrot", "cabbage", &mut rb)? );
        skip_whitespace(&mut rb);

        // NB: notice how cabbage (7chars) is longer than carrot (6) -- this show not EOF, even though 1st item will
        assert_eq!(true, option_2("cabbage", "carrot", &mut rb)? );        
        Ok(())
    }
}
