//! JSON Parsing structs, enums and functions

use std::io::{Read, Bytes};
use super::internals::{RewindableChars, ParseResult, ParseErr, Mark};

use super::internals::parsers::*;



/// This is the key to how an optional parse works.ParseErr
/// Don't mix return types.  All funcs must return ParseResult<x>

/*
  // Take a mark, try 1 then 2
        let mark = rc.mark();

        let mut matched = false;

        match func1(rc) {
            Ok(b) => matched = b ,
            Err(e) => {
                    // Eof may not be fatal in case of option
                    if let ParseErr::Eof = e { return Err(e) }
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
*/
/*

[
    { "name" : "Stuart"}
    , { "name" : "Sharon"}
    , { "name" : "Emily"}
]

=>

// So what are the parameters & returns?

So, should return the 
    1) matched item, and remaining stream
    2) in case of failure - the error 

Result<> -- seems to fit.

How to make stream immutable?  Who decides on re-wind

Don't want all json in memory!!

.. only an "or" rewinds -- as it gives option to try somethign different
.. in case below ("one of")
    .. mark 
        if f1.match.is_err() stream.rewind_to_mark()
        
        ## THIS IS THE KEY I THINK TO SUCCESS .. ONLY WHEN AN OPTION DO WE REWIND
         ... what about "lists" .. last item is optional (JSON allows empty lists)


.. can we use 

json() {
    value()
}

value() {
    one_of( 
        array(),
        object(),
        primitive(),
    )
}

primitive() {
    one_of(
        str("true"), str("null"), str("false"), json_string(), json_number()
    )
}

member() {
    json_string();
    whitespace() /*optional */
    primitive();
}

object() {
    str("{");
    whitespace();
    zero_or_more(
        member_list()
    )
}


member_list() {
    // hmm workout how to spec.
}


    (Start JSON)
    value
        ##literal## true|false|null
        number
        string
        array
        object

    string: quote  char_sequence quote

    char_sequence:  char*

    char:
        \u
        \r 
        \n
        \"
        \\

    number:
        int
        int frac E int

    int:
        [+|-] [0-9]+

    object: { member_list }
        

*/


/// Represents a basic value in JSON, effectively the primitive types (not arrays or objects), 
/// that are emitted by the parser.
/// TODO; how do we make this zero=copy ?? (STring vs &str ??)
#[derive(Debug)]
pub enum JsonEvent {

    /// A string value
    String(String),

    /// Event though it is a number, we'll leave to the client to decide what to co-erce it into (int, float or other)
    Number(String),

    /// Bool is true or false
    Boolean(bool),

    // Null keyword
    Null,

    ObjectStart,
    ObjectEnd,

    ArrayStart,
    ArrayEnd,

}


/// A Stream based JSON parser.
/// Goals are were possible to be zero copy.
pub struct JsonParser<R: Read> {

    rc: RewindableChars<R>,

    // We could do with a stack here for pattern matching;

}


impl <R: Read> JsonParser<R> {

    /// Create a new instance
    // TODO: bytes is
    pub fn new(bytes: Bytes<R>) -> JsonParser<R> {
        JsonParser{rc: RewindableChars::new(bytes)}
    }


    // TODO: remove in favour of re-structuring the JSON parser 
    pub fn mark(&mut self) -> Mark {
        self.rc.mark()
    }

    // TODO: remove in favour of re-structuring the JSON parser 
    pub fn rewind(&mut self, m: Mark) {
        self.rc.rewind(m);
    }

    

    /// Starts parsing of a Json doc
    pub fn parse(&mut self) -> ParseResult<()> {
        self.value() ?;
        Ok(())
    }


    fn value(&mut self) -> ParseResult<()> {
        skip_whitespace(&mut self.rc) ?;

        // TODO: value is object or array or primitive ..
        // self.object() ?; 
        match_or! (&mut self.rc, 
            self.json_string(), 
            self.json_number(),
            self.json_object(),
            self.json_array()
        )
    }


    fn json_array(&mut self) -> ParseResult<()> {

        match_all!(&mut self.rc,
            match_str("[", &mut self.rc),
            skip_whitespace(&mut self.rc),
            self.json_value_list(), 
            skip_whitespace(&mut self.rc),
            match_str("]", &mut self.rc)
        )
    }    

    fn json_object(&mut self) -> ParseResult<()> {

        match_all!(&mut self.rc,
            match_str("{", &mut self.rc),
            skip_whitespace(&mut self.rc),
            self.json_member_list(), 
            skip_whitespace(&mut self.rc),
            match_str("}", &mut self.rc)
        )
    }    


