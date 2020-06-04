use std::io::{Read, Bytes};
use std::vec::Vec;
use std::result::Result;
use self::Mark::*;



/// Macro for matching one of a series of parsing options
#[macro_export]
macro_rules! match_or {
    // first arg is parser/rewinder, then the args
    ($r:expr, $( $x:expr ),* ) => (
        { 
            let mut pr: ParseResult<()> = Err(ParseErr::DidNotMatch);
            $(
                if let Err(ParseErr::DidNotMatch) = pr {
                    let mark_or = $r.mark();
                    let result = $x;
                    // here is where we match the result
                    pr =  match result {
                        Ok(_) => { Ok(()) }, 
                        Err(ParseErr::DidNotMatch) => { $r.rewind(mark_or); Err(ParseErr::DidNotMatch)},
                        Err(ParseErr::Io(x)) => { Err(ParseErr::Io(x)) },
                        Err(ParseErr::BadData(s)) => { Err(ParseErr::BadData(s))},
                    };
                }
            )* 
            pr
        }
    );
}


/// Macro for matching all of a series of parsing options
#[macro_export]
macro_rules! match_all {
    // first arg is parser/rewinder, then the args
    ($r:expr, $( $x:expr ),* ) => (
        { 
            let mut pr: ParseResult<()> = Ok(());
            $(
                if let Ok(_) = pr {
                    let mark_or = $r.mark();
                    let result = $x;
                    // here is where we match the result
                    pr =  match result {
                        Ok(_) => { Ok(()) }, 
                        Err(ParseErr::DidNotMatch) => { $r.rewind(mark_or); Err(ParseErr::DidNotMatch)},
                        Err(ParseErr::Io(x)) => { Err(ParseErr::Io(x)) },
                        Err(ParseErr::BadData(s)) => { Err(ParseErr::BadData(s))},
                    };
                }
            )* 
            pr
        }
    );
}


/// Error type for parsing issues
#[derive(Debug)]
pub enum ParseErr {

    /// Parser did not match expected input
    DidNotMatch,

    /// Bad data - plus message.  Some un-recoverable error has occured
    BadData(String),

    /// Underlying I/O occured - which is considered fatal
    Io(std::io::Error),
}


/// Custom type used for parser results
pub type ParseResult<T> = Result<T, ParseErr>;


/// Struct that creates a iterator of chars from a Read
struct Chars<R: Read> {
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
    type Item = std::io::Result<char>;

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
                        panic!("Need to sort out proper UTF-8 encoding!")
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

#[macro_use]
pub mod parsers {
    //! Module provinding basic parsing funcs that other parsers can be build on.
    //! By convention return borrowed items such that parsing is zero copy.

    use super::{RewindableChars, ParseResult, ParseErr};
    use std::io::{Read};

    #[cfg(test)]
    use std::io::{Cursor};


    impl std::convert::From<std::io::Error> for ParseErr {

        fn from(io_err: std::io::Error) -> ParseErr {
            ParseErr::Io(io_err)
        }
    }


    // TODO: check effect if inlining
    pub fn p_chk<T: std::fmt::Debug>(pr: ParseResult<T>) -> ParseResult<Option<T>> {
        if let Ok(x) = pr { 
            Ok(Some(x)) 
        } else {
            let x = pr.unwrap_err();
            Err(x)
        }        
    }


    /// Matches a character ... quite useful
    #[inline]
    pub fn match_char<R>(c: char, rc: &mut RewindableChars<R>) -> ParseResult<()>
        where R:Read 
    {
        // check the io_option, if None -- ithe we're EOF
        if let Some(io_option) = rc.next() {
            
            if let Ok(y) = io_option {

                if y != c {
                    return Err(ParseErr::DidNotMatch);
                } else {
                    return Ok(())
                }
            } else {
                return Err(ParseErr::DidNotMatch);
            }
        } else {
            // unexpected oef
            return Err(ParseErr::DidNotMatch);
        }        
    }



