//! Json parser, using only an iterator over bytes

use std::io::Read;
use crate::internals::{ParseResult, ParseErr};

const U8_START_OBJ:u8 = '{' as u8;
const U8_END_OBJ:u8 = '}' as u8;
const U8_START_ARR:u8 = '[' as u8;
const U8_END_ARR:u8 = ']' as u8;
const U8_QUOTE:u8 = '\"' as u8;
const U8_ESCAPE:u8 = '\\' as u8;
const U8_COMMA:u8 = ',' as u8;
const U8_MINUS:u8 = '-' as u8;
const U8_0:u8 = '0' as u8;
const U8_9:u8 = '9' as u8;
const U8_PERIOD:u8 = '.' as u8;


macro_rules! byte_seq {
    // first arg is parser/rewinder, then the args
    ($s:expr, $( $x:expr ),* ) => (
        { 
            $(
                if let Some(z) = $s.next()? {
                    if z != $x as u8 {
                        return Err(ParseErr::DidNotMatch);
                    }
                } else {
                    return Err(ParseErr::DidNotMatch);
                }
            )* 
        }
    );
}

/// What does life time mean?
#[derive(Debug)]
pub enum JsonEvent2 {

    /// A string value
    String(/* &'a str */),

    /// Event though it is a number, we'll leave to the client to decide what to co-erce it into (int, float or other)
    Number(/* &'a str */),

    /// Bool is true or false
    Boolean(bool),

    // Null keyword
    Null,

    ObjectStart,
    ObjectEnd,

    ArrayStart,
    ArrayEnd,
}


pub struct JsonParser<R: Read> {

    /// Peekable means we can "look ahed" in the iteration
    // bytes: Bytes<R>,
    read: R,

    /// Local buffer seems faster than Reader.bytes() / Bytes
    buffer: Box<[u8]>,

    // buffer position and capacty info
    buf_pos: usize,
    buf_cap: usize,

    /// Peeked byte
    peeked: Option<u8>,

    counts: (u32, u32),

    string_buff: String

}


impl <R: Read> JsonParser<R> {

    pub fn new(r: R) -> JsonParser<R> {
        JsonParser {
            read: r,
            peeked: None,
            counts: (0, 0),
            buffer : Box::new([0u8; 8 * 1024]),
            buf_pos: 0,
            buf_cap: 0,
            string_buff : String::with_capacity(300), // guess at effective initial size
        }
    }


    //
    fn emit_token(&mut self, je: JsonEvent2) {
        match je {
            JsonEvent2::String() => { self.counts.0 += 1 },
            JsonEvent2::Number() => { self.counts.0 += 1 },
            JsonEvent2::ObjectStart => self.counts.1 += 1,
            _ => {},
        }
    }


