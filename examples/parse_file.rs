extern crate parsely;


use parsely::{internals::ParseResult, json2::JsonParser};
use std::io::BufReader;


fn main() -> ParseResult<()> {

    let args : Vec<String> = std::env::args().collect();

    let filename =  args.get(1).unwrap();
    
    for i in 0..20 {
        println!("Parsing #{}", i);
        let f = std::fs::File::open( filename ).unwrap();
        let buf = BufReader::with_capacity(1024 * 128, f);
        let json = JsonParser::new(buf);
        do_parse(json) ?;
        // let mut json = JsonParser::new(buf);
        // println!("Number of bytes in file: {}", json.count_all_bytes().unwrap());
    }

    Ok(())

}

fn do_parse<R: std::io::Read>(mut p: JsonParser<R>) -> ParseResult<()> {
    let mut obj_count = 0;
    let mut str_count = 0;
    let mut num_count = 0;
    while let Some(e) = p.next_token()? {
        match e {
            parsely::json2::JsonEvent2::ObjectStart => obj_count += 1,
            parsely::json2::JsonEvent2::Number(_n) => num_count += 1,
            parsely::json2::JsonEvent2::String(_s) => str_count += 1,
            _ => {}
        }
    }
    println!("#objects: {}, strings: {}, numbers: {}", obj_count, str_count, num_count);
    Ok(())
}