    /// Matches a specific string
    #[inline] 
    pub fn match_str<R>(s: &str, rc: &mut RewindableChars<R>) -> ParseResult<bool>
        where R: Read 
    {
        // TODO: what does the return look like
        // Can we reduce need to stop passing "rc" arround ?

        // Check each char
        for c in s.chars() {
            match_char(c, rc) ?
        }
        Ok(true)
    }


    /// Matches a String, but leaves parse position where it is 
    pub fn match_str_optional<R>(s: &str, rc: &mut RewindableChars<R>) -> ParseResult<bool> 
        where R:Read
    {
        let m = rc.mark();
        match match_str(s, rc) {
            Ok(true) =>  Ok(true) ,
            Ok(false) => { rc.rewind(m); Ok(false) } ,
            Err(ParseErr::DidNotMatch) => { rc.rewind(m); Ok(false) } ,
            Err ( x ) => Err(x) ,
        }
    }


    /// Captures while input is mandatory
    pub fn capture_while_mand<R, F>(f: F, s: &mut String, rc: &mut RewindableChars<R>) -> ParseResult<()> 
        where R: Read, F: Fn(char) -> bool 
    {
        let x = s.len();
        capture_while(f, s, rc)?;

        if s.len() != x {
            Ok(())
        } else {
            Err(ParseErr::DidNotMatch)
        }
    }

    /// Tricky .. could be any length -- could pass the the string?
    pub fn capture_while<R, F>(f: F, s: &mut String, rc: &mut RewindableChars<R>) -> ParseResult<()>
    where R: Read, F: Fn(char) -> bool 
    {

        while let Some(x) = rc.next() {
            if let Ok(y) = x {
                if !f(y) {
                    rc.backup();
                    return Ok(());
                } else {
                    s.push(y);
                }
            } else {
                // check for Eof, and ignore ?
            }
        } 
        // TODO: What do with EOF here ?
        Ok(())
    }


    /// Scans input while provided predicate is true
    pub fn skip_while<R, F>(f: F, rc: &mut RewindableChars<R>) -> ParseResult<()>
        where R: Read, F: Fn(char) -> bool 
    {
        while let Some(x) = rc.next() {
            if let Ok(y) = x {
                if !f(y) {
                    rc.backup();
                    return Ok(());
                }
            } else {
                // check for Eof, and ignore ?
            }
        } 
        // TODO: What do with EOF here ?
        Ok(())
    }

    /// Captures exactly "n" characters
    pub fn capture_n<R, F>(rc: &mut RewindableChars<R>, f: F, n: usize) -> ParseResult<String>
        where R: Read, F: Fn(char) -> bool 
    {
        let mut count = 0usize;
        let mut result = String::with_capacity(n); // typically in UTF-8
        while let Some(x) = rc.next() {
            if let Ok(y) = x {
                if f(y) {
                    result.push(y);
                    count += 1;
                    if count == n {
                        return Ok(result);
                    }
                } else {
                    // next char was not wanted .. backup
                    rc.backup();
                    return Err(ParseErr::DidNotMatch)
                }
            }
        }

        // not enough input, so EOF
        Err(ParseErr::DidNotMatch)
    }


    // TODO: return type needs to signal EOF
    /// To skip whitespace in character stream
    pub fn skip_whitespace<R>(rc: &mut RewindableChars<R>) -> ParseResult<()>
        where R: Read 
    {
        // we define a closure
        let f = |c:char| c.is_whitespace();
        skip_while(f, rc) ?;
        Ok(())
    }




