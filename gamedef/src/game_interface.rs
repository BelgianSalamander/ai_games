use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BuiltinType {
    U8, U16, U32, U64,

    I8, I16, I32, I64,

    F32, F64,

    Bool, Str
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: Type
}

pub type StructFields = Vec<StructField>;

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub types: StructFields
}

pub type EnumVariants = Vec<EnumVariant>;

pub fn get_enum_variant_type(variants: &EnumVariants) -> BuiltinType {
    let num_variants = variants.len();

    let log = ((num_variants as f64).log2() / 8.0) as i8;

    match log {
        0 => BuiltinType::U8,
        1 => BuiltinType::U16,
        2 => BuiltinType::U32,
        3 => BuiltinType::U64,
        _ => panic!("Too many enum variants")
    }
}

pub fn is_basic_enum(variants: &EnumVariants) -> bool {
    for variant in variants {
        if variant.types.len() != 0 {
            return false;
        }
    }

    true
}

#[derive(Debug, Clone)]
pub enum Type {
    Builtin(BuiltinType),
    Struct(Box<StructFields>),
    Array(Box<Type>, usize),
    DynamicArray(Box<Type>),
    Enum(Box<EnumVariants>),
    NamedType(String)
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub args: Vec<(String, Type)>,
    pub ret: Option<Type>
}

#[derive(Debug, Clone)]
pub struct GameInterface {
    pub name: String,
    pub types: Vec<(String, Type)>,
    pub functions: Vec<(String, FunctionSignature)>
}

pub fn try_reduce_struct_fields(fields: &StructFields, lookup: &HashMap<String, Type>) -> Option<StructFields> {
    let mut res = Vec::new();

    for field in fields {
        if let Some(ty) = try_reduce_type(&field.ty, lookup) {
            res.push(StructField {
                name: field.name.clone(),
                ty
            });
        } else {
            return None;
        }
    }

    Some(res)
}

pub fn try_reduce_type(ty: &Type, lookup: &HashMap<String, Type>) -> Option<Type> {
    match ty {
        Type::Builtin(_) => Some(ty.clone()),
        Type::Struct(fields) => {
            if let Some(fields) = try_reduce_struct_fields(fields, lookup) {
                Some(Type::Struct(Box::new(fields)))
            } else {
                None
            }
        },
        Type::Array(ty, len) => {
            if let Some(ty) = try_reduce_type(ty, lookup) {
                Some(Type::Array(Box::new(ty), *len))
            } else {
                None
            }
        },
        Type::DynamicArray(ty) => {
            if let Some(ty) = try_reduce_type(ty, lookup) {
                Some(Type::DynamicArray(Box::new(ty)))
            } else {
                None
            }
        },
        Type::Enum(variants) => {
            let mut res = Vec::new();

            for variant in variants.iter() {
                if let Some(fields) = try_reduce_struct_fields(&variant.types, lookup) {
                    res.push(EnumVariant {
                        name: variant.name.clone(),
                        types: fields
                    });
                } else {
                    return None;
                }
            }

            Some(Type::Enum(Box::new(res)))
        },
        Type::NamedType(name) => {
            match lookup.get(name) {
                Some(&Type::Array(ref ty, size)) => {
                    if let Some(ty) = try_reduce_type(&ty, lookup) {
                        Some(Type::Array(Box::new(ty), size))
                    } else {
                        None
                    }
                },
                Some(&Type::DynamicArray(ref ty)) => {
                    if let Some(ty) = try_reduce_type(&ty, lookup) {
                        Some(Type::DynamicArray(Box::new(ty)))
                    } else {
                        None
                    }
                },
                Some(&Type::Builtin(builtin)) => Some(Type::Builtin(builtin)),
                Some(_) => Some(Type::NamedType(name.clone())),
                None => None
            }
        }
    }
}

impl GameInterface {
    pub fn reduced(&self) -> Self {
        let mut types = Vec::new();
        let mut functions = Vec::new();

        let mut type_lookup: HashMap<String, Type> = HashMap::new();

        let mut remaining = self.types.clone();

        while remaining.len() > 0 {
            let mut i = 0;

            while i < remaining.len() {
                let (name, ty) = &remaining[i];

                if let Some(ty) = try_reduce_type(ty, &type_lookup) {
                    type_lookup.insert(name.clone(), ty.clone());
                    types.push((name.clone(), ty));
                    remaining.remove(i);
                }

                i += 1;
            }
        }

        for (name, func) in &self.functions {
            let mut args = Vec::new();

            for (name, ty) in &func.args {
                if let Some(ty) = try_reduce_type(ty, &type_lookup) {
                    args.push((name.clone(), ty));
                } else {
                    panic!("Failed to reduce type of argument {} in function {}", name, name);
                }
            }

            let ret = if let Some(ty) = &func.ret {
                if let Some(ty) = try_reduce_type(ty, &type_lookup) {
                    Some(ty)
                } else {
                    panic!("Failed to reduce type of return value in function {}", name);
                }
            } else {
                None
            };

            functions.push((name.clone(), FunctionSignature {
                args,
                ret
            }));
        }

        GameInterface {
            name: self.name.clone(),
            types,
            functions
        }
    }
}