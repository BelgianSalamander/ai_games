use gamedef::game_interface::{GameInterface, Type, BuiltinType, get_enum_variant_type, StructField, EnumVariant};

fn make_deserializer(ty: &Type) -> String {
    match ty {
        Type::Builtin(BuiltinType::Bool) => "((await data.read_u8()) != 0)".to_string(),
        Type::Builtin(BuiltinType::U8) => "(await data.read_u8())".to_string(),
        Type::Builtin(BuiltinType::U16) => "(await data.read_u16()".to_string(),
        Type::Builtin(BuiltinType::U32) => "(await data.read_u32())".to_string(),
        Type::Builtin(BuiltinType::U64) => "(await data.read_u64())".to_string(),
        Type::Builtin(BuiltinType::I8) => "(await data.read_i8())".to_string(),
        Type::Builtin(BuiltinType::I16) => "(await data.read_i16())".to_string(),
        Type::Builtin(BuiltinType::I32) => "(await data.read_i32())".to_string(),
        Type::Builtin(BuiltinType::I64) => "(await data.read_i64())".to_string(),
        Type::Builtin(BuiltinType::F32) => "(await data.read_f32())".to_string(),
        Type::Builtin(BuiltinType::F64) => "(await data.read_f64())".to_string(),
        Type::Builtin(BuiltinType::Str) => "(await data.read_str())".to_string(),

        Type::Array(ty, size) => {
            format!(
                "await Promise.all(Array({}).fill(0).map(async () => {}))",
                size,
                make_deserializer(ty)
            )
        },

        Type::DynamicArray(ty) => {
            format!(
                "await Promise.all(Array(await data.read_u32()).fill(0).map(async () => {}))",
                make_deserializer(ty)
            )
        },

        Type::NamedType(name) => format!("(await deserialize_{}(data))", name),

        _ => panic!("{:?} must be deserialized through named type", ty)
    }
}

pub fn make_js_deserializers(itf: &GameInterface) -> String {
    let itf = itf.reduced();
    let mut deserializers = String::new();

    for (name, ty) in &itf.types {
        deserializers.push_str(&format!(
            "async function deserialize_{}(data) {{\n",
            name
        ));

        match ty {
            Type::Builtin(_) | Type::Array(_, _) | Type::DynamicArray(_) | Type::NamedType(_) => {
                deserializers.push_str(&format!("  return {};\n", make_deserializer(ty)));
            },

            Type::Struct(fields) => {
                deserializers.push_str("  return {\n");

                for StructField {name, ty} in fields.iter() {
                    deserializers.push_str(&format!(
                        "    {}: {},\n",
                        name,
                        make_deserializer(ty)
                    ));
                }

                deserializers.push_str("  };\n");
            },
            
            Type::Enum(variants) => {
                deserializers.push_str(&format!("  switch ({}) {{\n", make_deserializer(&Type::Builtin(get_enum_variant_type(variants)))));

                for (i, EnumVariant{name, types}) in variants.iter().enumerate() {
                    deserializers.push_str(&format!(
                        "    case {}:\n      return {{\n        variant: \"{}\",\n        data: {{\n",
                        i, name
                    ));

                    for StructField {name, ty} in types.iter() {
                        deserializers.push_str(&format!(
                            "          {}: {},\n",
                            name,
                            make_deserializer(ty)
                        ));
                    }

                    deserializers.push_str("        }\n      };\n");
                }

                deserializers.push_str("    default: throw new Error(\"Invalid enum variant\");\n");

                deserializers.push_str("  }\n");
            }
        }

        deserializers.push_str("}\n\n");
    }

    deserializers   
}