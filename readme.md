Parsing Combinator Library

Key to building the combinators is the ability to "rewind" and input stream.
This is the basic building block of a parser combinator library.

29Apr - have basic feature to rewind.
    - basically a Vec to store data until its accepted
    - poss problem is that it could grow very large -- need to be cautious
        - but "rewind" could go all the way back in case of nesting

01May -
    string matcher
    whitespace skipping
        - learned to use closures

04May - 
    need to implement "option" .. ie first matching parser, in flexible way (e.g object or array or value)
    .. looking at macros to help with this
    idea: should rename RewindableChars to ParseBuffer or similar.. 

05May - 
    Idea: back to basics .. decide on bytes versus chars, look @ Read trait, do we need anything else other than that in "Rewindable"
    Idea: looad at String .. can this acts like a buffer
    decision: looks like Iterator trait  is too clunky, Some|noew, and the Some(Result) is answer, and result could be Ok or Err.

        e.g on iterator.  None is EOF.  This is from underlying.
            Some is a result.  Which could be an Error.
            To make funcs as composable as poss ... we would like to use the ? operator
            
        - we want to handle EOF gracefully ... what do we do ?
        
        - our "atomics" should handle 

        - want "?" to cover exceptions.   So reading from our buffer, 
        ParseResult new function is_eof

        looking at JSON escaping (and CSV for that matter)
        escape - if "\" then get next char.  if have next char & != u, emit escaped char.  if u then need 4 hex digits 0..9, a..f, A..F
        -- so do we need "next(3)", next(1)" ... and emit a 
        -- is escaping a form of transform???  whilst in a transform we 
        -- where "next()" takes a predicate?  

    TODO: need our macro / option to work so can start combining parse functions
        -- you could have different results, so build in "if else" blocks for the dev?  

        Iterator from Bytes trait gives Option<Result<u8>>
        The above block becomes awkward to code.  So "Some" is a match.  "None" is not a match

        -- I could start with something like this ?s

        // expr - an expression, seems to be already eavluated when passed in, so
        // this is no good really

        -- if Ok(mytype) = check!(function)? {
                // got my type, do something?
            } else if Some(anothertype) = check!(function)? {
                // append the bytes?
            }

        -- combining the comparators is up to he dev .. slightly longer winded, but more control???

        -- back to the "Result" problem -- EOF 
        -- Ok so Result<Option<T>, ParseError>  -- that is what we want!, we can make that work.
        -- and rewindable can use the same error tpye so its ergonomic
        -- so macro definition is to convert ParseResult<T, ParseErr> -> Result<Option<T>, ParseError>

        
        ** Check why debug trait is needed on my p_chk function??


08 May
    -- Decided that EOF handling is "iterator just runs out"
        -- our mini parsers will or won't match
        -- higher level, like a stack keeping track of JSON objects/arrays or XML elements would then return Err for incomplete block
        -- we could put fatal errors in for some parsing conditions?   .. as we'd need for the "scaffolding"


    -- Now how about Some|None from a parse-func for an optional result?
        try and model a func call
            function_name (p1, p2, p3)
        -- parameters are zero or more

        Decision! .. None does not make sense.  A parse func, does not know if is part of option etc, 
            using our function example.   input is "my_super_func(p1, p2)"
            comma,name is matched for p2 (as ")" cannot be part of name else ambiguos)
            ... next iteration would try and match ")" -- this is valid, this parse func just needs to say "does no match"

    -- we also need a "repeat" function/macro . like "one_of" that we can use for CSV (and number of parameters)

13 May
    -- the basic function parser is working
    -- created a skeleton macro that can be basis for optional matching (should check return types!)

06 Jun
    -- have basic JSON (no arrays) working inclding nested objects
    -- the matching of lists is a common pattern and I think can be optimised

TODO:
    - DONE! impement arrays
    
    - implement numbers (decimals(done), negatives(done), 
        !!! exponentails +/- on exponent and mantissa)
    
    - UTF-8 handling :-( breaks on Canada file
        -- ignored .. can we skip to u8 rather than char ..  alot less memory to shuffle about
    
    - implement common "list" pattern .. for zero length, 1 or more object members / array members

    - events to match Jackson

    - tidy up generics .. use over just the Read trait to make code tider
        - e.g. not having to call "bytes()" on to create
    - ensure numbers/nulls/true/false working
    - remove all warnings!
    - refactor, refactor, refactor, 
    - clippy
    - try over the Canada data
    - .. profile / profile / profile !!! (possible to compare with Jackson .. maybe "day job" files)




UTF8 Notes


in UTF -- all important control characters are in the ASCII range, 
and numbers etc .. so we only need to worry about those

looking at the encoding rules ... we can "ignore" anything > 127 ? (just skip it)

1-4 bytes

1-byte = 0xxxxxxx
2-byte = 110xxxxx 10xxxxxx
3-byte = 1110xxxx 10xxxxxx 10xxxxxx
4-byte = 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx

192 

10xxxxxx means >= 10000000 < 11000000
means >= 128 && < 192



Performance notes:

1) How stupid am I???   Our internal buffer we never called "accept" to clear it.
    - when do we clear ?
    - json, we know what must be next


extern crate parsely;



21Mar2021

I want to make this run in an iterator manner, rather than 
using the function calls.  An iterator also allows us more control 
when consuming the data.   How to change the function based approch to iterator.


"next()" -- needs to know what to expect.
We also need to keep a stack of our position

These are the events we make

    "SIMPLE" values
    String          --> push nothing on stack
    Number          --> push nothing on stack
    Boolean(bool)   --> push nothing on stack
    Null            --> push nothing on stack

    "STRUCTURAL" values

    ObjectStart,    --> push "object" on stack
    ObjectEnd,      --> pop "object" stack stack ** VALIDATE OBJECT START ON STACK

    ArrayStart      --> push array on stack
    ArrayEnd        --> pop array from stack  ** VALIDATE ARRAY START ON STACK !

31Mar2021
First iterative version working in rough, with basic errors when reach EOF, or invalid data (e..g  array not closed etc)


Playing with compile options
RUSTFLAGS="-Ctarget-cpu=native -Copt-level=2 -Ccodegen-units=1" cargo run --release --example parse_file  [datafile]

non-SIMD, codegen-units=default ... 257 MB/s
non-SIMD, codegen-units=1 ... 298 MB/s

non-SIMD, codegen-units=default ... 277 MB/s
non-SIMD, codegen-units=1 ... 340 MB/s


--> I was always building strings, I no longer do. ==> improved to 400 MB/s
    --> Need to store start/length of values should called wish to get them
    --> If buffer overrun, or needs un-escaping .. need to store that fact

--> Removed duplicative "skip whitespace" calls, improved to 430 MB/s

sudo sh -c 'echo 0 >/proc/sys/kernel/perf_event_paranoid'







