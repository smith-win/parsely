//! JSON Parsing structs, enums and functions

use std::io::{Read, Bytes};
use super::internals::{RewindableChars, ParseResult, ParseErr};
use super::internals::parsers::*;

/// The first matching parser function
#[macro_export]
macro_rules! parse_first {
    // first arg is parser/rewinder, then the args
    ($r:expr, $( $x:expr ),* ) => (
        println!("baloney"); 
        $r.parse() ?;
        $(
            println!(std::stringify!($x));
        )*
    );
}

// /// convert ParseResult<T, ParseErr> into Result<Option<T>, ParseError>
// /// Such that combinators can be reasonably ergonmic.   Parsing functions 
// #[macro_export]
// macro_rules! p_check {
//     ($e:expr) => {
//         if let x = Ok($e) { 
//             Ok(Some(x)) 
//         } else {
//             let x = $e.unwrap_err();
//             Err(x)
//         }
//     }
//         // match $e {
//         //     Ok(x) => Ok(Some(x)),
//         //     Err(x)  => match x { 
//         //             ParseError::Eof => Ok(None),
//         //             _ => Err(x),
//         //         }
//         //     }
//         // } 
    
// }

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
pub enum JsonValue<'a> {

    String(&'a str),

    /// Event though it is a number, we'll leave to the client to decide what to co-erce it into (int, float or other)
    Number(&'a str),
    Boolean(bool),
    Null

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


    /// Starts parsing of a Json doc
    pub fn parse(&mut self) -> ParseResult<()> {
        self.value() ?;
        Ok(())
    }



    fn value(&mut self) -> ParseResult<()> {

        skip_whitespace(&mut self.rc) ?;

        // TODO: value is object or array or primitive ..
        // self.object() ?; 

        self.json_string() ?;
        Ok(())
    }



    fn object(&mut self) -> ParseResult<()> {

        match_str("{", &mut self.rc) ?;
        skip_whitespace(&mut self.rc) ?;
        self.member_list() ? ; 
        match_str("}", &mut self.rc) ?;
        Ok(())
    }    

    ///
    fn member_list(&mut self) -> ParseResult<()> {
        // which has a na
        Ok(())
    }


    fn member(&mut self) -> ParseResult<()> {
        
        // name, colon, value
        self.json_string() ?;
        skip_whitespace(&mut self.rc) ?;
        match_str(":", &mut self.rc) ?;
        skip_whitespace(&mut self.rc) ?;
        self.value() ?;
        // 
        Ok(())
    }


    // // checks for an escape
    // #[inline]
    // fn json_escape(&mut self) -> std::io::Result<()> {
    // }

    fn json_string(&mut self) -> ParseResult<String> {

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
                    println!("Returning string: {}", &s);
                    return Ok(s);
                } 

                // Doesn't deal with hex \u00ff77
                escaped = false;
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

    ///! Tests!
    // #[macro_use]
    // extern crate parsley;

    use super::JsonParser;
    use super::ParseResult;
    use crate::internals::parsers::p_chk;

    use std::io::{Cursor, Read};

    fn create_jp(s: &str) -> JsonParser<Cursor<String>> {
        let c = Cursor::new(String::from(s));
        JsonParser::new(c.bytes())
    }
    
    
    #[test]
    pub fn check_string_no_escapes() -> ParseResult<()> {
        let mut jp = create_jp("\"Hello, I am a JSON String\"");
        jp.parse() ?;

        let mut jp = create_jp("\"Hello, I am a \\\"JSON\\\" String\"");
        jp.parse() ?;
        Ok(())
    }

    #[test]
    pub fn check_my_macro_no_args() {
        // parse_first!();
    }
    
    #[test]
    pub fn check_my_macro_args() -> ParseResult<()> {

        let mut jp = create_jp("\"Hello, I am a \\\"JSON String\\\"\"");

        //parse_first!(1, jp.parse(), 3,"doodle", 5,6,8);
        parse_first!(jp, 3,"doodle", 5+1,6,8);
        // parse_first!(1); // this one fails -- need to be consistent in params to type inference works
        Ok(())
    }



    #[test]
    fn check_macro() -> ParseResult<()> {
        //let mut jp = create_jp("\"Hello, I am a \\\"JSON String\\\"\"");
        let mut jp = create_jp("\"Hello, I am a \\\"JSON String\\\"");

        let result = p_chk(jp.json_string())?;
        println!("{:?}", result);

        if let Some(_x) = result  {
            panic!("arrrgh!");
        }

        Ok(())
    }
    

}