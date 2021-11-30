//
//  rss reader - main program
//
//
//#![feature(io)]
extern crate chrono;

mod rssread;
mod wordwrap;
#[tokio::main]
async fn main() {
    rssread::test1().await; // ***TEMP***
}