    #[inline]
    fn emit_event(&mut self, val : JsonEvent) {
        println!("JSON event {:?}", val);
    }

    fn json_value_list(&mut self) -> ParseResult<()> {
        // TODO: check with below and see if we can common the "list"
        loop {
            skip_whitespace(&mut self.rc);
            self.value()?;
            skip_whitespace(&mut self.rc);

            let m = self.rc.mark();
            match match_char(',', &mut self.rc) {
                Ok(_) => { /* do nothing, next loop */} ,
                Err(ParseErr::DidNotMatch) => { self.rc.rewind(m); break},
                Err(_) => { return Err( ParseErr::BadData("TODO: fix".to_owned()))  /* TOOD: deal with IO error correctly */ }
            }
        }

        Ok(())        
    }

    ///
    fn json_member_list(&mut self) -> ParseResult<()> {
        
        // A list is such a common pattern
        loop {
            skip_whitespace(&mut self.rc);
            self.json_member()?;
            skip_whitespace(&mut self.rc);

            let m = self.rc.mark();
            match match_char(',', &mut self.rc) {
                Ok(_) => { /* do nothing, next loop */} ,
                Err(ParseErr::DidNotMatch) => { self.rc.rewind(m); break},
                Err(_) => { return Err( ParseErr::BadData("TODO: fix".to_owned()))  /* TOOD: deal with IO error correctly */ }
            }
        }

        Ok(())

    }


    fn json_member(&mut self) -> ParseResult<()> {
        
        // name, colon, value
        self.json_string() ?;
        skip_whitespace(&mut self.rc) ?;
        match_str(":", &mut self.rc) ?;
        skip_whitespace(&mut self.rc) ?;
        self.value() ?;
        // 
        Ok(())
    }


    // So question is how to emit an event (pausing the parsing)
    // like some callback .. but iterating
    // like "next token" ... does a return pop up the stack??
    fn _emit_event(&self) {

    }


    /// Match a JSON number, the result returned as a string so that
    /// client can decide how to interpret
    /// option sign, optional numbers | decimal
    fn json_number(&mut self) -> ParseResult<()> {

        skip_whitespace(&mut self.rc) ?;
        let mut s  = String::new();

        let mut count = 0;

        // optional sign -- could have been "match while"
        if match_str_optional("-", &mut self.rc)? {
            s.push('-');
            count += 1;
        }

        let match_digit = |c| {
           c >= '0' && c <= '9'
        };

        capture_while_mand(match_digit, &mut s, &mut self.rc) ?;

        if s.len() < count {
            Err(ParseErr::DidNotMatch)
        } else {
            self.emit_event(JsonEvent::Number(s));
            Ok(())
        }

        // digits 

        // . digit +

        // digits  . digits

        // E+digits or E-digits

        // 'e' sign digits
        // 'E' sign digits


        // digits or a decimal -- here we need optional pattern
        // e.g. [\-]+ [0-9]* (\. [0-9]*)+ (e|E) ... etc

    }


    /// Matches a JSON string
    fn json_string(&mut self) -> ParseResult<()> {

        // TODO: not too keen on amount of logic in here, the parser library
        // should provide more


        // starts with a quote
        match_str("\"", &mut self.rc) ?;

        // TODO: look for escapes -- \\ \n \t etc, and unicode seq
        //      also add rules for allowed ranges
        // TODO: are \n \r allowed? maybe not
        let mut s = String::new();
        let mut escaped = false;
        loop {
            let x = self.rc.next();
            if x.is_some() {

                // NB: see how unwrap is std::io::Result, so I should be able to "?" operator it
                let mut c = x.unwrap() ?;
                
                if c == '\\' {
                    // println!("Escaping");
                    escaped = true;
                    continue;
                } 

                if escaped {
                    if c == '\"' {
                        // do nothing - just a quote
                    } else if c == 'r' {
                        c = '\r';
                    }
                    // need to escape it
                } else if c == '\"' {
                    // end of string !
                    self.emit_event(JsonEvent::String(s));
                    return Ok(());
                } 

                // Doesn't deal with hex \u00ff77
                escaped = false;

                if c == '\n' || c == '\r' {
                    return Err(ParseErr::BadData(String::from("\r or \n not allowed in string")));
                }

                // just append to result
                s.push(c);

            } else {
                // Unterminated string constant
                return Err(ParseErr::BadData(String::from("unterminated string")));
            }

        }

        //match_str("\"", &mut self.rc) ?;
        // Ok(())
    }


}

