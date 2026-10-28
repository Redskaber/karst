//! @path: karst/crates/karst_vm/builtin.rs
//! @author: redskaber
//! @datetime: 2026-10-28
//! @discription: karst::crates::karst_vm::builtin
//!
//! bytecode virtual machine - the builtin registry

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    /// acrithmetic
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    NumEq,
    /// pair and predcate
    Car,
    Cdr,
    Cons,
    IsPair,
    IsNil,
    Eq,
    Not,
    /// mininmal io
    ReadLine,
    WriteLine,
}

impl Builtin {
    pub const ALL: [(Builtin, &'static str, u32); 18] = [
        (Builtin::Add, "+", 2),
        (Builtin::Sub, "-", 2),
        (Builtin::Mul, "*", 2),
        (Builtin::Div, "/", 2),
        (Builtin::Lt, "<", 2),
        (Builtin::Le, "<=", 2),
        (Builtin::Gt, ">", 2),
        (Builtin::Ge, ">=", 2),
        (Builtin::NumEq, "=", 2),
        (Builtin::Car, "car", 1),
        (Builtin::Cdr, "cdr", 1),
        (Builtin::Cons, "cons", 2),
        (Builtin::IsPair, "pair?", 1),
        (Builtin::IsNil, "nil?", 1),
        (Builtin::Eq, "eq?", 2),
        (Builtin::Not, "not", 1),
        (Builtin::ReadLine, "read-line", 0),
        (Builtin::WriteLine, "write-line", 1),
    ];
    pub fn from_name(name: &str) -> Option<Builtin> {
        Builtin::ALL
            .iter()
            .find(|(_, n, _)| *n == name)
            .map(|(b, _, _)| *b)
    }
    pub fn name(&self) -> &'static str {
        match Builtin::ALL.iter().find(|(b, _, _)| b == self) {
            Some((_, n, _)) => n,
            None => unreachable!("every Builtin variant has a registry row"),
        }
    }
    pub fn arity(&self) -> u32 {
        match Builtin::ALL.iter().find(|(b, _, _)| b == self) {
            Some((_, _, a)) => *a,
            None => unreachable!("every Builtin variant has a registry row"),
        }
    }
}

impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#<builtin:{}>", self.name())
    }
}

#[cfg(test)]
mod tests {
    // more ...
}
