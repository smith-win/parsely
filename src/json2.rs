//! Json parser, using only an iterator over bytes

use std::{io::Read};
use std::vec::Vec;
use crate::internals::{ParseResult, ParseErr};

const U8_START_OBJ:u8 = '{' as u8;
const U8_START_ARR:u8 = '[' as u8;
const U8_QUOTE:u8 = '\"' as u8;
const U8_ESCAPE:u8 = '\\' as u8;
const U8_MINUS:u8 = '-' as u8;
const U8_0:u8 = '0' as u8;
const U8_9:u8 = '9' as u8;
const U8_PERIOD:u8 = '.' as u8;


// Character flags
// we have 8 to choose from
//  - non-plain text chars like " and \ when looking at strings
const FLAG_WS:u8 = 4;
const FLAG_DIGIT:u8 = 2;
const FLAG_NOT_TEXT:u8 = 1;
const CHAR_FLAGS : [u8; 256] = {
    let mut x = [0u8; 256];

    // Whitespace chars
    x[8] = FLAG_WS;
    x[9] = FLAG_WS;
    x[10] = FLAG_WS;
    x[13] = FLAG_WS;
    x[32] = FLAG_WS;


    // Digits
    x[b'0' as usize] = FLAG_DIGIT;
    x[b'1' as usize] = FLAG_DIGIT;
    x[b'2' as usize] = FLAG_DIGIT;
    x[b'3' as usize] = FLAG_DIGIT;
    x[b'4' as usize] = FLAG_DIGIT;
    x[b'5' as usize] = FLAG_DIGIT;
    x[b'6' as usize] = FLAG_DIGIT;
    x[b'7' as usize] = FLAG_DIGIT;
    x[b'8' as usize] = FLAG_DIGIT;
    x[b'9' as usize] = FLAG_DIGIT;

    // Not plain text
    x[b'\\' as usize] = FLAG_NOT_TEXT;
    x[b'"' as usize] = FLAG_NOT_TEXT;

    x
};

/// Is the item a digit
#[inline]
const fn is_digit(c: u8) -> bool {
    CHAR_FLAGS[c as usize] & FLAG_DIGIT == FLAG_DIGIT
}

/// Is the item whitespace
#[inline]
const fn is_whitespace(c: u8) -> bool {
    CHAR_FLAGS[c as usize] & FLAG_WS == FLAG_WS
}

#[inline]
const fn is_not_text(c: u8) -> bool {
    CHAR_FLAGS[c as usize] & FLAG_NOT_TEXT == FLAG_NOT_TEXT
    // c == b'"' || c == b'\\'
}


/// Checks a sequence of bytes match - useful for constants
macro_rules! byte_seq {
    // first arg is parser/rewinder, then the args
    ($s:expr, $( $x:expr ),* ) => (
        { 
            $(
                    $s.match_char($x as u8)?;
            )* 
        }
    );
}

