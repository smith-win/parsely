extern crate parsely;


use parsely::{internals::ParseResult, json2::JsonParser};
use std::io::BufReader;


fn main() -> ParseResult<()> {

    let args : Vec<String> = std::env::args().collect();

    let filename =  args.get(1).unwrap();
    
    for i in 0..5 {
        println!("Parsing #{}", i);
        let f = std::fs::File::open( filename ).unwrap();
        let buf = BufReader::new(f);
        let json = JsonParser::new(buf);
        do_parse(json) ?;
    }

    Ok(())

}

fn do_parse<R: std::io::Read>(mut p: JsonParser<R>) -> ParseResult<()> {
    let mut obj_count = 0;
    while let Some(e) = p.next_token()? {
        match e {
            parsely::json2::JsonEvent2::ObjectStart => obj_count += 1,
            _ => {}
        }
    }
    println!("#objects: {}", obj_count);
    Ok(())
}

