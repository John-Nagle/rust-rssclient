//
//  rss reader - main program
//
//
extern crate reqwest;
extern crate xml;
extern crate chrono;
use std::convert;
use std::io;
use std::io::Read;
use std::fmt;
use std::env;
use super::wordwrap;


//
//  xml::Element utility functions.
//  These should be in the XML module.
//
//
//  find_all --  find all elements for which test function returns true.
//
//  Someday this will accept unboxed lambdas for testfn, but that's not working yet in Rust 1.0 pre-alpha
//
pub fn find_all<'a>(tree: &'a xml::Element, testfn: fn(&xml::Element) -> bool, finds: &mut Vec<&'a xml::Element>, recurse: bool) {
    if testfn(tree)                                         // if match 
    {   finds.push(tree);                                   // save if match on name
        if !recurse { return }                              // don't explore within finds if non-recursive
    }
    for child in tree.children.iter() {                     // for all children
        if let xml::Xml::ElementNode(ref childelt) = *child { find_all(childelt, testfn, finds, recurse ); }
    }
}
//
//  find_all_text -- find all text below a tag.
//
pub fn find_all_text(tree: &xml::Element, s: &mut String, recurse: bool) {
    for child in tree.children.iter() {                     // for all children
        match *child {                                      // child is an Xml enum
            xml::Xml::ElementNode(ref childelt) => {
                if recurse {
                    find_all_text(childelt, s, recurse);    // do ElementNode recursively
                    }
                },
            xml::Xml::CharacterNode(ref text) => { s.push_str(text) }  // append text
            _ => ()                                         // ignore non ElementNode children
        }                                                   // end match child
    }
}
//
//  find_tag_text -- find text of single tag for which test function returns true.
//
//  No find, or multiple finds. return an empty string.
//
//  Yes, it's somewhat inefficient to create a Vec and collect all the items to find only one.
//
pub fn find_tag_text(tree: &xml::Element, testfn: fn(&xml::Element) -> bool, s: &mut String) {
    let mut itemelts = Vec::<&xml::Element>::new();         // accumulate tag elements
    find_all(tree, testfn, &mut itemelts, false);           // find item elts
    if itemelts.len() != 1 { return }                       // should be a singleton
    find_all_text(itemelts[0], s, false);                   // find text
}


//
//  Return type and its error handling
//
pub type FeedResult<T> = Result<T, FeedError>;

/// A set of errors that can occur handling RSS or Atom feeds.
//#[derive(Debug, PartialEq, Clone)] // crank out default functions
pub enum FeedError {
    /// Error detected at the I/O level
    Io(io::Error),
    /// Error detected at the HTTP level
    Http(reqwest::Error),
    /// Error detected at the XML parsing level
    XMLParse(xml::BuilderError),
    /// Error detected at the date parsing level
    DateParse(chrono::format::ParseError),
    /// XML, but feed type not recognized,
    UnknownFeedType,
    /// Required feed field missing or invalid
    Field(String),
    /// Got an HTML page instead of an XML page
    WasHTML(String)
}
//
//  Encapsulate errors from each of the lower level error types
//
impl convert::From<reqwest::Error> for FeedError {
    fn from(err: reqwest::Error) -> FeedError {
        FeedError::Http(err)
    }
}
impl convert::From<io::Error> for FeedError {
    fn from(err: io::Error) -> FeedError {
        FeedError::Io(err)
    }
}

impl convert::From<xml::BuilderError> for FeedError {
    fn from(err: xml::BuilderError) -> FeedError {
        FeedError::XMLParse(err)
    }
}

impl convert::From<chrono::format::ParseError> for FeedError {
    fn from(err: chrono::format::ParseError) -> FeedError {
        FeedError::DateParse(err)
    }
}


