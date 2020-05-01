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