    /// WIP - see if we can match an optional string
    pub fn option_2<R>(s1: &str, s2: &str, rc: &mut RewindableChars<R>) -> ParseResult<bool> 
    where R: Read 
    {
        // Take a mark, try 1 then 2
        let mark = rc.mark();

        let mut matched = false;

        match match_str(s1, rc) {
            Ok(b) => matched = b ,
            Err(e) => {
                if let ParseErr::Io(x) = e { return Err(ParseErr::Io(x)) }
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



    /// An optional that takes a closure - doesn't work as signatures of each closure are unique
    pub fn option_2_fn<F, R>(func1: F, func2: F, rc: &mut RewindableChars<R>) -> ParseResult<bool> 
        where R: Read , F: Fn(&mut RewindableChars<R>) -> ParseResult<bool>
    {
        // Take a mark, try 1 then 2
        let mark = rc.mark();

        let mut matched = false;

        match func1(rc) {
            Ok(b) => matched = b ,
            Err(e) => {
                    // Eof may not be fatal in case of option
                    if let ParseErr::Io(x) = e { return Err(ParseErr::Io(x)) }
                }
        };

        if matched {
            return Ok(true);
        } 
        
        // Even if EOF, can rewind
        rc.rewind(mark);
        return match func2(rc) {
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
    pub fn check_match_str() -> ParseResult<()> {
        let mut rb = create_rc("apple banana cherry");

        assert_eq!(true, match_str("apple", &mut rb)? );
        assert_eq!(true, match_str(" banana", &mut rb)? );
        assert!(match_str(" something else", &mut rb).is_err() );
        Ok(())
    }

    #[test]
    pub fn check_match_str_eof() -> ParseResult<()> {
        let mut rb = create_rc("apple banana cherry");

        assert_eq!(true, match_str("apple", &mut rb)? );
        assert_eq!(true, match_str(" banana", &mut rb)? );

        // this will run over the end, so will EOF .. how does it do that?
        // This is example of  to assert for error condition in a test
        assert!( match_str(" cherryblossome", &mut rb).is_err() ) ;
        Ok(())
    }



    #[test]
    pub fn check_match_str_and_skip_whitespace() -> ParseResult<()> {
        let mut rb = create_rc(" apple  banana\tcherry");
        
        skip_whitespace(&mut rb) ?;
        assert_eq!(true, match_str("apple", &mut rb) ? );
        skip_whitespace(&mut rb) ?;
        assert_eq!(true, match_str("banana", &mut rb) ? );
        skip_whitespace(&mut rb) ?;
        assert_eq!(true, match_str("cherry", &mut rb) ? );
        Ok(())
    }



    #[test]
    pub fn check_match_options() -> ParseResult<()> {
        let mut rb = create_rc("lamb carrot cabbage");

        skip_whitespace(&mut rb) ?;
        assert_eq!(true, option_2("beef", "lamb", &mut rb)? );
        skip_whitespace(&mut rb) ?;
        assert_eq!(true, option_2("carrot", "cabbage", &mut rb)? );
        skip_whitespace(&mut rb) ?;
        assert_eq!(true, option_2("carrot", "cabbage", &mut rb)? );
        Ok(())
    }


    /// Function_name ( param1 [, param]*)
    /// A function with zero or more params seperated by a comma
    #[test]
    pub fn check_func_and_params() {
        use crate::internals::parsers;
        use std::vec::Vec;

        /// alpha char
        fn p_alpha<R: Read>(rc: &mut RewindableChars<R>) -> ParseResult<char> {
            if let Some(x) = rc.next() {
                if let Ok(c) = x {
                    if (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') {
                        return Ok(c);
                    }
                }
            }
            Err(ParseErr::DidNotMatch)
        }

        /// Matches a name  - in our case a param name or a function name
        fn p_name<R: Read>(rc: &mut RewindableChars<R>) -> ParseResult<String> {

            // at least one alpha
            let mut s = String::new();
            s.push ( p_alpha(rc) ? );

            let fn_alphanum = |c| {
                (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
            };
            parsers::capture_while(fn_alphanum, &mut s, rc) ?;

            Ok(s)
        }

        /// Matches some parameters
        /// Result is empty, we could choose to emit a vector of params
        fn p_params<R: Read>(rc: &mut RewindableChars<R>) -> ParseResult<Vec<String>> {

            let mut vec : Vec<String> = Vec::new();
            let mut first = true;
            
            parsers::skip_whitespace(rc) ?;
            parsers::match_str("(", rc) ?;
            loop {
                
                // expect comma seperator
                skip_whitespace(rc) ?;
                if !first {
                    match parsers::match_str(",", rc)  {
                        Ok(_) => { },
                        Err(ParseErr::DidNotMatch) => { rc.backup(); break },
                        Err ( x ) => { return Err (x ) },
                    };
                    parsers::skip_whitespace(rc) ? ;
                }

                let name = p_name(rc)?;
                first = false;
                vec.push( name );
            }

            skip_whitespace(rc) ?;
            parsers::match_str(")", rc) ?;
            Ok(vec)
        }


        /// Matches and entire function - name and optional parameters
        /// The return is tuple - the name of the func and then a vec of the Params
        fn p_func<R: Read>(rc: &mut RewindableChars<R>)  -> ParseResult<(String, Vec<String>)> {
            skip_whitespace(rc) ?;
            let s = p_name(rc)? ;
            let v = p_params(rc) ?;
            Ok((s, v))
        }


        let mut rc = create_rc("myfunc12          (param1,param2,param3)");
        let mut result  = p_func(&mut rc);
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!("myfunc12", val.0);
        assert_eq!(vec!["param1", "param2", "param3"], val.1);


        // no end parenthesis
        rc = create_rc("myfunc12( param1, param2 , param3");
        result  = p_func(&mut rc);
        assert!(result.is_err());

        // mismatched parenthesis
        rc = create_rc("myfunc12( param1, param2 , param3]");
        result  = p_func(&mut rc);
        assert!(result.is_err());

        // invalid func name
        rc = create_rc("myf$$unc12( param1, param2 , param3]");
        result  = p_func(&mut rc);
        assert!(result.is_err());
    }



    #[test]
    pub fn check_eof_on_options() -> ParseResult<()> {
        //! Check that we can cope with an EOF, and that EOF not prematurely raised
        let mut rc = create_rc("lamb carrot carrot");

        skip_whitespace(&mut rc) ?;
        assert_eq!(true, option_2("beef", "lamb", &mut rc)? );
        skip_whitespace(&mut rc) ?;
        assert_eq!(true, option_2("carrot", "cabbage", &mut rc)? );
        skip_whitespace(&mut rc) ?;

        // NB: notice how cabbage (7chars) is longer than carrot (6) -- this show not EOF, even though 1st item will
        assert_eq!(true, option_2("cabbage", "carrot", &mut rc)? );        
        Ok(())
    }


    #[test]
    pub fn check_param_list() -> ParseResult<()> {

        use crate::internals::parsers;

        fn p_name<R: Read>(rc: &mut RewindableChars<R>) -> ParseResult<String> {
            let mut s = String::new();
            let fn_alphanum = |c| {
                (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
            };
            parsers::capture_while(fn_alphanum, &mut s, rc) ?;
            Ok(s)
        }

        let mut rc = create_rc("(param1,param2,param3)");
        
        // TODO: need to throw if not mand
        parsers::match_str("(", &mut rc) ?;
       
        let result  = p_name(&mut rc);  
        assert!(result.is_ok());
        println!("Result: {}", result.unwrap_or("Shite!".to_owned()));

        Ok(())

    }


    #[test]
    pub fn check_capture_n() {

        let mut rc = create_rc("AAAAAAAxxxxxxx");

        let match_a = |c| return c == 'A';
        let match_x = |c| return c == 'x';

        match capture_n(&mut rc, match_a, 7) {
            Ok(s) => assert_eq!("AAAAAAA", s), 
            _ => panic!("failed to match")
        };

        match capture_n(&mut rc, match_x, 7) {
            Ok(s) => assert_eq!("xxxxxxx", s), 
            _ => panic!("failed to match")
        };
    }


    #[test]
    pub fn check_match_all_macro() {
        let mut rc = create_rc("AAAAAAAxxxxxxxA");

        let match_a = |c:char| return c == 'A';
        let match_x = |c:char| return c == 'x';

        // match_or!(&mut rc, capture_n(&mut rc, match_a, 7)  ) ;
        let mut x = match_all!(&mut rc
            , capture_n(&mut rc, match_a, 4)
            , capture_n(&mut rc, match_a, 3)
            , capture_n(&mut rc, match_x, 1)  
            , capture_n(&mut rc, match_x, 5)  
            , capture_n(&mut rc, match_x, 1)  
            , capture_n(&mut rc, match_a, 1)  
        ) ;

        assert!(x.is_ok());

        rc = create_rc("AAAAAAAxxxxxAxxx");
        x = match_all!(&mut rc
            , capture_n(&mut rc, match_a, 4)
            , capture_n(&mut rc, match_a, 3)
            , capture_n(&mut rc, match_x, 1)  
            , capture_n(&mut rc, match_x, 5)  
            , capture_n(&mut rc, match_x, 1)  
            , capture_n(&mut rc, match_a, 1)  
        ) ;

        assert!(
            if let Err(ParseErr::DidNotMatch) = x {
                true
            } else {
                false
            }
        );

    }


    #[test]
    pub fn check_match_or() {
        let mut rc = create_rc("a");

        // simple version with no whitespace
        let mut x = match_or!(&mut rc, 
            match_char('x', &mut rc),
            match_char('y', &mut rc),
            match_char('z', &mut rc),
            match_char('b', &mut rc)
        ) ;
        
        // So cannot "return" from my macro == need to do something else!
        assert!(
            if let Err(ParseErr::DidNotMatch) = x {
                true
            } else {
                false
            }
        );

        x = match_or!(&mut rc, 
            match_char('x', &mut rc),
            match_char('y', &mut rc),
            match_char('z', &mut rc),
            match_char('a', &mut rc)
        ) ;
        assert!(x.is_ok());
        
    }

    #[test]
    pub fn check_func_using_macro() {

        use crate::internals::parsers;
        
        /// alpha char
        fn p_alpha<R: Read>(rc: &mut RewindableChars<R>) -> ParseResult<char> {
            if let Some(x) = rc.next() {
                if let Ok(c) = x {
                    if (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') {
                        return Ok(c);
                    }
                }
            }
            Err(ParseErr::DidNotMatch)
        }

        /// Matches a name  - in our case a param name or a function name
        fn p_name<R: Read>(rc: &mut RewindableChars<R>) -> ParseResult<()> {

            // at least one alpha
            let mut s = String::new();
            s.push ( p_alpha(rc) ? );

            let fn_alphanum = |c| {
                (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
            };
            parsers::capture_while(fn_alphanum, &mut s, rc) ?;
            println!("{} matched {}", line!(), s);
            Ok(())
        }
        
        /// params 
        ///  single param (name)
        ///  param ","" params
        fn p_params<R: Read>(rc: &mut RewindableChars<R>) -> ParseResult<()> {
            match_or!(rc
                , match_all!(rc, p_name(rc), match_char(',', rc), p_params(rc))
                , p_name(rc)
            )
        }
        
        fn p_func<R: Read>(rc: &mut RewindableChars<R>) -> ParseResult<()> {
            match_all!(
                rc,
                p_name(rc),
                match_char('(', rc),
                p_params(rc),
                match_char(')', rc)
            )
        }

        // simple version with no whitespace

        //check one match with multiple params
        let mut rc = create_rc("myFunc(x,y,z,xx,yy,zz)");
        assert!( p_func(&mut rc).is_ok() );

        // check single match with single param
        rc = create_rc("myFunc2(my1param)");
        assert!( p_func(&mut rc).is_ok() );

        // repeated calls
        rc = create_rc("myFunc1(my2param)myFunc2(my2param)");
        assert!( p_func(&mut rc).is_ok() );
        assert!( p_func(&mut rc).is_ok() );


    }

}
