//! @path: karst/crates/karst_syntax/symbol.rs
//! @author: redskaber
//! @datetime: 2026-09-26
//! @discription: karst::crates::karst_syntax::symbol

use std::{collections::HashMap, rc::Rc};

/// interning symbol (table index)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(pub u32);

impl Symbol {
    pub fn as_str(self, table: &SymbolTable) -> &str {
        table.name(self)
    }
}

/// core origin Keyword for S-expr
/// An S-expression is simply a scaffolding for a transformation,
/// where the importance of the keywords is irrelevant;
/// the key is that the S-expression's front-end transformation
/// must ultimately result in a front-end transformation
/// that conforms to the target language's grammar.
/// S-Express the concept in the most concise way possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    /// core
    Lambda,
    If,
    SetBang,
    Quote,
    DefineSyntax,
    SyntaxRules,

    /// Derivable
    Define,
    Begin,
    Module,
    Import,
    Export,
    Let,
    LetRec,
    LetStar,
    Cond,
    Else,
    And,
    Or,
    While,
}

impl Keyword {
    pub fn as_str(self) -> &'static str {
        match self {
            Keyword::Lambda => "lambda",
            Keyword::If => "if",
            Keyword::SetBang => "set!",
            Keyword::Quote => "quote",
            Keyword::DefineSyntax => "define_syntax",
            Keyword::SyntaxRules => "syntax_rules",
            Keyword::Define => "define",
            Keyword::Begin => "begin",
            Keyword::Module => "module",
            Keyword::Import => "import",
            Keyword::Export => "export",
            Keyword::Let => "let",
            Keyword::LetRec => "letrec",
            Keyword::LetStar => "let*",
            Keyword::Cond => "cond",
            Keyword::Else => "else",
            Keyword::And => "and",
            Keyword::Or => "or",
            Keyword::While => "while",
        }
    }

    pub fn from_name(name: &str) -> Option<Keyword> {
        Some(match name {
            "lambda" => Keyword::Lambda,
            "if" => Keyword::If,
            "set!" => Keyword::SetBang,
            "quote" => Keyword::Quote,
            "define_syntax" => Keyword::DefineSyntax,
            "syntax_rules" => Keyword::SyntaxRules,
            "define" => Keyword::Define,
            "begin" => Keyword::Begin,
            "module" => Keyword::Module,
            "import" => Keyword::Import,
            "export" => Keyword::Export,
            "let" => Keyword::Let,
            "letrec" => Keyword::LetRec,
            "let*" => Keyword::LetStar,
            "cond" => Keyword::Cond,
            "else" => Keyword::Else,
            "and" => Keyword::And,
            "or" => Keyword::Or,
            "while" => Keyword::While,
            _ => return None,
        })
    }
}

/// symbol table (unique)
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// store map keys
    names: Vec<Rc<str>>,
    /// store str to symbol map
    map: HashMap<Rc<str>, Symbol>,
    /// store keywords symbol
    keywords: HashMap<Keyword, Symbol>,
}

impl SymbolTable {
    /// create SymbolTable and register keywords to SymbolTable
    pub fn new() -> Self {
        let mut table = SymbolTable {
            names: Vec::new(),
            map: HashMap::new(),
            keywords: HashMap::new(),
        };

        for kw in [
            Keyword::Lambda,
            Keyword::If,
            Keyword::SetBang,
            Keyword::Quote,
            Keyword::DefineSyntax,
            Keyword::SyntaxRules,
            Keyword::Define,
            Keyword::Begin,
            Keyword::Module,
            Keyword::Import,
            Keyword::Export,
            Keyword::Let,
            Keyword::LetRec,
            Keyword::LetStar,
            Keyword::Cond,
            Keyword::Else,
            Keyword::And,
            Keyword::Or,
            Keyword::While,
        ] {
            let sym = table.intern(kw.as_str());
            table.keywords.insert(kw, sym);
        }

        table
    }

