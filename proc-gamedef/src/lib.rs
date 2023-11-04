#![feature(proc_macro_span, extend_one)]

extern crate proc_macro;
use std::path::Path;

use gamedef::{parser::parse_game_interface, game_interface::{GameInterface, Type, BuiltinType, StructFields, StructField, get_enum_variant_type}};
use proc_macro::{TokenStream, Span, TokenTree, Ident, Group, Punct, Literal};

fn make_type(ty: &Type, span: &Span, out: &mut TokenStream) {
    match ty {
        Type::Builtin(builtin) => {
            let name = match builtin {
                BuiltinType::U8 => "u8",
                BuiltinType::U16 => "u16",
                BuiltinType::U32 => "u32",
                BuiltinType::U64 => "u64",

                BuiltinType::I8 => "i8",
                BuiltinType::I16 => "i16",
                BuiltinType::I32 => "i32",
                BuiltinType::I64 => "i64",

                BuiltinType::F32 => "f32",
                BuiltinType::F64 => "f64",

                BuiltinType::Bool => "bool",

                BuiltinType::Str => "String",
            };


            out.extend(name.parse::<TokenStream>().unwrap());
        },
        Type::NamedType(ref name) => {
            out.extend(name.parse::<TokenStream>().unwrap());
        },
        Type::Array(ty, size) => {
            let mut stream = TokenStream::new();
            make_type(&ty, span, &mut stream);

            stream.extend_one(TokenTree::Punct(Punct::new(';', proc_macro::Spacing::Alone)));
            stream.extend_one(TokenTree::Literal(Literal::usize_suffixed(*size)));

            let group = Group::new(proc_macro::Delimiter::Bracket, stream);

            out.extend_one(TokenTree::Group(group));
        },
        Type::DynamicArray(ty) => {
            out.extend("Vec<".parse::<TokenStream>().unwrap());
            make_type(&ty, span, out);
            out.extend(">".parse::<TokenStream>().unwrap());
        },
        t => {
            panic!("Unsupported type: {:?}", t);
        }
    }
}

fn make_type_str(ty: &Type) -> String {
    match ty {
        Type::Builtin(builtin) => {
            let name = match builtin {
                BuiltinType::U8 => "u8",
                BuiltinType::U16 => "u16",
                BuiltinType::U32 => "u32",
                BuiltinType::U64 => "u64",

                BuiltinType::I8 => "i8",
                BuiltinType::I16 => "i16",
                BuiltinType::I32 => "i32",
                BuiltinType::I64 => "i64",

                BuiltinType::F32 => "f32",
                BuiltinType::F64 => "f64",

                BuiltinType::Bool => "bool",

                BuiltinType::Str => "String",
            };

            name.to_string()
        },
        Type::NamedType(ref name) => {
            name.to_string()
        },
        Type::Array(ty, size) => {
            format!("[{}; {}]", make_type_str(&ty), size)
        },
        Type::DynamicArray(ty) => {
            format!("Vec<{}>", make_type_str(&ty))
        },
        t => {
            panic!("Unsupported type: {:?}", t);
        }
    }
}

fn make_struct_fields(fields: &StructFields, span: &Span) -> TokenTree {
    let mut stream = TokenStream::new();

    for StructField {name, ty} in fields {
        stream.extend_one(TokenTree::Ident(Ident::new(name, *span)));
        stream.extend_one(TokenTree::Punct(Punct::new(':', proc_macro::Spacing::Alone)));
        make_type(ty, span, &mut stream);
        stream.extend_one(TokenTree::Punct(Punct::new(',', proc_macro::Spacing::Alone)));
    }

    let group = Group::new(proc_macro::Delimiter::Brace, stream);

    TokenTree::Group(group)
}

