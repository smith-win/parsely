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


psuedo code below.
Example input: lamb carrot cabbage
    - skip over white space
    - str("lamb") matches  => meat is "top" level, so re-wind no used.
    - skip over white space (accept but don't do anything)
    - .. and similar for carror, cabbage

Example input: lamb beef cabbage
    - skip over white space
    - --> one_of !! MARK
        - str("lamb") matches  
    - <-- one_of !! DISPOSE
    - whitespace
    - one_of MARK! (rewind is possible)
    -   beef - fails to match carrot  !! REWIND
    -   beef - fails to match cabbge  !! REWIND
    - 

meal() {
    whitespace(), // optional
    meat() ,      // mand
    whitespace(),
    veg() ,
    whitespace(),
    veg()
}

/// 
whitepace() {

}

meat() {
    one_of (
        str("lamb"), str("chicken")
    )
}

veg() [
    one_of (
        str("carrot"), str("cabbage")
    )
]