impl fmt::Display for FeedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FeedError::Io(ref xerr) => xerr.fmt(f),   // I/O error
            FeedError::Http(ref xerr) => xerr.fmt(f), // HTTP error
            FeedError::XMLParse(ref xerr) => xerr.fmt(f), // XML parse error
            FeedError::DateParse(ref xerr) => xerr.fmt(f), // Date parse error
            FeedError::UnknownFeedType => write!(f, "Unknown feed type."),
            FeedError::Field(ref s) => write!(f, "Required field \"{}\" missing from RSS/Atom feed.", s),
            FeedError::WasHTML(ref s) => write!(f, "Expected an RSS/ATOM feed but received a web page \"{}\".", s)
            //_(ref xerr) => xerr.fmt(f) // would be convenient, but not allowed in Rust.
        }
    }
}

//
//  FeedChannel --  an RSS or Atom channel
//
pub struct FeedChannel {
    title: String,                                          // channel title
    link: String,                                           // URL of channel (from RSS channel)
    description: String,                                    // channel description if present
    }
//
//  FeedItem - a RSS or Atom feed item
//
pub struct FeedItem {
    title: String,                                          // the title
    description: String,                                    // the description, i.e. content, as HTML
    author: String,                                         // name of author
    pubdate: chrono::DateTime<chrono::FixedOffset>          // publication date
    }
//
//  FeedReply -- data returned from a feed
//
pub struct FeedReply {
    channel: FeedChannel,                                   // channel info
    items: Vec<FeedItem>                                    // items found
    }
    
impl FeedItem {
    pub fn dump(&self){                                     // dump content for debug
        println!("Feed Item");
        println!(" Title: {}", self.title);
        println!(" Author: {}", self.author);
        println!(" Publication date: {}", self.pubdate);
        println!(" Description:\n{}", &wordwrap::wordwrap(&self.description, 72,20));
        println!();
    }
}

impl FeedChannel {
    pub fn new() -> FeedChannel {
        FeedChannel{title: String::new(), link: String::new(), description: String::new()}
    }


    pub fn dump(&self) {
        println!("Feed Channel");
        println!(" Title: {}", self.title);
        println!(" Link: {}", self.link);
        println!(" Description: {}", self.description);
        println!();
    }
}
    
impl FeedReply {
    pub fn new() -> FeedReply {
        FeedReply{channel: FeedChannel::new(), items: Vec::<FeedItem>::new()}
    }