    /// "Peek" the next byte - used if we want to check if the next token
    /// is equal to something, and only consume it if is.  (Say we want ot check for  keyword etc})
    fn peek(&mut self) -> ParseResult<Option<u8>> {
        self.ensure_buffer()?;
        if self.buf_pos < self.buf_cap && self.buf_pos < self.buffer.len() {
            Ok(Some(self.buffer[self.buf_pos]))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn ensure_buffer(&mut self) -> ParseResult<()> {
        if self.buf_pos >= self.buf_cap {
            self.replace_buffer()? ;
        }
        Ok(())
    }

    /// If current position matches the char, eat it and return true
    fn consume_if(&mut self, b: u8) -> ParseResult<bool> {
        self.ensure_buffer()?;
        if self.buf_pos < self.buffer.len() && b == self.buffer[self.buf_pos] {
            self.buf_pos += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }


    fn replace_buffer(&mut self) -> ParseResult<()> {
        // re-fill the buffer
        self.buf_pos = 0;
        match self.read.read(&mut *self.buffer) {
            Ok(n) => { self.buf_cap = n; Ok(())} ,
            Err(io) =>  Err(ParseErr::Io(io)),
        }
    }

    /// We can keep this up by using peek pos to simulate a peek?  Rather than 
    /// 
    fn next(&mut self) -> ParseResult<Option<u8>> {

        if self.peeked.is_some() {
            return Ok(self.peeked.take());
        } 

        // Check if that < buffer.len() means we skip extra bounds check
        if self.buf_pos < self.buf_cap && self.buf_pos < self.buffer.len() {
            let r = self.buffer[self.buf_pos];
            self.buf_pos += 1;
            return Ok(Some(r));
        }

        self.buf_pos = 0;

        // re-fill the buffer
        match self.read.read(&mut *self.buffer) {
            Ok(n) => self.buf_cap = n,
            Err(io) => return Err(ParseErr::Io(io)),
        }

        // this is same block as above .. so we could simplify somehow
        if self.buf_pos < self.buf_cap && self.buf_pos < self.buffer.len() {
            let r = self.buffer[self.buf_pos];
            self.buf_pos +=1;
            Ok(Some(r))
        } else {
            Ok(None)
        }

    }





    /// Moves on until next char is whitespace
    #[inline]
    fn skip_whitespace(&mut self) ->ParseResult<()> {

        loop {

            if self.buf_pos < self.buf_cap && self.buf_pos < self.buffer.len() {
                let x = self.buffer[self.buf_pos] ;

                if x == 32 || x==9 || x == 8 || x == 10 || x == 13 {
                    self.buf_pos += 1 ;
                } else {
                    return Ok(());
                }
            } else {
                self.replace_buffer()?;
            }
        }

    }

    

    // Matches a member
    fn match_member(&mut self) -> ParseResult<()> {
        self.skip_whitespace() ? ;
        self.match_string() ? ;
        self.skip_whitespace() ? ;
        self.match_char(':' as u8) ?;
        self.skip_whitespace() ? ;
        self.match_value()
    }


    fn match_member_list(&mut self) -> ParseResult<()> {

        // first member
        self.match_member() ? ;
        self.skip_whitespace() ?;

        while self.consume_if(U8_COMMA)? {
            self.match_member() ? ;
            self.skip_whitespace() ?;
        }
        Ok(())
    }


    fn match_object(&mut self) -> ParseResult<()> {
        self.match_char( U8_START_OBJ ) ?;
        self.emit_token(JsonEvent2::ObjectStart);


        // need a member list
        self.skip_whitespace() ?;

        // ! quick exit for empty array
        if self.consume_if( U8_END_OBJ )? {
            self.emit_token(JsonEvent2::ObjectEnd);
            return Ok(());
        }
        
        self.match_member_list()? ;
        self.skip_whitespace() ?;
        self.match_char( U8_END_OBJ ) ?;
        self.emit_token(JsonEvent2::ObjectEnd);
        Ok(())
    }


    /// Don't inline it -- check it makes go any faster!
    #[inline]
    fn match_char(&mut self, c: u8) -> ParseResult<()> {
        let x = self.next() ?;
        match x {
            Some (n) if n == c => Ok(()),
            _  => Err(ParseErr::DidNotMatch) ,
        }
    }

    
    fn match_number(&mut self) -> ParseResult<()> {
        
        // prob not necessary - we scan number only if matches
        // self.skip_whitespace() ?;

        // leading char already checked
        //self.string_buff.push(leading as char);
        
        // can we capture using the 
        // let match_digit = |c| {
        //     c >= '0' as u8 && c <= '9' u8
        // };
        // very first could be a minus!
        if self.buf_pos < self.buffer.len() && self.buffer[self.buf_pos] == '-' as u8 {
            self.string_buff.push('-');
            self.buf_pos += 1;
        }


        let mut count = 0u16 ;

        while self.buf_cap > 0 {
        
            if self.buf_pos < self.buf_cap && self.buf_pos < self.buffer.len()  {
                    
                let  n = self.buffer[self.buf_pos] ;

                if n >= '0' as u8  && n <= '9' as u8 {
                    self.buf_pos += 1 ;
                    self.string_buff.push(n as char);
                    count +=1;
                } else {
                    break;
                }
            }  else {
                self.replace_buffer()?;
            }
        }

        // if no numbers, its a cockup
        if count == 0 {
            &self.string_buff;
            return Err(ParseErr::DidNotMatch);
        }

        count = 0;
        //self.peeked.take(); // hack again
        if self.consume_if( U8_PERIOD )? {
            // same again ..
            self.string_buff.push('.');
            while self.buf_cap > 0 {
        
                if self.buf_pos < self.buf_cap && self.buf_pos < self.buffer.len()  {
                        
                    let  n = self.buffer[self.buf_pos] ;
    
                    if n >= '0' as u8  && n <= '9' as u8 {
                        self.buf_pos += 1 ;
                        self.string_buff.push(n as char);
                        count +=1;
                    } else {
                        break;
                    }
                }  else {
                    self.replace_buffer()?;
                }
            }
        }

        while self.buf_cap > 0 {
        
            if self.buf_pos < self.buf_cap && self.buf_pos < self.buffer.len()  {
                    
                let  n = self.buffer[self.buf_pos] ;

                if n >= '0' as u8  && n <= '9' as u8 {
                    self.buf_pos += 1 ;
                    self.string_buff.push(n as char);
                    count +=1;
                } else {
                    break;
                }
            }  else {
                self.replace_buffer()?;
            }
        }


        self.emit_token(JsonEvent2::Number());
        self.string_buff.clear();

        // self.peeked.take();
        Ok(())
        // digits 

        // . digit +

        // digits  . digits

        // E+digits or E-digits

        // 'e' sign digits
        // 'E' sign digits


        // digits or a decimal -- here we need optional pattern
        // e.g. [\-]+ [0-9]* (\. [0-9]*)+ (e|E) ... etc

    }

    /// Matches a quoted string
    fn match_string(&mut self) -> ParseResult<()> {

        self.match_char( U8_QUOTE ) ?;

        // TODO: try and get directly into our required byte slice
        //let mut s = String::new();
        self.string_buff.clear();
        let mut escaped = false;
        if self.peeked.is_some() {
            self.peeked.take();
            self.buf_pos -= 1;

        }

        // trying to significantly beat 4.2 seconds
        // String "push" -- does not appear to be issue
        // let mut range_start = self.buf_pos;
        // let fast_check = [0u8, 0u8, 0u8, 0u8];

        // quote         34 - 
        // back slash = 134 = 

        loop {

            if self.buf_pos < self.buf_cap && self.buf_pos < self.buffer.len() {
                
                let mut c = self.buffer[self.buf_pos];
                self.buf_pos += 1;

                if c == U8_ESCAPE {
                    escaped = true;
                    continue;
                } 

                // not if in "escape mode"                
                if c == '\n' as u8 || c == '\r' as u8 {
                    return Err(ParseErr::BadData(String::from("\r or \n not allowed in string")));
                }

                // asssume input is valid utf8, so its just a char
                if c > 127 { 
                    continue;
                }

                if escaped {
                    // Doesn't deal with hex \u00ff77
                    escaped = false;
                    if c == '\"' as u8 {
                        // do nothing - just a quote
                    } else if c == 'r' as u8 {
                        c = '\r' as u8;
                    }
                    // TODO : \n and \u812376 \nd \t
                    // need to escape it
                } else if c == U8_QUOTE {
                    // end of string !
                    // TODO: zero-copy of string please!
                    // let the_str = self.string_buff.as_str();
                    self.emit_token(JsonEvent2::String( /* the_str */ ));
                    &self.string_buff;
                    return Ok(());
                } 

                // TODO: try not to cast to char?
                // just append to result
                //

            }  else {
                self.replace_buffer() ?;
            }
        }
    }

    fn match_array(&mut self) -> ParseResult<()> {
        self.skip_whitespace() ?;
        
        // special case .. zero length
        self.match_char(U8_START_ARR) ?;
        self.emit_token(JsonEvent2::ArrayStart);

        // TODO: middle bit, which is a "value list"

        self.skip_whitespace() ?;
        // ! quick exit for empty array
        if self.consume_if( U8_END_ARR )? {
            self.emit_token(JsonEvent2::ArrayEnd);
            return Ok(());
        }

        self.match_value_list()? ;
        self.skip_whitespace() ?;

        self.match_char(U8_END_ARR) ?;
        self.emit_token(JsonEvent2::ArrayEnd);
        Ok(())

    }


    fn match_value_list(&mut self) -> ParseResult<()> {
        // first member
        self.match_value() ? ;
        self.skip_whitespace() ?;

        while self.consume_if(U8_COMMA)? {
            self.match_value() ? ;
            self.skip_whitespace() ?;
        }

        Ok(())
    }


    pub fn match_keyword(&mut self, b: u8) -> ParseResult<()> {

        // we've already skipped white sapce
        // self.skip_whitespace() ?;


        // we could do with a check here that is very quick ...
        // .. and then this "slow" version here, so we can poss make use of 
        // . .e.g vectorizing or using out internal buffer more intelligently

        if b == 't' as u8 { 
            //true
            byte_seq!(self, 't', 'r', 'u', 'e');
            self.emit_token(JsonEvent2::Boolean(true));
        } else if b == 'f' as u8 { 
            //false
            byte_seq!(self, 'f', 'a', 'l', 's', 'e');
            self.emit_token(JsonEvent2::Boolean(false));
        } else if b == 'n' as u8 { 
            //null
            byte_seq!(self, 'n', 'u', 'l', 'l');
            self.emit_token(JsonEvent2::Null);
        } 

        Ok(())
    }


    /// 
    pub fn match_value(&mut self) -> ParseResult<()> {

        self.skip_whitespace() ?;
        // Peek the char
        match self.peek()? {
            Some( U8_QUOTE ) => self.match_string(),
            Some( U8_START_ARR ) => self.match_array(),
            Some( U8_START_OBJ ) => self.match_object(),
            Some ( n ) if (n >= U8_0 && n <= U8_9) || n == U8_MINUS => self.match_number(),

            // true,false,null -- we can put thisin here 't', 'f', 'n' etc
            Some( b ) => self.match_keyword( b ) ,
            _ => Err(ParseErr::DidNotMatch),

        }

    }


    /// Plus register a call back
    pub fn parse(&mut self) -> ParseResult<()> {

        self.match_value()?;
        println!("#objects: {}, #values {}", self.counts.1, self.counts.0);
        Ok(())
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


    #[test]
    pub fn check_peek_and_next() -> ParseResult<()> {
        let mut p = test_parser(r##"     "hello \"world\"""##);
        p.match_value() ?;
        Ok(())
    }

    #[test]
    pub fn check_empty_object() -> ParseResult<()> {
        let mut p = test_parser(r##" { } "##);
        p.match_value() ?;
        Ok(())
    }

    #[test]
    pub fn check_single_value_object() -> ParseResult<()> {
        let mut p = test_parser(r##" { "apple":"banana"} "##);
        p.match_value() ?;
        Ok(())
    }

    #[test]
    pub fn check_single_multi_value_object() -> ParseResult<()> {
        //
        let mut p = test_parser(r##"{"$schema": "http://donnees-data.tpsgc-pwgsc.gc.ca/br1/delaipaiement-promptpayment/delaipaiement-promptpayment_schema.json",
 
                 "xx": [
                    {
                        "procurement-id_id-approvisionnement":"EJ19670258",
                        "Project-number_Numéro-de-projet":"R.041736.894",
                        "Vendor-name_Nom-du-fournisseur":"AUTOMATED LOGIC - CANADA, LTD."
                    }, {  
                    }
                    ]
            } "##);
        p.match_value() ?;
        Ok(())
    }

    #[test]
    pub fn check_empty_array() -> ParseResult<()> {
        let mut p = test_parser(r##"[ ]"##);
        p.match_value() ?;
        Ok(())
    }

    #[test]
    pub fn check_array() -> ParseResult<()> {
        let mut p = test_parser(r##"[ 
            "aa", {}, "cc", 12, -34 ]"##);
        p.match_value() ?;
        Ok(())
    }

    #[test]
    pub fn check_number() -> ParseResult<()> {
        let mut p = test_parser(r##" 12343"##);
        p.match_value() ?;

        let mut p = test_parser(r##" 12343.12"##);
        p.match_value() ?;

        let mut p = test_parser(r##" -12343.12"##);
        p.match_value() ?;
        Ok(())
    }


    #[test]
    pub fn check_keyword() -> ParseResult<()> {
        let mut p = test_parser(r##" true "##);
        p.match_value() ?;

        let mut p = test_parser(r##" null "##);
        p.match_value() ?;

        let mut p = test_parser(r##" false "##);
        p.match_value() ?;

        Ok(())
    }



    #[test]
    pub fn check_string() -> ParseResult<()> {
        let mut p = test_parser(r##""simple string""##);
        p.match_string() ?;

        p = test_parser(r#""""#);
        p.match_string() ?;

        p = test_parser(r#""with\rin 0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789""#);
        p.match_string() ?;

        Ok(())
    }


}