fn make_serializer(ty: &Type, val: &str, out: &str) -> String {
    match ty {
        Type::Builtin(BuiltinType::Str) => format!("{{let bytes = {val}.as_bytes();\n{out}.extend(&bytes.len().to_le_bytes());\n{out}.extend(bytes);}}", val=val, out=out),
        Type::Builtin(_) => format!("{out}.extend({val}.to_le_bytes());", val=val, out=out),
        Type::NamedType(name) => format!("serialize_{name}(&{val}, {out});", val=val, out=out, name=name),
        Type::Array(ty, _) => {
            let mut res = String::new();

            res.push_str(&format!("for x in ({}).iter() {{\n", val));
            res.push_str(&make_serializer(ty, "x", out));
            res.push_str("}\n");

            res
        },
        Type::DynamicArray(ty) => {
            let mut res = String::new();

            res.push_str(&format!("{out}.extend(&{val}.len().to_le_bytes());", val=val, out=out));

            res.push_str(&format!("for x in ({}).iter() {{\n", val));
            res.push_str(&make_serializer(ty, "x", out));
            res.push_str("}\n");

            res
        },
        _ => panic!("Cannot make inline serializer{:?}", ty)
    }
}

fn make_deserializer(ty: &Type, instance: &str) -> String {
    match ty {
        Type::Builtin(x) => {
            let name = match x {
                BuiltinType::U8 => "u8",
                BuiltinType::U16 => "u16",
                BuiltinType::U32 => "u32",
                BuiltinType::U64 => "u64",

                BuiltinType::I8 => "i8",
                BuiltinType::I16 => "i16",
                BuiltinType::I32 => "i32",
                BuiltinType::I64 => "i64",

                BuiltinType::F32 => "f32",
                BuiltinType::F64 => "f64",

                BuiltinType::Bool => "bool",

                BuiltinType::Str => "str",
            };

            format!("{}.read_{}().await?", instance, name)
        },
        Type::NamedType(name) => format!("deserialize_{}({}).await?", name, instance),
        Type::Array(ty, count) => {
            let mut res = String::new();
            res.push_str("unsafe {");

            res.push_str(&format!("let mut res: [{}; {}] = std::mem::uninitialized();\n", make_type_str(ty), count));
            
            res.push_str(&format!("for i in 0..{} {{\n", count));
            res.push_str(&format!("res[i] = {};\n", make_deserializer(ty, instance)));
            res.push_str("}\n");

            res.push_str("res");

            res.push_str("}");

            res
        },
        Type::DynamicArray(ty) => {
            let mut res = String::new();

            res.push_str("{");

            res.push_str(&format!("let array_size = {}.read_u32().await?;\n", instance));
            res.push_str(&format!("let mut res: Vec<{}> = Vec::with_capacity(array_size as usize);\n", make_type_str(ty)));

            res.push_str("for i in 0..array_size {\n");
            res.push_str(&format!("res.push({});\n", make_deserializer(ty, instance)));
            res.push_str("}\n");

            res.push_str("res");

            res.push_str("}");

            res
        },
        _ => panic!("Cannot make inline deserializer{:?}", ty)
    }
}

fn are_struct_fields_copyable(fields: &StructFields, itf: &GameInterface) -> bool {
    fields.iter().all(|StructField {ty, ..}| is_copyable(ty, itf))
}

fn is_copyable(ty: &Type, itf: &GameInterface) -> bool {
    match ty {
        Type::Builtin(_) => true,
        Type::NamedType(name) => {
            let (_, ty) = itf.types.iter().filter(|x| x.0 == *name).next().unwrap();

            is_copyable(ty, itf)
        },
        Type::Array(ty, _) => is_copyable(ty, itf),
        Type::DynamicArray(_) => false,
        Type::Struct(fields) => are_struct_fields_copyable(fields, itf),
        Type::Enum(variants) => variants.iter().all(|variant| are_struct_fields_copyable(&variant.types, itf)),
    }
}

fn are_struct_fields_equatable(fields: &StructFields, itf: &GameInterface) -> bool {
    fields.iter().all(|StructField {ty, ..}| is_equatable(ty, itf))
}

fn is_equatable(ty: &Type, itf: &GameInterface) -> bool {
    match ty {
        Type::Builtin(BuiltinType::F32) => false,
        Type::Builtin(BuiltinType::F64) => false,

        Type::Builtin(_) => true,

        Type::NamedType(name) => {
            let (_, ty) = itf.types.iter().filter(|x| x.0 == *name).next().unwrap();

            is_equatable(ty, itf)
        },

        Type::Array(ty, _) => is_equatable(ty, itf),

        Type::DynamicArray(ty) => is_equatable(ty, itf),

        Type::Struct(fields) => are_struct_fields_equatable(fields, itf),

        Type::Enum(variants) => variants.iter().all(|variant| are_struct_fields_equatable(&variant.types, itf)),
    }
}