    pub fn dump(&self) {
        self.channel.dump();
        for item in self.items.iter() {
            item.dump()
        }
    }
}
//
//  handletree  --  handle a tree generated by the XML parser
//
//  May be RSS, Atom, or HTML.
//
//  HTML is an error, usually indicating some problem with a firewall or the site.
//
pub fn handletree(tree: &xml::Element, reply: &mut FeedReply) -> FeedResult<()> {
                                                            // ***MORE***
    handlersstree(tree, reply)                              // for now, just RSS
}    
//
//  handlersstree -- handle a tree of RSS items
//
pub fn handlersstree(tree: &xml::Element, reply: &mut FeedReply) -> FeedResult<()> {
    //println!("Tree: {}\n\n", tree);                       // ***TEMP***
    //  ***NEED TO COLLECT CHANNEL INFO***
    let mut itemelts = Vec::<&xml::Element>::new();         // accumulate ITEM entries
    fn isitem(e: &xml::Element) -> bool { e.name == "item" }// someday this will be an unboxed lambda
    find_all(tree, isitem, &mut itemelts, false);           // find all items
    for itemelt in itemelts.iter() {                        // extract important fields from items
        let mut authorstr = String::new();                  // title string
        fn isauthor(e: &xml::Element) -> bool { e.name == "author" } // someday this will be an unboxed lambda
        find_tag_text(*itemelt, isauthor, &mut authorstr);
        
        let mut pubdatestr = String::new();                 // date string
        fn ispubdate(e: &xml::Element) -> bool { e.name == "pubDate" } // someday this will be an unboxed lambda
        find_tag_text(*itemelt, ispubdate, &mut pubdatestr);
        let pubtimestamp = match chrono::DateTime::parse_from_rfc2822(&pubdatestr) {       
            Ok(date) => date,
            Err(xerr) => return Err(convert::From::from(xerr))
        };
        
        let mut titlestr = String::new();                   // title string
        fn istitle(e: &xml::Element) -> bool { e.name == "title" } // someday this will be an unboxed lambda
        find_tag_text(*itemelt, istitle, &mut titlestr);

        let mut descriptionstr = String::new();             // description string
        fn isdescription(e: &xml::Element) -> bool { e.name == "description" } // someday this will be an unboxed lambda
        find_tag_text(*itemelt, isdescription, &mut descriptionstr);

        let feeditem = FeedItem{title: titlestr, description: descriptionstr, author: authorstr, pubdate: pubtimestamp};
        reply.items.push(feeditem);
    }        
    ////println!("Item: {}", item);
    Ok(())
}
//
//  readfeed -- read from an RSS or Atom feed.
// 
pub fn readfeed(url: &str, reply: &mut FeedReply, verbose: bool) -> FeedResult<()> {
    ////let client = hyper::Client::new();                  // create HTTP client from Hyper crate.
    let mut res = match reqwest::blocking::get(url) {
    ////let mut res = match client.get(url).send() {            // HTTP GET request
        Ok(res) => res,
        Err(xerr) => return Err(convert::From::from(xerr)) // fails, return error
    };
    //  Successful read of URL.  Have valid result object.
    if verbose {
        println!("Response: {}", res.status());               // debug print
        println!("Headers:\n{:?}", res.headers());
    }
    let mut p = xml::Parser::new();                         // get XML parser
    let mut e = xml::ElementBuilder::new();                 // get tree builder
    let mut ws = String::new();                             // get work string for XML
    match res.read_to_string(&mut ws) {                     // read the XML
        Ok(_) => (),                                        // OK, keep going
        Err(xerr) => { return Err(convert::From::from(xerr)) }   // I/O error
    };
    p.feed_str(&ws[..]);                                    // prepare XML parser
    for event in p {                                        // for each full XML tree (normally only one)
        if let Some(rep) = e.handle_event(event) {                                  // returns an Option wrapped around a Result
            match rep {
                Ok(ev) => match handletree(&ev, reply) {    // we have an XML tree to process
                    Ok(()) => (),
                    Err(xerr) => return Err(xerr)           // RSS/Atom error
                    },                                      // end match handletree
                Err(xerr) => return Err(convert::From::from(xerr)) 
            }                                           // end match rep
        }                                                   // end match handle_event
    }                                                       // end for
    Ok(())
}
    
//
//  test1  -- temporary test function
//
//  Ask for URL of RSS feed, handle it.
//
pub fn test1() {
    let args : Vec<_> = env::args().collect();      // command line args as vector of strings
    match args.len() {
        2 => (),
        _ => {
            println!("Usage: client <url>");
            return;
        }
    };
    let url = &*args[1];
    println!("Reading \"{}\"", url);
    let mut result = FeedReply::new();            // accumulate ITEM entries
    let res = readfeed(url, &mut result, true);
    match res {
        Ok(_) => println!("OK."),
        Err(err) => println!("Error: {}", err)
        }
    result.dump();
}
//
//  Unit tests.
//
#[test]
fn testreutersrss() {
    ////let url = "http://feeds.reuters.com/reuters/topNews?format=xml";    // ***TEMP***
    let url = "https://rss.nytimes.com/services/xml/rss/nyt/World.xml";
    let mut freply = FeedReply::new();            // accumulate ITEM entries
    let res = readfeed(url, &mut freply, true);
    match res {
        Ok(_) => println!("OK."),
        Err(err) => panic!("Error: {}", err)
        }
    freply.dump();
}



