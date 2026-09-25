//! @path: karst/src/main.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::main

use karst_span::source_map::SourceTable;

fn main() {
    let mut sm = SourceTable::new();
    let id = sm.add_file("t.krf", "(define x 1)\n");
    let loc = sm.render_location(id, 9, 0);
    let exp = sm.excerpt(id, 9);
    println!("{}", loc);
    println!("{}", exp);
}
