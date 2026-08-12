use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Function(u16),
}

#[derive(Default)]
pub struct ConstantPool {
    constants: Vec<Value>,
    intTable: HashMap<i64, u16>,
    floatTable: HashMap<u64, u16>,
    stringTable: HashMap<String, u16>,
}

impl ConstantPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, value: Value) -> u16 {
        match &value {
            Value::Int(i) => {
                if let Some(&idx) = self.intTable.get(i) {
                    return idx;
                }
                self.intTable.insert(*i, self.constants.len() as u16);
            }
            Value::Float(f) => {
                let bits = f.to_bits();
                if let Some(&idx) = self.floatTable.get(&bits) {
                    return idx;
                }
                self.floatTable.insert(bits, self.constants.len() as u16);
            }
            Value::String(s) => {
                if let Some(&idx) = self.stringTable.get(s) {
                    return idx;
                }
                self.stringTable
                    .insert(s.clone(), self.constants.len() as u16);
            }
            _ => {}
        }
        let idx = self.constants.len() as u16;
        self.constants.push(value);
        idx
    }

    pub fn get(&self, index: u16) -> Option<&Value> {
        self.constants.get(index as usize)
    }

    pub fn len(&self) -> usize {
        self.constants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }

    pub fn get_constants(&self) -> &[Value] {
        &self.constants
    }
}
