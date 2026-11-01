//! @path: karst/crates/karst_vm/value.rs
//! @author: redskaber
//! @datetime:2026-11-01
//! @discription: karst::crates::karst_vm::value
//!
//! bytecode virtual machine - runtime value and object beap

use std::fmt;

use crate::Builtin;

use karst_core::LiteralValue;

pub type HeapRef = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureData {
    pub proto: u32,
    pub env: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeapObj {
    Pair(Value, Value),
    Closure(ClosureData),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Heap {
    cells: Vec<HeapObj>,
}

impl Heap {
    pub fn get(&self, r: HeapRef) -> Option<&HeapObj> {
        self.cells.get(r as usize)
    }
    pub fn alloc(&mut self, obj: HeapObj) -> HeapRef {
        let r = self.cells.len() as HeapRef;
        self.cells.push(obj);
        r
    }
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Nil,
    Pair(HeapRef),
    Closure(HeapRef),
    Builtin(Builtin),
}

impl Value {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Nil => "nil",
            Value::Pair(_) => "pair",
            Value::Closure(_) => "closure",
            Value::Builtin(_) => "builtin",
        }
    }

    pub fn as_pair<'h>(&self, heap: &'h Heap) -> Option<(&'h Value, &'h Value)> {
        match self {
            Value::Pair(r) => match heap.get(*r) {
                Some(HeapObj::Pair(a, b)) => Some((a, b)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_closure<'h>(&self, heap: &'h Heap) -> Option<&'h ClosureData> {
        match self {
            Value::Closure(r) => match heap.get(*r) {
                Some(HeapObj::Closure(data)) => Some(data),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn eq_identity(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Pair(a), Value::Pair(b)) => a == b,
            (Value::Closure(a), Value::Closure(b)) => a == b,
            (Value::Builtin(a), Value::Builtin(b)) => a == b,
            _ => false,
        }
    }

    pub fn from_litral(lit: &LiteralValue, heap: &mut Heap) -> Value {
        match lit {
            LiteralValue::Int(v) => Value::Int(*v),
            LiteralValue::Float(v) => Value::Float(*v),
            LiteralValue::Bool(v) => Value::Bool(*v),
            LiteralValue::String(v) => Value::Str(v.clone()),
            LiteralValue::Nil => Value::Nil,
            LiteralValue::Pair(a, b) => {
                let av = Value::from_litral(a, heap);
                let bv = Value::from_litral(b, heap);
                Value::Pair(heap.alloc(HeapObj::Pair(av, bv)))
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Str(v) => write!(f, "\"{v}\""),
            Value::Nil => write!(f, "nil"),
            Value::Pair(_) => write!(f, "#<pair>"),
            Value::Closure(_) => write!(f, "#<closure>"),
            Value::Builtin(b) => write!(f, "#<builtin: {}>", b.name()),
        }
    }
}