    /// interning symbol
    pub fn intern(&mut self, name: &str) -> Symbol {
        // 1. normalized
        let normalized = if is_nfc(name) {
            Rc::from(name)
        } else {
            Rc::from(normalized_nfc(name))
        };

        // 2. query
        if let Some(&sym) = self.map.get(&normalized) {
            return sym;
        }

        // 3. build and store
        let sym = Symbol(self.names.len() as u32);
        self.names.push(normalized.clone());
        self.map.insert(normalized, sym);
        sym
    }

    pub fn keyword_symbol(&self, kw: Keyword) -> Symbol {
        self.keywords[&kw]
    }

    pub fn is_keyword(&self, sym: Symbol, kw: Keyword) -> bool {
        self.keywords.get(&kw) == Some(&sym)
    }

    /// from symbol to str
    pub fn name(&self, sym: Symbol) -> &str {
        &self.names[sym.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// TODO: unicode - handle
/// normalized chars if nfc (stage 0)
fn is_nfc(s: &str) -> bool {
    !s.chars().any(is_combining_mark)
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}' | '\u{1AB0}'..='\u{1AFF}' | '\u{20D0}'..='\u{20FF}')
}

/// min nfc normalized
fn normalized_nfc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if is_combining_mark(c)
            && let Some(base) = out.chars().last()
            && let Some(pre) = compose_mark(base, c)
        {
            out.pop();
            out.push(pre);
            continue;
        }
        out.push(c);
    }
    out
}

/// Base character + accent combination → Pre-composed character (covers common Latin-1 characters; returns `None` if no mapping is found).
fn compose_mark(base: char, mark: char) -> Option<char> {
    match (base, mark) {
        ('A', '\u{0300}') => Some('À'),
        ('A', '\u{0301}') => Some('Á'),
        ('A', '\u{0302}') => Some('Â'),
        ('E', '\u{0301}') => Some('É'),
        ('e', '\u{0301}') => Some('é'),
        ('a', '\u{0300}') => Some('à'),
        ('a', '\u{0301}') => Some('á'),
        ('u', '\u{0308}') => Some('ü'),
        ('u', '\u{0301}') => Some('ú'),
        ('n', '\u{0303}') => Some('ñ'),
        ('N', '\u{0303}') => Some('Ñ'),
        ('c', '\u{0327}') => Some('ç'),
        ('C', '\u{0327}') => Some('Ç'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_is_idempotent() {
        let mut t = SymbolTable::new();
        let a = t.intern("fib");
        let b = t.intern("fib");
        assert_eq!(a, b);
        let c = t.intern("other");
        assert_ne!(a, c);
        assert_eq!(t.name(a), "fib");
    }

    #[test]
    fn keywords_prefetched() {
        let mut t = SymbolTable::new();
        let sym = t.intern("lambda");
        assert_eq!(t.keyword_symbol(Keyword::Lambda), sym);
        assert!(t.is_keyword(sym, Keyword::Lambda));
        assert!(!t.is_keyword(sym, Keyword::If));
    }

    #[test]
    fn nfc_normalization_one_time() {
        let mut t = SymbolTable::new();
        // "e" + U+0301 (combining acute) → "é"
        let a = t.intern("e\u{0301}");
        let b = t.intern("\u{00E9}");
        assert_eq!(
            a, b,
            "The combined and pre-grouped forms must be represented by the same symbol."
        );
        assert_eq!(t.name(a), "\u{00E9}");
    }

    #[test]
    fn keyword_roundtrip() {
        for kw in [
            Keyword::Lambda,
            Keyword::SetBang,
            Keyword::DefineSyntax,
            Keyword::SyntaxRules,
        ] {
            assert_eq!(Keyword::from_name(kw.as_str()), Some(kw));
        }
        assert_eq!(Keyword::from_name("not-a-keyword"), None);
    }
}