fn read_struct_fields(fields: &StructFields, instance: &str) -> TokenTree {
    let mut stream = TokenStream::new();

    for StructField {name, ty} in fields {
        stream.extend_one(TokenTree::Ident(Ident::new(name, Span::call_site())));
        stream.extend_one(TokenTree::Punct(Punct::new(':', proc_macro::Spacing::Alone)));
        stream.extend(make_deserializer(ty, instance).parse::<TokenStream>().unwrap());
        stream.extend_one(TokenTree::Punct(Punct::new(',', proc_macro::Spacing::Alone)));
    }

    let group = Group::new(proc_macro::Delimiter::Brace, stream);

    TokenTree::Group(group)
}

fn get_derives(ty: &Type, itf: &GameInterface) -> Option<String> {
    let mut derives = Vec::new();

    match ty {
        Type::Builtin(_) | Type::NamedType(_) | Type::Array(_, _) | Type::DynamicArray(_) => return None,
        _ => {}
    };

    derives.push("Debug".to_string());
    derives.push("Clone".to_string());
    derives.push("PartialEq".to_string());
    derives.push("serde::Serialize".to_string());

    if is_copyable(ty, itf) {
        derives.push("Copy".to_string());
    }

    if is_equatable(ty, itf) {
        derives.push("Eq".to_string());
    }

    Some(format!("#[derive({})]", derives.join(", ")))
}

