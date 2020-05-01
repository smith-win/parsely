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