/// What does life time mean?
#[derive(Debug)]
pub enum JsonEvent2<'a> {

    /// A string value
    String(&'a str),

    /// Event though it is a number, we'll leave to the client to decide what to co-erce it into (int, float or other)
    Number(&'a str),

    /// Bool is true or false
    Boolean(bool),

    // Null keyword
    Null,

    ObjectStart,
    ObjectEnd,

    ArrayStart,
    ArrayEnd,
}


/// These functions are mainly used to support test cases
impl <'a> JsonEvent2<'a> {

    pub fn is_string(&self) -> bool {
        match self {
            &JsonEvent2::String(_) => true,
            _ => false
        }
    }
    pub fn is_number(&self) -> bool {
        match self {
            &JsonEvent2::Number(_) => true,
            _ => false
        }
    }
    pub fn is_null(&self) -> bool {
        match self {
            &JsonEvent2::Null => true,
            _ => false
        }
    }
    pub fn is_bool(&self) -> bool {
        match self {
            &JsonEvent2::Boolean(_) => true,
            _ => false
        }
    }
    pub fn is_obj_start(&self) -> bool {
        match self {
            &JsonEvent2::ObjectStart  => true,
            _ => false
        }
    }
    pub fn is_obj_end(&self) -> bool {
        match self {
            &JsonEvent2::ObjectEnd => true,
            _ => false
        }
    }
    pub fn is_arr_start(&self) -> bool {
        match self {
            &JsonEvent2::ArrayStart => true,
            _ => false
        }
    }
    pub fn is_arr_end(&self) -> bool {
        match self {
            &JsonEvent2::ArrayEnd => true,
            _ => false
        }
    }

}


/// Private enum that keeps track of parse position
#[derive(Debug)]
enum JsonStackItem {
    /// Where vaue is number of elements in array discovered during the parse
    Array(usize),
    /// WHere value is number of named members in the JSON object discovered so far
    Object(usize)
}

pub struct JsonParser<R: Read> {

    /// Peekable means we can "look ahed" in the iteration
    // bytes: Bytes<R>,
    read: R,

    /// Local buffer seems faster than Reader.bytes() / Bytes
    buffer: Vec<u8>,

    // buffer position and capacty info
    buf_pos: usize,
    //buf_cap: usize,

    /// A buffer for collecting the current value being parsed
    string_buff: String,

    /// For iterative parsing, we keep items on the 
    /// stack 
    stack: Vec<JsonStackItem>

}


impl <R: Read> JsonParser<R> {

    pub fn new(r: R) -> JsonParser<R> {
        JsonParser {
            read: r,
            buffer : Vec::with_capacity(32 * 1024),
            buf_pos: 0,
            string_buff : String::with_capacity(300), // guess at effective initial size
            stack: Vec::with_capacity(10), // 10 deep reasonable default
        }
    }

    /// "Peek" the next byte - used if we want to check if the next token
    /// is equal to something, and only consume it if is.  (Say we want ot check for  keyword etc})
    #[inline]
    fn peek(&mut self) -> ParseResult<Option<u8>> {
        self.ensure_buffer()?;
        if self.buf_pos < self.buffer.len() {
            Ok(Some(self.buffer[self.buf_pos]))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn ensure_buffer(&mut self) -> ParseResult<()> {
        if self.buf_pos >= self.buffer.len() {
            self.replace_buffer()? ;
        }
        Ok(())
    }

    /// If current position matches the char, eat it and return true
    #[inline]
    fn consume_if(&mut self, b: u8) -> ParseResult<bool> {
        self.ensure_buffer()?;
        if self.buf_pos < self.buffer.len() &&  b == self.buffer[self.buf_pos] {
            self.buf_pos += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }


    fn replace_buffer(&mut self) -> ParseResult<()> {
        // re-fill the buffer
        self.buffer.resize(8*0124, 0);
        self.buf_pos = 0;
        match self.read.read(&mut self.buffer) {
            Ok(n) => { 
                self.buffer.truncate(n);
                Ok(())
            } ,
            Err(io) =>  Err(ParseErr::Io(io)),
        }
    }


    /// Moves on until next char is whitespace
    #[inline]
    fn skip_whitespace(&mut self) ->ParseResult<()> {

        loop {

            if self.buf_pos < self.buffer.len() {
                let x = self.buffer[self.buf_pos] ;
                //if x == 32 || x==9 || x == 8 || x == 10 || x == 13 {
                if is_whitespace(x) {
                    self.buf_pos += 1 ;
                } else {
                    return Ok(());
                }
            } else {
                self.replace_buffer()?;
                // check for EOF
                if self.buffer.len() == 0 {
                    return Ok(());
                }
            }
        }

    }

    /// Don't inline it -- check it makes go any faster!
    #[inline]
    fn match_char(&mut self, c: u8) -> ParseResult<()> {

        // No, what if we need to get the next char?
        //self.buf_pos += 1;
        self.ensure_buffer()?;
        if self.buf_pos < self.buffer.len() {
            if self.buffer[self.buf_pos]  == c {
                self.buf_pos += 1;
                return Ok(());
            }
        }
        Err(ParseErr::DidNotMatch)
    }

    /// Used on the remainder of the SIMD func, we put here
    /// see can see the effect of SIMD vs non-SiMD in the result
    fn count_digits_slow(arr: &[u8]) -> ParseResult<usize> {
        Ok(arr.iter()
            .take_while( |n| **n >= b'0' && **n <= b'9')
            .count()
        )
    }

    /// Attempt at SIMD (sse2) for matching digits 
    #[inline]
    fn match_digits_simd(&mut self) -> ParseResult<bool> {
        //Compare packed unsigned 8-bit integers in a and b based 
        // on the comparison operand specified by imm8, and store the results
        // in mask vector k.

        //sse2 simd intructions are not nightly -- thats why I use them :-)
            
        use core::arch::x86_64::{/*__m128i, */ _mm_movemask_epi8, _mm_set_epi8, _mm_cmplt_epi8, _mm_cmpgt_epi8, _mm_set1_epi8, _mm_tzcnt_32 };

        // we could const these ?  no create each time?
        let zeros = unsafe {_mm_set1_epi8(b'0' as i8) };
        let nines = unsafe { _mm_set1_epi8(b'9' as i8) };

        // edge-case --ensure we have room in buffer, not starting on empty!
        while !self.buffer.is_empty()  {
            //assert!(self.buf_cap <= self.buffer.len()); // to remove bounds checking

            let c = self.buffer[self.buf_pos..self.buffer.len()].chunks_exact(16);
            let remainder = c.remainder();

            let mut num_digits = 0usize;
            let mut end = false;
            // process each block of 16
            for b in c {

                let x = unsafe {
                    // _mm_setr_epi8 -- "r" means in reverse order
                    let vec = _mm_set_epi8(b[15] as i8, b[14] as i8, b[13] as i8, b[12] as i8, b[11] as i8, b[10] as i8, b[9] as i8, b[8] as i8, b[7] as i8, b[6] as i8, b[5] as i8, b[4] as i8, b[3] as i8, b[2] as i8, b[1] as i8, b[0] as i8 );
                    let x = _mm_cmplt_epi8(vec, zeros);
                    let y = _mm_cmpgt_epi8(vec, nines);
            
                    let x1 = _mm_movemask_epi8(x);
                    let y1 = _mm_movemask_epi8(y);

                    // rather than take the min, AND with 31 (as we know answer will be in lower bits)
                    // the two results x1,y1 are both < 16, 
                    // answer from _mm_tzcnt_32 can be 32 if all matched (as it takes u32 rather than u16)
                    0b11111 & _mm_tzcnt_32( (x1 | y1 ) as u32) as usize

                    // Shown to be slower then the AND trick is a "min" as it doesn't need a compare
                    //std::cmp::min(_mm_tzcnt_32( (x1 | y1 ) as u32), 16) as usize
                };

                num_digits += x ;

                // this compare and branch is quite slow
                if x != 16 {
                    end = true;
                    break;
                } 
            }

            if !end {
                // we don't need to check remainder if we cut out early
                // we go slowly (one at time over the remainder)
                num_digits += Self::count_digits_slow(remainder)?
            }
            
            let end_pos = self.buf_pos + num_digits;
            // commented out to get "perf" to measure parse-time only
            // unsafe {
            //     self.string_buff.as_mut_vec().extend_from_slice( &self.buffer[self.buf_pos..end_pos]);
            // }
            
            self.buf_pos = end_pos;
            // if we got to the end of the buffer, reload and start again
            // TODO: check boundary here?
            if self.buf_pos < self.buffer.len() {
                return Ok(true);
            } else {
                self.replace_buffer()?;
            }
        }

        Ok(false)

   }
    
    /// Called only from match number, returns true if any digits matched
    /// // we'll try inlining, because only used in number check
    #[inline]
    fn match_digits(&mut self) -> ParseResult<bool> {
        
        while !self.buffer.is_empty() {

            while self.buf_pos < self.buffer.len() - 3 {
                let s = &self.buffer[self.buf_pos..self.buf_pos+4];
                if !is_digit(s[0]) { return Ok(true)}
                if !is_digit(s[1]) { self.buf_pos += 1; return Ok(true);}
                if !is_digit(s[2]) { self.buf_pos += 2; return Ok(true);}
                if !is_digit(s[3]) { self.buf_pos += 3; return Ok(true);}

                self.buf_pos += 4;
            }

            // and the remainder of buffer
            let end_pos = self.buf_pos + self.buffer[self.buf_pos..self.buffer.len()]
                .iter()
                //.take_while( |n| **n >= b'0' && **n <= b'9')
                .take_while( |n| is_digit(**n))
                .count();


            // let mut n = 0usize;
            // for c in &self.buffer[self.buf_pos..self.buf_cap] {
            //     if *c <  b'0' || *c > b'9'{
            //     // if !is_digit(*c) {
            //         break;
            //     }
            //     n += 1;
            // }
            // let end_pos = self.buf_pos + n;

            // unsafe {
            //     self.string_buff.as_mut_vec().extend_from_slice( &self.buffer[self.buf_pos..end_pos]);
            // }

            self.buf_pos = end_pos;
                
            // TODO: check boundary here?
            if self.buf_pos < self.buffer.len() {
                return Ok(true);
            }

            self.replace_buffer()?;
        }
        Ok(false)
    }


    // TODO: .. "12323E+22" style numbers, positive and negative
    // TODO: also check all number formats 
    /// Matches a number in the JSON input
    fn match_number(&mut self) -> ParseResult<JsonEvent2> {
        
        // prob not necessary - we scan number only if matches
        // self.skip_whitespace() ?;

        // very first could be a minus!
        //if self.buf_pos < self.buffer.len() && self.buffer[self.buf_pos] == '-' as u8 {
        //self.string_buff.clear();
        if self.consume_if( U8_MINUS )? {
            //self.string_buff.push('-');
        }

        // if no numbers, its a cockup
        if !self.match_digits_simd()? {
            return Err(ParseErr::DidNotMatch);
        }

        //self.peeked.take(); // hack again
        if self.consume_if( U8_PERIOD )? {
            //self.string_buff.push('.');

            if !self.match_digits_simd()? {
                return Err(ParseErr::DidNotMatch);
            }
        }

        Ok(JsonEvent2::Number(&self.string_buff))

    }


    /// Matches a quoted string
    fn match_string(&mut self) -> ParseResult<()> {

        self.match_char( U8_QUOTE ) ?;

        // TODO: try and get directly into our required byte slice
        //let mut s = String::new();
        // self.string_buff.clear();

        // put this locally to attempt to localise the var .. put in register?
        let mut pos = self.buf_pos;
        
        loop { // end of string 
            loop { // to refill buffer
                // find the next non-text char .. 

                // we'll try unrolling here in blocks of 4
                if pos + 7 < self.buffer.len() /*&& !self.buffer.is_empty() */{
                    
                    let mut slice = &self.buffer[pos..pos+4];

                    let c0 = slice[0];
                    let c1 = slice[1];
                    let c2 = slice[2];
                    let c3 = slice[3];

                    if is_not_text(c0) {
                        // don not add
                        break;
                    }
                    if is_not_text(c1) {
                        pos += 1;
                        break;
                    }
                    if is_not_text(c2) {
                        pos += 2;
                        break;
                    }
                    if is_not_text(c3) {
                        pos += 3;
                        break;
                    }

                    slice = &self.buffer[pos+4..pos+8];
                    let c0 = slice[0];
                    let c1 = slice[1];
                    let c2 = slice[2];
                    let c3 = slice[3];

                    if is_not_text(c0) {
                        pos += 4;
                        break ;
                    }
                    if is_not_text(c1) {
                        pos += 5;
                        break;
                    }
                    if is_not_text(c2) {
                        pos += 6;
                        break;
                    }
                    if is_not_text(c3) {
                        pos += 7;
                        break;
                    }

                    pos += 8;
                } else {
                    // we need to slow check
                    let mut found = false;
                    pos += self.buffer[pos..self.buffer.len()].iter()
                        .take_while( |x| {
                            if CHAR_FLAGS[**x as usize] & 1 == 1 { 
                                found = true;
                                false
                            } else {
                                true
                            }
                        }).count();

                    if found { 
                        break
                    };

                }

                // if we haven't broke out .. continue
                self.buf_pos = pos;
                self.ensure_buffer() ?;
                pos = self.buf_pos;
            }

            // Well always have a char here

            // now actuall check the last char
            if self.buffer[pos] == b'\\' {
                // escape - just ignore next char for now, need to re-loop
                // to collect rest of the String
                pos += 1;
            } else if self.buffer[pos] == b'"' { 
                // break out of outer loop after passed the quote
                pos +=1;
                break;
            }

        }
        self.buf_pos = pos;
        Ok(())
    }

    pub fn match_keyword(&mut self, b: u8) -> ParseResult<JsonEvent2> {

        // we've already skipped white sapce
        // self.skip_whitespace() ?;


        // we could do with a check here that is very quick ...
        // .. and then this "slow" version here, so we can poss make use of 
        // . .e.g vectorizing or using out internal buffer more intelligently

        if b == 't' as u8 { 
            //true
            byte_seq!(self, 't', 'r', 'u', 'e');
            return Ok(JsonEvent2::Boolean(true));
        } else if b == 'f' as u8 { 
            //false
            byte_seq!(self, 'f', 'a', 'l', 's', 'e');
            return Ok(JsonEvent2::Boolean(false));
        } else if b == 'n' as u8 { 
            //null
            byte_seq!(self, 'n', 'u', 'l', 'l');
            return Ok(JsonEvent2::Null);
        } 

        Err(ParseErr::DidNotMatch)
    }

    //  #[inline] - did no make much difference
    fn it_match_value(&mut self) -> ParseResult<JsonEvent2> {
        self.skip_whitespace() ?;
        // Peek the char
        match self.peek()? {
            Some( U8_QUOTE ) => { self.match_string()?; 
                Ok(JsonEvent2::String(&self.string_buff)) }
            Some( U8_START_ARR ) => {self.stack.push(JsonStackItem::Array(0)); 
                self.buf_pos += 1; Ok(JsonEvent2::ArrayStart)}
            Some( U8_START_OBJ ) => {self.stack.push(JsonStackItem::Object(0)); 
                self.buf_pos += 1; Ok(JsonEvent2::ObjectStart)}
            Some ( n ) if (n >= U8_0 && n <= U8_9) || n == U8_MINUS => 
                self.match_number(),
            Some( b ) => self.match_keyword( b ) ,
            _ => Err(ParseErr::DidNotMatch),
        }        
    }

    /// Match an array
    fn it_match_obj_array(&mut self, n: usize) -> ParseResult<JsonEvent2> {
        // whitespace skipped before entry
        // self.skip_whitespace()?;

        // we can take end of object immediatley
        if self.consume_if(b']')? {
            // pop from stack, ensure char is consumed
            self.stack.pop();
            return Ok(JsonEvent2::ArrayEnd);
        }

        // if not object end, check if we need comma or not
        if n != 0 {
            self.match_char(b',')?;
            self.skip_whitespace()?;
        }
        // TODO: increment the object member counter!!
        if let Some(JsonStackItem::Array(n)) = self.stack.last_mut() {
            *n += 1
        }

        // value name
        self.it_match_value()

    }

    fn it_match_obj_member(&mut self, n: usize) -> ParseResult<JsonEvent2> {
        // we can take end of object immediatley
        // self.skip_whitespace()?;
        if self.consume_if(b'}')? {
            // pop from stack, ensure char is consumed
            self.stack.pop();
            return Ok(JsonEvent2::ObjectEnd);
        }

        // if not object end, check if we need comma or not
        if n != 0 {
            self.match_char(b',')?;
            self.skip_whitespace()?;
        }
        // TODO: increment the object member counter!!
        if let Some(JsonStackItem::Object(n)) = self.stack.last_mut() {
            *n += 1
        }

        // value name
        self.match_string() ? ;  
        self.skip_whitespace() ? ;
        self.match_char(':' as u8) ?;
        self.skip_whitespace() ? ;
        self.it_match_value()

    }

    /// Iteratir style function, returns the next token in the parse 
    pub fn next_token(&mut self) -> ParseResult<Option<JsonEvent2>> {
        // if stack is empty, any valie JSON Value item can be next
        // println!("Stack len = {:?}", self.stack);

        // We skip whitespace at start of each call, should not need to do in 
        // each parse method
        self.skip_whitespace() ?;

        // bit hacky .. check for EOF
        if self.buffer.is_empty() {
            return match self.stack.is_empty() {
                true => Ok(None),
                false => Err(ParseErr::DidNotMatch),
            }
        }

        //let _b = self.peek()?;
        match self.stack.last_mut() {
            Some(JsonStackItem::Object(n)) => {let copy = *n; Ok(Some(self.it_match_obj_member(copy)?)) },
            Some(JsonStackItem::Array(n)) => {let copy = *n; Ok(Some(self.it_match_obj_array(copy)?)) },
            None => Ok(Some(self.it_match_value()? )),
        }

    } 

    /// read all bytes!
    pub fn count_all_bytes(&mut self) -> ParseResult<usize>{
        let mut result = 0usize;
        self.replace_buffer()?;

        let mut s = String::with_capacity(50);

        while !self.buffer.is_empty() {

            for c in &self.buffer[self.buf_pos..self.buffer.len()] {
                if *c >= b'0' && *c <= b'9' {
                    result +=1;
                    s.push(*c as char);
                } else if s.len() >0 {
                    s.clear();
                }
            }
            self.replace_buffer()?;
        }

        println!("{}", s);

        Ok(result)

    }

}


#[cfg(test)]
mod tests {

    use super::*;
    use std::io::Cursor;
    
    /// Create parser used during tests
    fn test_parser(s: &str) -> JsonParser<Cursor<&str>> {
        JsonParser::new(
            Cursor::new(s)
        )
    }


    /// Little test helper
    fn token<R: Read>(p: &mut JsonParser<R> ) -> JsonEvent2 {
        p.next_token().unwrap().unwrap()
    }


    #[test]
    fn test_it_parse_obj() -> ParseResult<()> {
        let x = r##"{ 
            "hello": "world", "fruit0":"apple", "fruit1":"banana", "fruit2":"cherry", "fruit3":"damson"
            , "second": {"Aardvark":"A"}
            , "third": {
                "Albatross":"A",
                "fourth": { "x": "x"},
                "a_number": 12.34 ,
                "boolE": false
            }
        }"##;
        println!("{}", x);
        let mut p = test_parser(x);
        let p = &mut p;

        assert!( token(p).is_obj_start() );
            assert!( token(p).is_string() );
            assert!( token(p).is_string() );
            assert!( token(p).is_string() );
            assert!( token(p).is_string() );
            assert!( token(p).is_string() );
            assert!( token(p).is_obj_start() );
                assert!( token(p).is_string() ); // Aardvark
            assert!( token(p).is_obj_end() );
            assert!( token(p).is_obj_start() ); // third
                assert!( token(p).is_string() );
                assert!( token(p).is_obj_start() ); // fourth
                    assert!( token(p).is_string() ); 
                assert!( token(p).is_obj_end() );
                assert!( token(p).is_number() ); 
                assert!( token(p).is_bool() ); 
            assert!( token(p).is_obj_end() );
        assert!( token(p).is_obj_end() );
                
        assert!( (p.next_token()?).is_none() );

        // while let Some(e) = p.next_token()? {
        //     println!(">> {:?}", e);
        //     count += 1;
        //     if count > 100 {
        //         panic!("not supposed to loop forever!")
        //     }
        //     match e {
        //         JsonEvent2::String(s) => println!("String value is {}", s),
        //         _ => {}
        //     }
        // }
        Ok(())
    }

    #[test]
    fn test_it_parse_arr() -> ParseResult<()> {
        let mut p = test_parser(r##"[1, [1.1, "1.2", 1.3], 3, {"a":"nested"}, true, false, null]"##);
        let p = &mut p;
        assert!( token(p).is_arr_start() );
        assert!( token(p).is_number() );
        assert!( token(p).is_arr_start() );
        assert!( token(p).is_number() );
        assert!( token(p).is_string() );
        assert!( token(p).is_number() );
        assert!( token(p).is_arr_end() );
        assert!( token(p).is_number() );
        assert!( token(p).is_obj_start() );
        assert!( token(p).is_string() );
        assert!( token(p).is_obj_end() );
        assert!( token(p).is_bool() );
        assert!( token(p).is_bool() );
        assert!( token(p).is_null() );
        assert!( token(p).is_arr_end() );

        assert!( (p.next_token()?).is_none() );
        Ok(())
    }


    /// Create a byemask from a char
     fn byte_mask_128(b: u8) -> u128 {
        let mut x = 0u128;
        for _i in 0..16 {
            x += b as u128;
            x <<= 8;
        }
        x
    }


    /// Find the first quote in a String
    fn find_quote(s: &str) -> u32 {
        // we can assume its UTF-8 ?
        let b = s.as_bytes();

        let quote = '\"' as u8;
        let chk = &[quote; 16];
        println!("{:?}", chk);
        println!("{:?}", b);
        println!("Mask {:#130b}", byte_mask_128(quote));
        let res = &mut[0u8; 16];
        
        let mut count = 0;
        let iter = b.chunks_exact(16);
        for bytes in iter {
            
            for n in 0..16 {
                // println!("<< {}  {}  {} >>", bytes[n], chk[n], bytes[n] | chk[n]);
                res[n] = bytes[n] & chk[n] ;
            }

            let x = u128::from_be_bytes(*res) & byte_mask_128(quote) ;

            println!("{} {} {:#130b}", count, x, x);
            println!("{:?}", res);

            if x != 0 {
                return count + x.leading_zeros() as u32;
            }
            count += 16;

        }

        // worry about remainder later
        b.len() as u32
    }


    #[test]
    pub fn check_find_quote() {
        assert_eq!(7, find_quote(r##"0123456701234567012345670123456701234567"##));
    } 


}

