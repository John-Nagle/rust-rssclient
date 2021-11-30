//
//  wordwrap.rs -- Word wrap based on graphemes.
//  
//  Useful for debugging outputs of long text.
//  
use std::str;
extern crate unicode_segmentation;
extern crate itertools;
use self::itertools::Itertools;
use self::unicode_segmentation::UnicodeSegmentation;
///
///  rfind for an array of graphemes
///
fn rfind(s: &[&str], key: &str) -> Option<usize> {
    (0 .. s.len()).rev().find(|&i| s[i] == key)     // search from right
}

///
///  wordwrapline --  wordwrap for one line
///
fn wordwrapline(line: &[&str], maxline: usize, maxword: usize) -> String {
    let mut wline = line;                       // mut ref to array of graphemes as slice                    
    let mut sline = Vec::<&str>::new();         // working vector of graphemes, which are string slices
    while wline.len() > maxline {               // while line too long
        let ix = rfind(&wline[maxline-maxword .. maxline]," ");              // find rightmost space
        wline = match ix {                      // usable word break point?
        None =>     { sline.extend(&wline[0 .. maxline]); // no, really long word which must be broken at margin
                      sline.push("\n");
                      &wline[maxline ..] }      // return shorter wline
        Some(ix) => { sline.extend(&wline[.. ix + maxline - maxword]); // yes, break at space
                      sline.push("\n"); 
                      &wline[ix+maxline-maxword+1 ..] } // return shorter wline
        }
    }
    sline.extend(wline);                        // accum remainder of line
    sline.join("")                              // return string
}
///
///  wordwrap -- understands graphemes
///
pub fn wordwrap(s: &str, maxline: usize, maxword: usize) -> String {
    debug_assert!(maxword < maxline);           // sanity check on params
    s.lines()
        .map(|bline| UnicodeSegmentation::graphemes(bline, true)    // yields vec of graphemes (&str)
            .collect::<Vec<&str>>())
        .map(|line| wordwrapline(&line, maxline, maxword))     
        .join("\n")
}