#[cfg(test)]
mod tests {

    use super::JsonParser;
    use super::ParseResult;
    use crate::internals::parsers::p_chk;

    use std::io::{Cursor, Read};

    fn create_jp(s: &str) -> JsonParser<Cursor<String>> {
        let c = Cursor::new(String::from(s));
        JsonParser::new(c.bytes())
    }


    #[test]
    fn check_json_string() -> ParseResult<()> {
        let mut jp = create_jp("\"Hello, I am a \\\"JSON String\\\"\"");
        let result = p_chk(jp.json_string())?;
        println!("{:?}", result);

        assert!(result.is_some());

        Ok(())
    }


    #[test]
    fn check_json_number() -> ParseResult<()> {
        let mut jp = create_jp("-12345");
        let result = p_chk(jp.json_number())?;
        assert!(result.is_some());


        // should fail as non-numeric in string ?
        // or maybe fail on next ("unexpected character 'a'")
        let mut jp = create_jp("-12345asas");
        let result = p_chk(jp.json_number())?;
        assert!(!result.is_some());

        Ok(())
    }


    #[test]
    fn check_json_member_list() { 
        println!("-----");
        let mut jp = create_jp(r##""hello":2123"##);
        let mut result =jp.json_member_list();
        assert!(result.is_ok());

        println!("-----");
        jp = create_jp(r##""hello":2123","banana":123"##);
        result =jp.json_member_list();
        assert!(result.is_ok());

        println!("-----");
        jp = create_jp(r##"    "hello"  : 2123"  ,   "banana"  :  123  "##);
        result =jp.json_member_list();
        assert!(result.is_ok());
    }

    #[test]
    fn check_json_object() { 
        let mut jp = create_jp(r##"{ "hello":2123 }"##);
        let result =jp.json_object();
        assert!(result.is_ok());

        let mut jp = create_jp(r##"{ "hello":2123 , "apple": "a day", "another": 12}"##);
        let result =jp.json_object();
        assert!(result.is_ok());

        
        println!("-----");
        let mut jp = create_jp(r##"{ "hello":2123, }"##);
        let result =jp.json_object();
        assert!(result.is_err());

    }


    #[test]
    fn check_nested_json_object() { 
        // double nested objects!
        let mut jp = create_jp(r##"{ "hello":2123, "nested":{ "a":1, "b":"2"} }"##);
        let result =jp.json_object();
        assert!(result.is_ok());

        println!("----------");
        // double nested objects!
        let mut jp = create_jp(r##"{ "hello":2123, "nested":{ "a":1, "b":"2", "c": {"x2":1} }, "nested-again": {"a":3} }"##);
        let result =jp.json_object();
        assert!(result.is_ok());
        
    }


    #[test]
    fn check_json_value() -> ParseResult<()> {
        let mut jp = create_jp("\"banana\"");
        println!("-----");
        let result = p_chk(jp.value())?;

        assert!(result.is_some());

        // check it matches a number too
        jp = create_jp("12345");
        let result = p_chk(jp.value())?;

        assert!(result.is_some());

        Ok(())
    }


    #[test]
    fn check_json_array() -> ParseResult<()> {
        // all numbers
        let mut jp = create_jp(r##"[1, 2, 3, 4, 5, 6, 7, 8, 9] "##);
        let mut result = p_chk(jp.json_array())?;
        assert!(result.is_some());
        println!("-----");

        // strings
        jp = create_jp(r##"["one", "two", "three", "four"] "##);
        result = p_chk(jp.json_array())?;
        assert!(result.is_some());

        // strings + numbers 
        jp = create_jp(r##"["one", 99, "three", -1] "##);
        result = p_chk(jp.json_array())?;
        assert!(result.is_some());

        // objects
        jp = create_jp(r##"[ {"a":1}, { "b": { "c":1 } }, "three", -1] "##);
        result = p_chk(jp.json_array())?;
        assert!(result.is_some());

        Ok(())

    }

    #[test]
    fn check_nested_arrays() -> ParseResult<()>  {
        // nested arrays 
        let mut jp = create_jp(r##"[ {"a":1}, { "b": { "c":1 } }, "three", [ [1, 2], [3], [4,5,"c"] ] ]"##);
        let result = p_chk(jp.json_array())?;
        assert!(result.is_some());

        Ok(())
    }


}