fn make_interface(itf: &GameInterface, span: &Span) -> TokenStream {
    let mut res = TokenStream::new();

    for (name, ty) in &itf.types {
        if let Some(derives) = get_derives(ty, itf) {
            res.extend(derives.parse::<TokenStream>().unwrap());
        }

        match ty {
            Type::Builtin(_) | Type::NamedType(_) | Type::Array(_,_) | Type::DynamicArray(_) => {
                res.extend(format!("pub type {} = ", name).parse::<TokenStream>().unwrap());

                make_type(&ty, span, &mut res);

                res.extend_one(TokenTree::Punct(Punct::new(';', proc_macro::Spacing::Alone)));
            },
            Type::Struct(fields) => {
                res.extend(format!("pub struct {} ", name).parse::<TokenStream>().unwrap());

                res.extend_one(make_struct_fields(fields, span));
            },
            Type::Enum(variants) => {
                res.extend(format!("pub enum {} ", name).parse::<TokenStream>().unwrap());

                let mut stream = TokenStream::new();

                for variant in variants.iter() {
                    stream.extend_one(TokenTree::Ident(Ident::new(&variant.name, *span)));

                    if variant.types.len() != 0 {
                        stream.extend_one(make_struct_fields(&variant.types, span))
                    }

                    stream.extend_one(TokenTree::Punct(Punct::new(',', proc_macro::Spacing::Alone)));
                }

                let group = Group::new(proc_macro::Delimiter::Brace, stream);

                res.extend_one(TokenTree::Group(group));
            }
        }
    }

    for (name, ty) in &itf.types {
        res.extend(format!("fn serialize_{}(value: &{}, out: &mut Vec<u8>)", name, name).parse::<TokenStream>().unwrap());

        let mut stream = TokenStream::new();

        match ty {
            Type::Struct(fields) => {
                let mut res = String::new();

                for StructField {name, ty} in fields.iter() {
                    res.push_str(&make_serializer(ty, &format!("value.{}", name), "out"));
                }

                stream.extend(res.parse::<TokenStream>().unwrap());
            },
            Type::Enum(variants) => {
                let mut res = String::new();

                let variant_type = get_enum_variant_type(variants);

                let width = match variant_type {
                    BuiltinType::U8 => 1,
                    BuiltinType::U16 => 2,
                    BuiltinType::U32 => 4,
                    BuiltinType::U64 => 8,
                    _ => panic!("Invalid enum variant type")
                };

                res.push_str(&format!("match value {{\n"));

                for (i, variant) in variants.iter().enumerate() {
                    res.push_str(&format!("{}::{}", name, variant.name));

                    if variant.types.len() != 0 {
                        res.push_str("{");
                        for StructField {name, ..} in variant.types.iter() {
                            res.push_str(name);
                            res.push(',');
                        }
                        res.push_str("}");
                    }

                    res.push_str(" => {\n");

                    res.push_str("out.extend(&[");
                    let bytes = i.to_le_bytes();

                    for i in 0..width {
                        res.push_str(&format!("0x{:02x},", bytes[i]));
                    }

                    res.push_str("]);\n");

                    for StructField {name, ty} in variant.types.iter() {
                        res.push_str(&make_serializer(ty, name, "out"));
                    }

                    res.push_str("}\n");
                }

                res.push_str("}\n");

                stream.extend(res.parse::<TokenStream>().unwrap());
            }
            x => stream.extend(make_serializer(x, "value", "out").parse::<TokenStream>().unwrap())
        }

        let group = Group::new(proc_macro::Delimiter::Brace, stream);

        res.extend_one(TokenTree::Group(group));
    }

    for (name, ty) in &itf.types {
        res.extend(format!("async fn deserialize_{}(instance: &mut crate::isolate::sandbox::RunningJob) -> Result<{}, async_std::io::Error>", name, name).parse::<TokenStream>().unwrap());

        let mut stream = TokenStream::new();

        match ty {
            Type::Struct(fields) => {
                stream.extend_one(TokenTree::Ident(Ident::new(name, *span)));
                stream.extend_one(read_struct_fields(fields, "instance"));
            },
            Type::Enum(variants) => {
                let variant_type = get_enum_variant_type(&variants);

                let reader = match variant_type {
                    BuiltinType::U8 => "instance.read_u8().await?",
                    BuiltinType::U16 => "instance.read_u16().await?",
                    BuiltinType::U32 => "instance.read_u32().await?",
                    BuiltinType::U64 => "instance.read_u64().await?",
                    _ => panic!("Invalid enum variant type")
                };

                stream.extend_one(TokenTree::Ident(Ident::new("match", *span)));
                stream.extend(reader.parse::<TokenStream>().unwrap());

                let mut stream2 = TokenStream::new();

                for (i, variant) in variants.iter().enumerate() {
                    stream2.extend_one(TokenTree::Literal(match variant_type {
                        BuiltinType::U8 => Literal::u8_suffixed(i as u8),
                        BuiltinType::U16 => Literal::u16_suffixed(i as u16),
                        BuiltinType::U32 => Literal::u32_suffixed(i as u32),
                        BuiltinType::U64 => Literal::u64_suffixed(i as u64),
                        _ => panic!("Invalid enum variant type")
                    }));

                    stream2.extend_one(TokenTree::Punct(Punct::new('=', proc_macro::Spacing::Joint)));
                    stream2.extend_one(TokenTree::Punct(Punct::new('>', proc_macro::Spacing::Alone)));

                    stream2.extend(format!("{}::{}", name, variant.name).parse::<TokenStream>().unwrap());
                    
                    if variant.types.len() != 0 {
                        stream2.extend_one(read_struct_fields(&variant.types, "instance"));
                    }

                    stream2.extend_one(TokenTree::Punct(Punct::new(',', proc_macro::Spacing::Alone)));
                }

                stream2.extend("_ => return Err(async_std::io::Error::new(async_std::io::ErrorKind::InvalidData, \"Invalid enum variant\"))".parse::<TokenStream>().unwrap());

                let group = Group::new(proc_macro::Delimiter::Brace, stream2);

                stream.extend_one(TokenTree::Group(group));
            },
            x => stream.extend_one(make_deserializer(x, "instance").parse::<TokenStream>().unwrap())
        }

        let mut stream2 = TokenStream::new();
        stream2.extend_one(TokenTree::Ident(Ident::new("Ok", *span)));
        let ok_group = Group::new(proc_macro::Delimiter::Parenthesis, stream);
        stream2.extend_one(TokenTree::Group(ok_group));

        let group = Group::new(proc_macro::Delimiter::Brace, stream2);

        res.extend_one(TokenTree::Group(group));
    }

    res.extend("
    struct Agent<'a> {
        instance: &'a mut  crate::isolate::sandbox::RunningJob
    }".parse::<TokenStream>().unwrap());

    res.extend("impl<'a> Agent<'a>".parse::<TokenStream>().unwrap());

    let mut stream = TokenStream::new();

    stream.extend("pub fn new(instance: &'a mut crate::isolate::sandbox::RunningJob) -> Self { Self {instance} }".parse::<TokenStream>().unwrap());

    for (i, (name, function)) in itf.functions.iter().enumerate() {
        stream.extend(format!("pub async fn {}", name).parse::<TokenStream>().unwrap());

        let mut args = TokenStream::new();
        args.extend("&mut self".parse::<TokenStream>().unwrap());

        for (name, ty) in &function.args {
            args.extend_one(TokenTree::Punct(Punct::new(',', proc_macro::Spacing::Alone)));
            args.extend_one(TokenTree::Ident(Ident::new(name, *span)));
            args.extend_one(TokenTree::Punct(Punct::new(':', proc_macro::Spacing::Alone)));

            //Needs reference?
            let needs_ref = match ty {
                Type::Builtin(BuiltinType::Str) => true,
                Type::Builtin(_) => false,
                _ => true
            };

            if needs_ref {
                args.extend_one(TokenTree::Punct(Punct::new('&', proc_macro::Spacing::Alone)));
            }
            
            make_type(ty, span, &mut args);
        }

        stream.extend_one(TokenTree::Group(Group::new(proc_macro::Delimiter::Parenthesis, args)));

        
        stream.extend("-> Result<".parse::<TokenStream>().unwrap());
        if let Some(ref ty) = function.ret {
            make_type(&ty, span, &mut stream);
        } else {
            stream.extend("()".parse::<TokenStream>().unwrap());
        }
        stream.extend(", async_std::io::Error>".parse::<TokenStream>().unwrap());
        
        let mut body = TokenStream::new();

        body.extend("let mut out_bytes: Vec<u8> = Vec::new();".parse::<TokenStream>().unwrap());
        body.extend(format!("out_bytes.extend(&[{}]);", i).parse::<TokenStream>().unwrap());

        for (name, ty) in &function.args {
            body.extend(make_serializer(ty, &name, "(&mut out_bytes)").parse::<TokenStream>().unwrap());
        }

        body.extend("self.instance.write(&out_bytes).await?;".parse::<TokenStream>().unwrap());

        if let Some(ref ty) = function.ret {
            let mut res = TokenStream::new();
            res.extend(make_deserializer(ty, "(&mut self.instance)").parse::<TokenStream>().unwrap());

            body.extend_one(TokenTree::Ident(Ident::new("Ok", *span)));
            body.extend_one(TokenTree::Group(Group::new(proc_macro::Delimiter::Parenthesis, res)));
        } else {
            body.extend("Ok(())".parse::<TokenStream>().unwrap());
        }

        stream.extend_one(TokenTree::Group(Group::new(proc_macro::Delimiter::Brace, body)));
    }
    

    stream.extend("pub async fn kill(mut self) { match self.instance.kill().await {
        Ok(_) => (),
        Err(e) => log::error!(\"Failed to kill sandbox: {:?}\", e)
    } }".parse::<TokenStream>().unwrap());

    stream.extend("pub fn set_error(&mut self, error: String) { self.instance.set_error(error) }".parse::<TokenStream>().unwrap());

    res.extend_one(TokenTree::Group(Group::new(proc_macro::Delimiter::Brace, stream)));

    println!("Generated interface: {}", res.to_string());

    res
}

#[proc_macro]
pub fn make_server(tokens: TokenStream) -> TokenStream {
    let name = tokens.to_string().replace("\"", "");
    let path = Path::new(&name);
    let span = Span::call_site();
    let call_file = span.source_file().path();
    let dir = call_file.parent().unwrap();
    let path= dir.join(path);

    println!("Loading game def {:?}", path);

    let name = name.replace(" ", "_").replace("/", "_").replace(".", "_");

    let content = std::fs::read_to_string(path).unwrap();
    let game_interface = parse_game_interface(&content, name).unwrap();

    make_interface(&game_interface, &span)
}